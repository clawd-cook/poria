use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use poria_commands::{
    CodingChannel, FileLock, HumanLoop, PipelineRun, PipelineStore, PipelineWorker, Queue,
    Recovery, WorkerDeps,
};
use poria_core::pipeline::PipelineEvent;
use poria_core::types::{Pipeline, PipelineStatus, RepoConfig, Stage};
use poria_infrastructure::auth::get_workspaces_root;
use poria_infrastructure::store::{
    HumanLoopRenotify, PipelineQueue, PipelineRecovery, SqlitePipelineStore, WorkerLock,
    WorktreeCleaner,
};
use poria_resources::WorktreeResource;
use poria_skills::{HumanLoop as SkillsHumanLoop, HumanLoopCoordinator};
use tauri::{AppHandle, Manager};

use crate::commands::pipeline::{run_auto_loop, AutoRunRuntime};
use crate::AppState;

pub(crate) fn spawn_pipeline_worker(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        if let Err(error) = run_pipeline_worker(app).await {
            tracing::warn!(error = %error, "pipeline worker exited");
        }
    });
}

async fn run_pipeline_worker(
    app: AppHandle,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let Some(state) = app.try_state::<AppState>() else {
        return Err("AppState missing".into());
    };
    let worker = PipelineWorker::new(
        WorkerDeps {
            store: DesktopStore(state.store.clone()),
            queue: Box::new(DesktopQueue(state.pipeline_queue.clone())),
            runner: Arc::new(DesktopRunner {
                app: app.clone(),
                current_run: state.current_run.clone(),
                runtime: AutoRunRuntime::from_state(&state),
            }),
            recovery: Box::new(DesktopRecovery {
                store: state.store.clone(),
                queue: state.pipeline_queue.clone(),
            }),
            coding_channel: None::<Box<dyn CodingChannel>>,
            human_loop: None::<Box<dyn HumanLoop>>,
            max_parallel: Arc::new(|| {
                poria_infrastructure::config::load_config(None).effective_max_parallel_pipelines()
            }),
        },
        Box::new(DesktopLock(state.worker_lock.clone())),
        state.worker_stopped.clone(),
    )
    .without_mr_poll();
    worker.start().await
}

struct DesktopStore(Arc<SqlitePipelineStore>);

#[async_trait]
impl PipelineStore for DesktopStore {
    async fn load(
        &self,
        pipeline_id: &str,
    ) -> Result<Pipeline, Box<dyn std::error::Error + Send + Sync>> {
        self.0
            .load(pipeline_id)?
            .ok_or_else(|| format!("Pipeline not found: {pipeline_id}").into())
    }

    fn save_stage_tx(&self, stage: Option<&Stage>, pipeline: &Pipeline, events: &[PipelineEvent]) {
        if let Err(error) = self.0.save_stage_tx(stage, pipeline, events) {
            tracing::warn!(error = %error, "worker save_stage_tx failed");
        }
    }

    async fn find_by_status(
        &self,
        status: &str,
    ) -> Result<Vec<Pipeline>, Box<dyn std::error::Error + Send + Sync>> {
        let parsed: PipelineStatus =
            serde_json::from_value(serde_json::Value::String(status.into()))?;
        Ok(self.0.find_by_status(parsed)?)
    }
}

struct DesktopQueue(Arc<PipelineQueue>);

impl Queue for DesktopQueue {
    fn dequeue(&self) -> Option<String> {
        match self.0.dequeue() {
            Ok(id) => id,
            Err(error) => {
                tracing::warn!(error = %error, "pipeline queue dequeue failed");
                None
            }
        }
    }
}

struct DesktopLock(Arc<WorkerLock>);

impl FileLock for DesktopLock {
    fn acquire(&self) -> bool {
        match self.0.acquire() {
            Ok(acquired) => acquired,
            Err(error) => {
                tracing::warn!(error = %error, "worker lock acquire failed");
                false
            }
        }
    }

    fn release(&self) {
        if let Err(error) = self.0.release() {
            tracing::warn!(error = %error, "worker lock release failed");
        }
    }
}

struct DesktopRunner {
    app: AppHandle,
    current_run: Arc<Mutex<HashSet<String>>>,
    runtime: AutoRunRuntime,
}

#[async_trait]
impl PipelineRun for DesktopRunner {
    async fn run_pipeline(
        &self,
        pipeline_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Ok(mut current) = self.current_run.lock() {
            current.insert(pipeline_id.to_string());
        }
        run_auto_loop(
            self.app.clone(),
            self.runtime.clone(),
            pipeline_id.to_string(),
        )
        .await;
        if let Ok(mut current) = self.current_run.lock() {
            current.remove(pipeline_id);
        }
        Ok(())
    }
}

struct DesktopRecovery {
    store: Arc<SqlitePipelineStore>,
    queue: Arc<PipelineQueue>,
}

#[async_trait]
impl Recovery for DesktopRecovery {
    async fn recover_all(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let recovery = PipelineRecovery::new(self.store.clone());
        let result = recovery
            .recover_all(Some(&DesktopWorktreeCleaner), Some(&DesktopRenotify))
            .await?;
        for error in &result.errors {
            tracing::warn!(error = %error, "pipeline recovery error");
        }
        for id in result.running_recovered {
            self.queue.enqueue(&id, 10)?;
        }
        for pipeline in self.store.find_by_status(PipelineStatus::Created)? {
            self.queue.enqueue(&pipeline.id, 0)?;
        }
        Ok(())
    }
}

struct DesktopWorktreeCleaner;

#[async_trait]
impl WorktreeCleaner for DesktopWorktreeCleaner {
    async fn clean_dirty_state(&self, repo_name: &str, pipeline_id: &str) -> Result<(), String> {
        let repo = RepoConfig {
            name: repo_name.to_string(),
            git_url: String::new(),
            branch: String::new(),
            base_branch: String::new(),
            gitlab_project_path: String::new(),
            depends_on: None,
            build_cmd: None,
        };
        let root = get_workspaces_root(None).to_string_lossy().into_owned();
        WorktreeResource::new(Some(root))
            .clean_dirty_state(&repo, pipeline_id)
            .await
            .map_err(|error| error.to_string())
    }
}

struct DesktopRenotify;

#[async_trait]
impl HumanLoopRenotify for DesktopRenotify {
    async fn renotify(&self, pipeline: &Pipeline, stage: &Stage) -> Result<(), String> {
        SkillsHumanLoop::renotify(&HumanLoopCoordinator::new(), pipeline, stage)
            .await
            .map_err(|error| error.to_string())
    }
}

pub(crate) fn request_worker_stop(stopped: &AtomicBool) {
    stopped.store(true, Ordering::SeqCst);
}
