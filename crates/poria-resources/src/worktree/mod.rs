use std::path::{Path, PathBuf};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::error::ResourceError;
use crate::terminal;
use poria_core::contracts::{CapabilityMetadata, ResourceContext};
use poria_core::types::RepoConfig;

// ---------- Types ----------

/// Input for creating a git worktree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeCreateInput {
    pub repo: RepoConfig,
    pub pipeline_id: String,
    pub base_branch: String,
    #[serde(default)]
    pub git_root: String,
}

/// Result of creating a git worktree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeCreateResult {
    pub worktree_path: String,
    pub branch: String,
}

// ---------- Constants ----------

/// Default workspace root where worktrees are created.
const DEFAULT_WORKSPACE_ROOT: &str = "workspace/projects";

/// Timeout for git operations (2 minutes).
const GIT_MEDIUM_TIMEOUT: u64 = 120_000;

// ---------- Worktree Resource ----------

/// Git worktree lifecycle management: create, clean, remove.
///
/// Manages git worktrees by shelling out to `git worktree` commands
/// via `TerminalResource`.
pub struct WorktreeResource {
    metadata: CapabilityMetadata,
    workspace_root: PathBuf,
}

impl Default for WorktreeResource {
    fn default() -> Self {
        Self::new(None)
    }
}

impl WorktreeResource {
    pub fn new(workspace_root: Option<String>) -> Self {
        Self {
            metadata: CapabilityMetadata {
                id: "resource:worktree".into(),
                name: "Git Worktree".into(),
                description: "Git worktree lifecycle management: create, clean, remove".into(),
                version: "0.1.0".into(),
            },
            workspace_root: PathBuf::from(
                workspace_root.unwrap_or_else(|| DEFAULT_WORKSPACE_ROOT.into()),
            ),
        }
    }

    /// Return the conventional worktree path for a repo within a pipeline.
    /// Format: `{workspace_root}/{pipeline_id}/{repo_name}`
    pub fn path(&self, repo: &RepoConfig, pipeline_id: &str) -> PathBuf {
        self.workspace_root.join(pipeline_id).join(&repo.name)
    }

    /// Create a git worktree, or reuse it when the path already has `repo.branch`.
    pub async fn create(
        &self,
        repo: &RepoConfig,
        pipeline_id: &str,
        base_branch: &str,
        git_root: &Path,
    ) -> Result<WorktreeCreateResult, ResourceError> {
        let worktree_path = self.path(repo, pipeline_id);
        let branch = &repo.branch;
        let worktree_str = worktree_path.to_string_lossy().into_owned();
        let git_root_str = git_root.to_string_lossy().into_owned();

        if worktree_path.exists() {
            let existing = terminal::git_current_branch(&worktree_path).await?;
            if existing == *branch {
                info!(
                    repo = %repo.name,
                    branch = %branch,
                    worktree_path = %worktree_str,
                    "reusing existing git worktree"
                );
                return Ok(WorktreeCreateResult {
                    worktree_path: worktree_str,
                    branch: branch.clone(),
                });
            }
            return Err(ResourceError::Other(format!(
                "worktree already exists at {worktree_str} on branch {existing}, expected {branch}"
            )));
        }

        if let Some(parent) = worktree_path.parent() {
            std::fs::create_dir_all(parent).map_err(ResourceError::SpawnError)?;
        }

        info!(
            repo = %repo.name,
            branch = %branch,
            worktree_path = %worktree_str,
            git_root = %git_root_str,
            "creating git worktree"
        );

        let local_ref = format!("refs/heads/{branch}");
        let remote_ref = format!("refs/remotes/origin/{branch}");
        let has_local = git_ref_exists(&git_root_str, &local_ref).await?;
        let has_remote = git_ref_exists(&git_root_str, &remote_ref).await?;
        let command = if has_local {
            format!("git worktree add \"{worktree_str}\" \"{branch}\"")
        } else if has_remote {
            format!("git worktree add -b \"{branch}\" \"{worktree_str}\" \"origin/{branch}\"")
        } else {
            format!("git worktree add -b \"{branch}\" \"{worktree_str}\" \"{base_branch}\"")
        };

        let result = terminal::exec(terminal::TerminalExecInput {
            command: command.clone(),
            cwd: Some(git_root_str),
            env: None,
            timeout_ms: Some(GIT_MEDIUM_TIMEOUT),
        })
        .await?;
        if result.code != 0 {
            let detail = if result.stderr.trim().is_empty() {
                result.stdout.trim().to_string()
            } else {
                result.stderr.trim().to_string()
            };
            return Err(ResourceError::Other(format!(
                "git worktree add failed: {detail}"
            )));
        }

        Ok(WorktreeCreateResult {
            worktree_path: worktree_str,
            branch: branch.clone(),
        })
    }

    /// Clean dirty state in a worktree (recovery scenario).
    /// Discards uncommitted changes: `git checkout .` + `git clean -fd`.
    pub async fn clean_dirty_state(
        &self,
        repo: &RepoConfig,
        pipeline_id: &str,
    ) -> Result<(), ResourceError> {
        let worktree_path = self.path(repo, pipeline_id);

        if !worktree_path.exists() {
            debug!(
                worktree_path = %worktree_path.display(),
                "worktree does not exist, skipping clean"
            );
            return Ok(());
        }

        let cwd = worktree_path.to_string_lossy().into_owned();

        // Discard tracked file changes
        terminal::exec(terminal::TerminalExecInput {
            command: "git checkout .".into(),
            cwd: Some(cwd.clone()),
            env: None,
            timeout_ms: Some(GIT_MEDIUM_TIMEOUT),
        })
        .await?;

        // Remove untracked files and directories
        terminal::exec(terminal::TerminalExecInput {
            command: "git clean -fd".into(),
            cwd: Some(cwd),
            env: None,
            timeout_ms: Some(GIT_MEDIUM_TIMEOUT),
        })
        .await?;

        Ok(())
    }

    /// Remove a worktree. Checks existence first, uses --force for locked worktrees.
    pub async fn remove(&self, repo: &RepoConfig, pipeline_id: &str) -> Result<(), ResourceError> {
        let worktree_path = self.path(repo, pipeline_id);

        if !worktree_path.exists() {
            debug!(
                worktree_path = %worktree_path.display(),
                "worktree does not exist, skipping remove"
            );
            return Ok(());
        }

        let worktree_str = worktree_path.to_string_lossy();

        info!(
            worktree_path = %worktree_str,
            "removing git worktree"
        );

        let command = format!("git worktree remove --force \"{}\"", worktree_str);

        terminal::exec(terminal::TerminalExecInput {
            command,
            cwd: Some(self.resolve_repo_root().to_string()),
            env: None,
            timeout_ms: Some(GIT_MEDIUM_TIMEOUT),
        })
        .await?;

        Ok(())
    }

    fn resolve_repo_root(&self) -> &str {
        "."
    }
}

async fn git_ref_exists(git_root: &str, spec: &str) -> Result<bool, ResourceError> {
    let result = terminal::exec(terminal::TerminalExecInput {
        command: format!("git show-ref --verify --quiet {spec}"),
        cwd: Some(git_root.to_string()),
        env: None,
        timeout_ms: Some(GIT_MEDIUM_TIMEOUT),
    })
    .await?;
    Ok(result.code == 0)
}

#[async_trait]
impl poria_core::contracts::Resource for WorktreeResource {
    fn metadata(&self) -> &CapabilityMetadata {
        &self.metadata
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _ctx: ResourceContext,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let input: WorktreeCreateInput = serde_json::from_value(input)?;
        if input.git_root.trim().is_empty() {
            return Err("missing git_root for worktree create".into());
        }
        let result = self
            .create(
                &input.repo,
                &input.pipeline_id,
                &input.base_branch,
                Path::new(&input.git_root),
            )
            .await?;
        Ok(serde_json::to_value(result)?)
    }
}

// Suppress dead_code warning for resolve_repo_root since it is used but clippy
// can't see through the string borrow.
#[allow(dead_code)]
fn _assert_path_type(_: &Path) {}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_repo() -> RepoConfig {
        RepoConfig {
            name: "test-repo".into(),
            git_url: "https://example.com/test.git".into(),
            branch: "feature/test".into(),
            base_branch: "main".into(),
            gitlab_project_path: "group/test".into(),
            depends_on: None,
            build_cmd: None,
        }
    }

    #[test]
    fn test_path_construction() {
        let resource = WorktreeResource::new(Some("/tmp/ws".into()));
        let repo = test_repo();
        let path = resource.path(&repo, "pipeline-123");
        assert_eq!(path, PathBuf::from("/tmp/ws/pipeline-123/test-repo"));
    }

    #[test]
    fn test_default_workspace_root() {
        let resource = WorktreeResource::default();
        let repo = test_repo();
        let path = resource.path(&repo, "p1");
        assert_eq!(path, PathBuf::from("workspace/projects/p1/test-repo"));
    }
}
