use std::sync::Arc;

use async_trait::async_trait;
use poria_core::pipeline::{transition_stage, PipelineEvent};
use poria_core::types::{Pipeline, PipelineStatus, Stage, StageEnum, StageStatus};

use super::SqlitePipelineStore;

/// Outcome of `PipelineRecovery::recover_all`.
#[derive(Debug, Default, Clone)]
pub struct RecoveryResult {
    pub running_recovered: Vec<String>,
    pub blocked_renotified: Vec<String>,
    pub errors: Vec<String>,
}

/// Cleans uncommitted worktree changes for an interrupted Dev/CR stage.
#[async_trait]
pub trait WorktreeCleaner: Send + Sync {
    async fn clean_dirty_state(&self, repo_name: &str, pipeline_id: &str) -> Result<(), String>;
}

/// Re-sends HITL notify for pipelines that were blocked across a restart.
#[async_trait]
pub trait HumanLoopRenotify: Send + Sync {
    async fn renotify(&self, pipeline: &Pipeline, stage: &Stage) -> Result<(), String>;
}

/// Restores interrupted RUNNING pipelines so the worker can resume them.
pub struct PipelineRecovery {
    store: Arc<SqlitePipelineStore>,
}

impl PipelineRecovery {
    pub fn new(store: Arc<SqlitePipelineStore>) -> Self {
        Self { store }
    }

    /// Mark leftover Running stages as Failed (retryable) and re-notify Blocked HITL.
    pub async fn recover_all(
        &self,
        cleaner: Option<&dyn WorktreeCleaner>,
        notifier: Option<&dyn HumanLoopRenotify>,
    ) -> Result<RecoveryResult, String> {
        let mut result = RecoveryResult::default();

        match self.store.find_by_status(PipelineStatus::Running) {
            Ok(running) => {
                for pipeline in running {
                    match self.recover_running(pipeline, cleaner).await {
                        Ok(id) => result.running_recovered.push(id),
                        Err(error) => result.errors.push(error),
                    }
                }
            }
            Err(error) => result.errors.push(error),
        }

        match self.store.find_by_status(PipelineStatus::Blocked) {
            Ok(blocked) => {
                for pipeline in blocked {
                    match self.renotify_blocked(&pipeline, notifier).await {
                        Ok(Some(id)) => result.blocked_renotified.push(id),
                        Ok(None) => {}
                        Err(error) => result.errors.push(error),
                    }
                }
            }
            Err(error) => result.errors.push(error),
        }

        Ok(result)
    }

    async fn recover_running(
        &self,
        mut pipeline: Pipeline,
        cleaner: Option<&dyn WorktreeCleaner>,
    ) -> Result<String, String> {
        let interrupted_idx = pipeline
            .stages
            .iter()
            .position(|stage| stage.status == StageStatus::Running);

        if let Some(idx) = interrupted_idx {
            let stage_name = pipeline.stages[idx].name;
            if matches!(stage_name, StageEnum::Dev | StageEnum::Cr) {
                if let Some(cleaner) = cleaner {
                    for repo in &pipeline.repos {
                        if let Err(error) =
                            cleaner.clean_dirty_state(&repo.name, &pipeline.id).await
                        {
                            tracing::warn!(
                                pipeline_id = %pipeline.id,
                                repo = %repo.name,
                                error = %error,
                                "recovery worktree clean failed"
                            );
                        }
                    }
                }
            }

            if let Err(error) =
                transition_stage(&mut pipeline.stages[idx].status, StageStatus::Failed)
            {
                return Err(format!("{}: {error}", pipeline.id));
            }
            pipeline.stages[idx].retry_count += 1;

            let mut events = vec![PipelineEvent::stage_failed(
                &pipeline.id,
                stage_name,
                "recovery: interrupted running stage",
                pipeline.stages[idx].retry_count,
            )];
            if matches!(stage_name, StageEnum::Dev | StageEnum::Cr) {
                for repo in &pipeline.repos {
                    events.push(PipelineEvent::worktree_cleaned(
                        &pipeline.id,
                        &repo.name,
                        "recovery: cleaned uncommitted changes",
                    ));
                }
            }

            self.store
                .save_stage_tx(Some(&pipeline.stages[idx]), &pipeline, &events)?;
        }

        Ok(pipeline.id)
    }

    async fn renotify_blocked(
        &self,
        pipeline: &Pipeline,
        notifier: Option<&dyn HumanLoopRenotify>,
    ) -> Result<Option<String>, String> {
        let Some(stage) = pipeline
            .stages
            .iter()
            .find(|stage| stage.status == StageStatus::Blocked && stage.issue.is_some())
        else {
            return Ok(None);
        };
        let Some(notifier) = notifier else {
            return Ok(Some(pipeline.id.clone()));
        };
        notifier.renotify(pipeline, stage).await?;
        Ok(Some(pipeline.id.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use poria_core::types::{PipelineConfig, RepoConfig, StageIssue, STAGE_ORDER};
    use rusqlite::Connection;
    use std::sync::Mutex;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "journal_mode", "WAL").unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS schema_version (version INTEGER NOT NULL, applied_at TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS pipelines (id TEXT PRIMARY KEY, demand_id INTEGER NOT NULL, demand_code TEXT NOT NULL, demand_name TEXT, status TEXT NOT NULL DEFAULT 'created', raw_link TEXT NOT NULL, operator TEXT NOT NULL, has_regressed INTEGER DEFAULT 0, config TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS stages (id INTEGER PRIMARY KEY AUTOINCREMENT, pipeline_id TEXT NOT NULL REFERENCES pipelines(id), name TEXT NOT NULL, status TEXT NOT NULL DEFAULT 'pending', skill_id TEXT, retry_count INTEGER DEFAULT 0, max_retries INTEGER DEFAULT 3, input TEXT, output TEXT, gate_results TEXT, issue TEXT, rollback TEXT, agent_session_id TEXT, started_at TEXT, completed_at TEXT);
            CREATE TABLE IF NOT EXISTS events (seq INTEGER PRIMARY KEY AUTOINCREMENT, pipeline_id TEXT NOT NULL REFERENCES pipelines(id), kind TEXT NOT NULL, payload TEXT NOT NULL, created_at TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS audit_log (id INTEGER PRIMARY KEY AUTOINCREMENT, pipeline_id TEXT NOT NULL, stage TEXT, action TEXT NOT NULL, operator TEXT NOT NULL, detail TEXT, created_at TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS queue (id INTEGER PRIMARY KEY AUTOINCREMENT, pipeline_id TEXT NOT NULL UNIQUE, priority INTEGER DEFAULT 0, enqueued_at TEXT NOT NULL);
        ").unwrap();
        conn
    }

    fn frontend_repo() -> RepoConfig {
        RepoConfig {
            name: "ls-entrance".into(),
            git_url: "git@coding.jd.com:ls/ls-entrance.git".into(),
            branch: "feature_REQ".into(),
            base_branch: "master".into(),
            gitlab_project_path: "ls/ls-entrance".into(),
            depends_on: None,
            build_cmd: None,
        }
    }

    fn stage(name: StageEnum, status: StageStatus) -> Stage {
        Stage {
            id: None,
            pipeline_id: "pl-1".into(),
            name,
            status,
            skill_id: None,
            retry_count: 0,
            max_retries: 3,
            input: None,
            output: None,
            gate_results: None,
            issue: None,
            rollback: None,
            agent_session_id: None,
            started_at: None,
            completed_at: None,
        }
    }

    fn pipeline_with_stages(status: PipelineStatus, stages: Vec<Stage>) -> Pipeline {
        let now = Utc::now();
        Pipeline {
            id: "pl-1".into(),
            demand_id: 1,
            demand_code: "REQ-1".into(),
            demand_name: Some("test".into()),
            status,
            raw_link: "https://xingyun.jd.com/demand/1".into(),
            operator: "tester".into(),
            has_regressed: false,
            config: PipelineConfig {
                repos: vec![frontend_repo()],
                ..PipelineConfig::default()
            },
            stages,
            repos: vec![frontend_repo()],
            created_at: now,
            updated_at: now,
        }
    }

    struct RecordingCleaner {
        calls: Mutex<Vec<(String, String)>>,
    }

    impl RecordingCleaner {
        fn new() -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl WorktreeCleaner for RecordingCleaner {
        async fn clean_dirty_state(
            &self,
            repo_name: &str,
            pipeline_id: &str,
        ) -> Result<(), String> {
            self.calls
                .lock()
                .unwrap()
                .push((repo_name.to_string(), pipeline_id.to_string()));
            Ok(())
        }
    }

    struct RecordingNotifier {
        calls: Mutex<Vec<String>>,
    }

    impl RecordingNotifier {
        fn new() -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl HumanLoopRenotify for RecordingNotifier {
        async fn renotify(&self, pipeline: &Pipeline, _stage: &Stage) -> Result<(), String> {
            self.calls.lock().unwrap().push(pipeline.id.clone());
            Ok(())
        }
    }

    fn ordered_stages(overrides: &[(StageEnum, StageStatus)]) -> Vec<Stage> {
        STAGE_ORDER
            .iter()
            .map(|name| {
                let status = overrides
                    .iter()
                    .find(|(n, _)| n == name)
                    .map(|(_, s)| *s)
                    .unwrap_or(StageStatus::Pending);
                stage(*name, status)
            })
            .collect()
    }

    #[tokio::test]
    async fn recover_all_marks_interrupted_dev_failed_and_retries() {
        let store = Arc::new(SqlitePipelineStore::new(test_conn()));
        let stages = ordered_stages(&[
            (StageEnum::Init, StageStatus::Completed),
            (StageEnum::ReviewPrd, StageStatus::Completed),
            (StageEnum::Design, StageStatus::Completed),
            (StageEnum::Dev, StageStatus::Running),
        ]);
        store
            .create(&pipeline_with_stages(PipelineStatus::Running, stages))
            .unwrap();

        let cleaner = RecordingCleaner::new();
        let recovery = PipelineRecovery::new(store.clone());
        let result = recovery.recover_all(Some(&cleaner), None).await.unwrap();

        assert_eq!(result.running_recovered, vec!["pl-1".to_string()]);
        assert_eq!(
            cleaner.calls.lock().unwrap().as_slice(),
            &[("ls-entrance".to_string(), "pl-1".to_string())]
        );

        let loaded = store.load("pl-1").unwrap().unwrap();
        assert_eq!(loaded.status, PipelineStatus::Running);
        let dev = loaded
            .stages
            .iter()
            .find(|s| s.name == StageEnum::Dev)
            .unwrap();
        assert_eq!(dev.status, StageStatus::Failed);
        assert_eq!(dev.retry_count, 1);
    }

    #[tokio::test]
    async fn recover_all_skips_worktree_clean_for_init() {
        let store = Arc::new(SqlitePipelineStore::new(test_conn()));
        let stages = ordered_stages(&[(StageEnum::Init, StageStatus::Running)]);
        store
            .create(&pipeline_with_stages(PipelineStatus::Running, stages))
            .unwrap();

        let cleaner = RecordingCleaner::new();
        let recovery = PipelineRecovery::new(store.clone());
        recovery.recover_all(Some(&cleaner), None).await.unwrap();

        assert!(cleaner.calls.lock().unwrap().is_empty());
        let init = store
            .load("pl-1")
            .unwrap()
            .unwrap()
            .stages
            .into_iter()
            .find(|s| s.name == StageEnum::Init)
            .unwrap();
        assert_eq!(init.status, StageStatus::Failed);
        assert_eq!(init.retry_count, 1);
    }

    #[tokio::test]
    async fn recover_all_renotifies_blocked_with_issue() {
        let store = Arc::new(SqlitePipelineStore::new(test_conn()));
        let mut stages = ordered_stages(&[
            (StageEnum::Init, StageStatus::Completed),
            (StageEnum::ReviewPrd, StageStatus::Completed),
            (StageEnum::Design, StageStatus::Blocked),
        ]);
        stages[2].issue = Some(StageIssue {
            class: "requirement_ambiguous".into(),
            message: "P0 unanswered".into(),
            retryable: true,
        });
        store
            .create(&pipeline_with_stages(PipelineStatus::Blocked, stages))
            .unwrap();

        let notifier = RecordingNotifier::new();
        let recovery = PipelineRecovery::new(store);
        let result = recovery.recover_all(None, Some(&notifier)).await.unwrap();

        assert_eq!(result.blocked_renotified, vec!["pl-1".to_string()]);
        assert_eq!(
            notifier.calls.lock().unwrap().as_slice(),
            &["pl-1".to_string()]
        );
    }
}
