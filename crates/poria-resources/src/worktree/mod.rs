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
        let remote_base = format!("refs/remotes/origin/{base_branch}");
        let has_local = git_ref_exists(Path::new(&git_root_str), &local_ref).await?;
        let has_remote_base = git_ref_exists(Path::new(&git_root_str), &remote_base).await?;
        let command = if has_local {
            format!("git worktree add \"{worktree_str}\" \"{branch}\"")
        } else if has_remote_base {
            format!(
                "git worktree add -b \"{branch}\" \"{worktree_str}\" \"origin/{base_branch}\""
            )
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

    /// Create a detached (read-only) worktree at `origin/<branch>` or the local branch.
    /// Hosted clone can stay on the default branch; same-branch lock is avoided.
    pub async fn create_detached(
        &self,
        repo_name: &str,
        pipeline_id: &str,
        git_root: &Path,
        branch: &str,
    ) -> Result<WorktreeCreateResult, ResourceError> {
        let worktree_path = self.workspace_root.join(pipeline_id).join(repo_name);
        let worktree_str = worktree_path.to_string_lossy().into_owned();
        let git_root_str = git_root.to_string_lossy().into_owned();
        let branch = branch.trim();
        if branch.is_empty() {
            return Err(ResourceError::Other("后端分支不能为空".into()));
        }

        if worktree_path.exists() {
            if worktree_path.join(".git").exists() {
                info!(
                    repo = %repo_name,
                    branch = %branch,
                    worktree_path = %worktree_str,
                    "reusing existing detached git worktree"
                );
                return Ok(WorktreeCreateResult {
                    worktree_path: worktree_str,
                    branch: branch.to_string(),
                });
            }
            return Err(ResourceError::Other(format!(
                "worktree path already exists and is not a git worktree: {worktree_str}"
            )));
        }

        if let Some(parent) = worktree_path.parent() {
            std::fs::create_dir_all(parent).map_err(ResourceError::SpawnError)?;
        }

        let remote_ref = format!("refs/remotes/origin/{branch}");
        let local_ref = format!("refs/heads/{branch}");
        let start_point = if git_ref_exists(Path::new(&git_root_str), &remote_ref).await? {
            format!("origin/{branch}")
        } else if git_ref_exists(Path::new(&git_root_str), &local_ref).await? {
            branch.to_string()
        } else {
            return Err(ResourceError::Other(format!(
                "找不到后端分支 {branch}（origin/{branch} 与本地均不存在）"
            )));
        };

        info!(
            repo = %repo_name,
            start_point = %start_point,
            worktree_path = %worktree_str,
            git_root = %git_root_str,
            "creating detached git worktree"
        );

        let command = format!("git worktree add --detach \"{worktree_str}\" \"{start_point}\"");
        let result = terminal::exec(terminal::TerminalExecInput {
            command,
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
                "git worktree add --detach failed: {detail}"
            )));
        }

        Ok(WorktreeCreateResult {
            worktree_path: worktree_str,
            branch: branch.to_string(),
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
        self.remove_path(Path::new("."), &worktree_path).await
    }

    /// Remove a worktree using the hosted clone as `git_root`.
    pub async fn remove_path(
        &self,
        git_root: &Path,
        worktree_path: &Path,
    ) -> Result<(), ResourceError> {
        if !worktree_path.exists() {
            debug!(
                worktree_path = %worktree_path.display(),
                "worktree does not exist, skipping remove"
            );
            return Ok(());
        }

        let worktree_str = worktree_path.to_string_lossy().into_owned();
        let git_root_str = git_root.to_string_lossy().into_owned();

        info!(
            worktree_path = %worktree_str,
            "removing git worktree"
        );

        let command = format!("git worktree remove --force \"{worktree_str}\"");
        let _ = terminal::exec(terminal::TerminalExecInput {
            command,
            cwd: Some(git_root_str),
            env: None,
            timeout_ms: Some(GIT_MEDIUM_TIMEOUT),
        })
        .await;
        if worktree_path.exists() {
            std::fs::remove_dir_all(worktree_path).map_err(ResourceError::SpawnError)?;
        }
        Ok(())
    }
}

async fn git_ref_exists(git_root: &Path, spec: &str) -> Result<bool, ResourceError> {
    crate::terminal::git_ref_exists(git_root, spec).await
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

    fn unique_temp(prefix: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "{prefix}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    async fn init_repo_with_master(path: &Path) {
        std::fs::create_dir_all(path).unwrap();
        std::fs::write(path.join("README"), "hi").unwrap();
        let init = terminal::exec(terminal::TerminalExecInput {
            command: "git init -b master && git config user.email test@example.com && git config user.name test && git add README && git -c commit.gpgsign=false commit -m init".into(),
            cwd: Some(path.to_string_lossy().into_owned()),
            env: None,
            timeout_ms: Some(15_000),
        })
        .await
        .unwrap();
        assert_eq!(init.code, 0, "git init failed: {}", init.stderr);
    }

    #[tokio::test]
    async fn create_detached_keeps_hosted_clone_on_master() {
        let root = unique_temp("poria-wt-detach");
        let hosted = root.join("hosted");
        init_repo_with_master(&hosted).await;
        let add_branch = terminal::exec(terminal::TerminalExecInput {
            command: "git checkout -b develop && git -c commit.gpgsign=false commit --allow-empty -m develop && git checkout master".into(),
            cwd: Some(hosted.to_string_lossy().into_owned()),
            env: None,
            timeout_ms: Some(15_000),
        })
        .await
        .unwrap();
        assert_eq!(add_branch.code, 0, "{}", add_branch.stderr);

        let resource = WorktreeResource::new(Some(root.join("worktrees").to_string_lossy().into_owned()));
        let created = resource
            .create_detached("ls-api", "pipe-1", &hosted, "master")
            .await
            .unwrap();
        assert!(Path::new(&created.worktree_path).join("README").exists());
        assert_eq!(
            terminal::git_current_branch(&hosted).await.unwrap(),
            "master"
        );
        let detached = terminal::git_current_branch(Path::new(&created.worktree_path))
            .await
            .unwrap();
        assert_eq!(detached, "HEAD");

        resource
            .remove_path(&hosted, Path::new(&created.worktree_path))
            .await
            .unwrap();
        assert!(!Path::new(&created.worktree_path).exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn create_feature_from_local_base_does_not_move_hosted_branch() {
        let root = unique_temp("poria-wt-feature");
        let hosted = root.join("hosted");
        init_repo_with_master(&hosted).await;
        let resource = WorktreeResource::new(Some(root.join("worktrees").to_string_lossy().into_owned()));
        let mut repo = test_repo();
        repo.name = "ls-entrance".into();
        repo.branch = "feature_TEST".into();
        repo.base_branch = "master".into();
        let created = resource
            .create(&repo, "pipe-1", "master", &hosted)
            .await
            .unwrap();
        assert_eq!(created.branch, "feature_TEST");
        assert_eq!(
            terminal::git_current_branch(Path::new(&created.worktree_path))
                .await
                .unwrap(),
            "feature_TEST"
        );
        assert_eq!(
            terminal::git_current_branch(&hosted).await.unwrap(),
            "master"
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}
