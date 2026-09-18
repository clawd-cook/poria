use poria_core::pipeline::PipelineEvent;
use poria_core::types::{Pipeline, RollbackCommand, RollbackCommandType, Stage, StageEnum};

use crate::traits::{CodingChannel, FileSystem, Messenger, PipelineStore, Terminal};

/// Parses a GitLab MR URL into its project path and IID.
fn parse_mr_url(url: &str) -> (String, i64) {
    // GitLab MR URLs: https://coding.jd.com/group/project/-/merge_requests/123
    let re = regex::Regex::new(r"/([^/]+/[^/]+)/-/merge_requests/(\d+)").unwrap();
    match re.captures(url) {
        Some(caps) => {
            let project_path = caps.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
            let iid = caps
                .get(2)
                .and_then(|m| m.as_str().parse::<i64>().ok())
                .unwrap_or(0);
            (project_path, iid)
        }
        None => (String::new(), 0),
    }
}

/// Dependencies for rollback execution.
pub struct RollbackDeps {
    pub store: Box<dyn PipelineStore>,
    pub terminal: Box<dyn Terminal>,
    pub coding_channel: Box<dyn CodingChannel>,
    pub messenger: Box<dyn Messenger>,
    pub fs: Box<dyn FileSystem>,
}

/// Handles rolling back a pipeline, either by closing MRs and cleaning up
/// resources (unmerged) or by creating revert MRs (merged).
pub struct PipelineRollback {
    deps: RollbackDeps,
}

impl PipelineRollback {
    pub fn new(deps: RollbackDeps) -> Self {
        Self { deps }
    }

    pub async fn execute(
        &self,
        pipeline_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let pipeline = self.deps.store.load(pipeline_id).await?;
        let mr_urls = Self::deploy_mr_urls(&pipeline);

        let has_merged = self.check_any_merged(&mr_urls).await;

        if has_merged {
            self.rollback_merged(&pipeline).await;
        } else {
            self.rollback_unmerged(&pipeline).await;
        }

        Ok(())
    }

    fn deploy_mr_urls(pipeline: &Pipeline) -> Vec<String> {
        poria_core::pipeline::collect_deploy_mr_urls(pipeline)
    }

    async fn rollback_unmerged(&self, pipeline: &Pipeline) {
        let mut events: Vec<PipelineEvent> = Vec::new();
        let stages_reversed: Vec<&Stage> = pipeline.stages.iter().rev().collect();

        for stage in stages_reversed {
            if let Some(ref rollback) = stage.rollback {
                for cmd in &rollback.commands {
                    self.execute_rollback_command(cmd, &pipeline.id, &mut events)
                        .await;
                }
            }
        }

        if !events.is_empty() {
            self.deps.store.save_stage_tx(None, pipeline, &events);
        }
    }

    async fn rollback_merged(&self, pipeline: &Pipeline) {
        let mut events: Vec<PipelineEvent> = Vec::new();

        for repo in &pipeline.repos {
            let deploy_stage = pipeline.stages.iter().find(|s| s.name == StageEnum::Deploy);
            let repo_output = Self::get_repo_output(deploy_stage);

            let branch = repo_output
                .and_then(|o| o.get("branch"))
                .and_then(|v| v.as_str());

            let branch = match branch {
                Some(b) => b.to_string(),
                None => {
                    tracing::warn!("No branch found for repo {}, skipping revert", repo.name);
                    continue;
                }
            };

            let worktree_path = repo_output
                .and_then(|o| o.get("worktreePath"))
                .and_then(|v| v.as_str())
                .map(String::from);

            match self
                .do_revert(pipeline, repo, &branch, worktree_path.as_deref())
                .await
            {
                Ok(mr_url) => {
                    events.push(PipelineEvent::rollback_executed(
                        &pipeline.id,
                        "revert_mr",
                        format!("Revert MR created: {}", mr_url),
                    ));

                    if !pipeline.operator.is_empty() {
                        let _ = self
                            .deps
                            .messenger
                            .send(
                                &format!(
                                    "[Poria Rollback] Revert MR created for {}: {}",
                                    repo.name, mr_url
                                ),
                                &pipeline.operator,
                            )
                            .await;
                    }
                }
                Err(e) => {
                    tracing::warn!("Revert failed for repo {}: {}", repo.name, e);
                    events.push(PipelineEvent::rollback_executed(
                        &pipeline.id,
                        "revert_mr_failed",
                        format!("Repo {}: {}", repo.name, e),
                    ));
                }
            }
        }

        if !events.is_empty() {
            self.deps.store.save_stage_tx(None, pipeline, &events);
        }
    }

    async fn do_revert(
        &self,
        pipeline: &Pipeline,
        repo: &poria_core::types::RepoConfig,
        branch: &str,
        cwd: Option<&str>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        self.deps
            .terminal
            .exec("git revert --no-edit HEAD", cwd, Some(120_000))
            .await?;

        self.deps
            .terminal
            .exec(&format!("git push origin {}", branch), cwd, Some(120_000))
            .await?;

        let title = pipeline
            .demand_name
            .as_deref()
            .unwrap_or(&pipeline.demand_code);

        let mr_result = self
            .deps
            .coding_channel
            .create_merge_request(serde_json::json!({
                "projectPath": repo.gitlab_project_path,
                "sourceBranch": branch,
                "targetBranch": repo.base_branch,
                "title": format!("Revert: {}", title),
            }))
            .await?;

        Ok(mr_result.url)
    }

    async fn execute_rollback_command(
        &self,
        cmd: &RollbackCommand,
        pipeline_id: &str,
        events: &mut Vec<PipelineEvent>,
    ) {
        let result = self
            .try_execute_rollback_command(cmd, pipeline_id, events)
            .await;
        if let Err(e) = result {
            tracing::warn!("Rollback command {:?} failed: {}", cmd.command_type, e);
            events.push(PipelineEvent::rollback_executed(
                pipeline_id,
                format!("{:?}_failed", cmd.command_type),
                e.to_string(),
            ));
        }
    }

    async fn try_execute_rollback_command(
        &self,
        cmd: &RollbackCommand,
        pipeline_id: &str,
        events: &mut Vec<PipelineEvent>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match cmd.command_type {
            RollbackCommandType::CloseMr => {
                let mr_url = cmd.params.get("mrUrl").map(|s| s.as_str()).unwrap_or("");
                let status = self.deps.coding_channel.get_mr_status(mr_url).await?;
                if status == "closed" || status == "merged" {
                    events.push(PipelineEvent::rollback_executed(
                        pipeline_id,
                        "close_mr",
                        format!("Skipped (already {}): {}", status, mr_url),
                    ));
                    return Ok(());
                }
                let (project_path, iid) = parse_mr_url(mr_url);
                self.deps
                    .coding_channel
                    .close_mr(&project_path, iid)
                    .await?;
                events.push(PipelineEvent::rollback_executed(
                    pipeline_id,
                    "close_mr",
                    mr_url,
                ));
            }
            RollbackCommandType::DeleteBranch => {
                let branch = cmd.params.get("branch").map(|s| s.as_str()).unwrap_or("");
                let _ = self
                    .deps
                    .terminal
                    .exec(
                        &format!("git push origin --delete {}", branch),
                        None,
                        Some(30_000),
                    )
                    .await;
                events.push(PipelineEvent::rollback_executed(
                    pipeline_id,
                    "delete_branch",
                    branch,
                ));
            }
            RollbackCommandType::RemoveWorktree => {
                let path = cmd.params.get("path").map(|s| s.as_str()).unwrap_or("");
                if !self.deps.fs.file_exists(path) {
                    events.push(PipelineEvent::rollback_executed(
                        pipeline_id,
                        "remove_worktree",
                        format!("Skipped (not found): {}", path),
                    ));
                    return Ok(());
                }
                self.deps
                    .terminal
                    .exec(
                        &format!("git worktree remove --force {}", path),
                        None,
                        Some(30_000),
                    )
                    .await?;
                events.push(PipelineEvent::rollback_executed(
                    pipeline_id,
                    "remove_worktree",
                    path,
                ));
            }
            RollbackCommandType::RemoveDirectory => {
                let path = cmd.params.get("path").map(|s| s.as_str()).unwrap_or("");
                let dir = std::path::Path::new(path);
                poria_infrastructure::auth::assert_path_under_workspaces_root(None, dir)?;
                if dir.exists() {
                    std::fs::remove_dir_all(dir)?;
                }
                events.push(PipelineEvent::rollback_executed(
                    pipeline_id,
                    "remove_directory",
                    path,
                ));
            }
            _ => {
                events.push(PipelineEvent::rollback_executed(
                    pipeline_id,
                    format!("{:?}", cmd.command_type),
                    "Unknown command type",
                ));
            }
        }
        Ok(())
    }

    async fn check_any_merged(&self, mr_urls: &[String]) -> bool {
        for url in mr_urls {
            if let Ok(status) = self.deps.coding_channel.get_mr_status(url).await {
                if status == "merged" {
                    return true;
                }
            }
        }
        false
    }

    fn get_repo_output(stage: Option<&Stage>) -> Option<&serde_json::Value> {
        stage?.output.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mr_url_valid() {
        let (path, iid) = parse_mr_url("https://coding.jd.com/group/project/-/merge_requests/123");
        assert_eq!(path, "group/project");
        assert_eq!(iid, 123);
    }

    #[test]
    fn test_parse_mr_url_invalid() {
        let (path, iid) = parse_mr_url("https://example.com/something");
        assert_eq!(path, "");
        assert_eq!(iid, 0);
    }

    #[test]
    fn test_parse_mr_url_nested_group() {
        let (path, iid) =
            parse_mr_url("https://coding.jd.com/a/nested/path/proj/-/merge_requests/42");
        assert_eq!(path, "path/proj");
        assert_eq!(iid, 42);
    }
}
