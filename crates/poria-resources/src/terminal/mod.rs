use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use tracing::{debug, warn};

use crate::error::ResourceError;
use poria_core::contracts::{CapabilityMetadata, ResourceContext};

// ---------- Types ----------

/// Input for a terminal command execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalExecInput {
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

/// Result of a terminal command execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalExecResult {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

// ---------- Defaults ----------

const DEFAULT_TIMEOUT_MS: u64 = 30_000;
const GIT_CLONE_TIMEOUT_MS: u64 = 600_000;
const GIT_FETCH_TIMEOUT_MS: u64 = 60_000;
const GIT_PUSH_TIMEOUT_MS: u64 = 180_000;
const GIT_QUERY_TIMEOUT_MS: u64 = 15_000;

// ---------- Terminal Resource ----------

/// Shell command execution with timeout support.
///
/// Wraps `tokio::process::Command` with a configurable timeout.
/// When the timeout fires, the child process is killed and a `TimeoutError` is returned.
pub struct TerminalResource {
    metadata: CapabilityMetadata,
}

impl Default for TerminalResource {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalResource {
    pub fn new() -> Self {
        Self {
            metadata: CapabilityMetadata {
                id: "resource:terminal".into(),
                name: "Terminal".into(),
                description: "Shell command execution with timeout support".into(),
                version: "0.1.0".into(),
            },
        }
    }

    pub fn metadata(&self) -> &CapabilityMetadata {
        &self.metadata
    }
}

#[async_trait]
impl poria_core::contracts::Resource for TerminalResource {
    fn metadata(&self) -> &CapabilityMetadata {
        &self.metadata
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _ctx: ResourceContext,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let input: TerminalExecInput = serde_json::from_value(input)?;
        let result = exec(input).await?;
        Ok(serde_json::to_value(result)?)
    }
}

/// Execute a shell command with timeout support.
///
/// Uses `/bin/sh -c` (Unix) to run the command in a shell.
/// If the process does not complete within the timeout, it is killed
/// and a `ResourceError::Timeout` is returned.
pub async fn exec(input: TerminalExecInput) -> Result<TerminalExecResult, ResourceError> {
    let timeout_ms = match input.timeout_ms {
        Some(ms) if ms > 0 => ms,
        _ => DEFAULT_TIMEOUT_MS,
    };

    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(&input.command);

    if let Some(ref cwd) = input.cwd {
        cmd.current_dir(cwd);
    }

    if let Some(ref env) = input.env {
        for (key, value) in env {
            cmd.env(key, value);
        }
    }

    // Capture stdout and stderr
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    debug!(command = %input.command, timeout_ms, "spawning shell command");

    let child = cmd.spawn()?;

    let timeout_duration = std::time::Duration::from_millis(timeout_ms);

    match tokio::time::timeout(timeout_duration, child.wait_with_output()).await {
        Ok(Ok(output)) => {
            let code = output.status.code().unwrap_or(1);
            let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

            debug!(command = %input.command, code, "command completed");

            Ok(TerminalExecResult {
                code,
                stdout,
                stderr,
            })
        }
        Ok(Err(e)) => Err(ResourceError::SpawnError(e)),
        Err(_elapsed) => {
            warn!(
                command = %input.command,
                timeout_ms,
                "command timed out, killing process"
            );
            Err(ResourceError::Timeout {
                command: input.command,
                timeout_ms,
            })
        }
    }
}

/// Clone a git repository into `dest` using argv (not a shell) to avoid injection.
///
/// `dest` must not already exist; callers should remove leftover directories first.
pub async fn git_clone(git_url: &str, dest: &Path) -> Result<(), ResourceError> {
    git_clone_with_timeout(git_url, dest, GIT_CLONE_TIMEOUT_MS).await
}

pub async fn git_clone_with_timeout(
    git_url: &str,
    dest: &Path,
    timeout_ms: u64,
) -> Result<(), ResourceError> {
    let dest_str = dest.to_string_lossy().into_owned();
    let command = format!("git clone -- {git_url} {dest_str}");

    debug!(git_url, dest = %dest_str, timeout_ms, "cloning git repository");

    let mut cmd = Command::new("git");
    cmd.arg("clone")
        .arg("--")
        .arg(git_url)
        .arg(&dest_str)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let child = cmd.spawn()?;
    match tokio::time::timeout(
        std::time::Duration::from_millis(timeout_ms),
        child.wait_with_output(),
    )
    .await
    {
        Ok(Ok(output)) => {
            if output.status.success() {
                Ok(())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
                let detail = if !stderr.trim().is_empty() {
                    stderr.trim().to_string()
                } else {
                    stdout.trim().to_string()
                };
                Err(ResourceError::Other(format!("git clone failed: {detail}")))
            }
        }
        Ok(Err(e)) => Err(ResourceError::SpawnError(e)),
        Err(_elapsed) => {
            warn!(git_url, dest = %dest_str, timeout_ms, "git clone timed out");
            Err(ResourceError::Timeout {
                command,
                timeout_ms,
            })
        }
    }
}

pub(crate) async fn run_git(
    args: &[&str],
    cwd: Option<&Path>,
    timeout_ms: u64,
) -> Result<std::process::Output, ResourceError> {
    let command = format!("git {}", args.join(" "));
    let mut cmd = Command::new("git");
    cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }

    let child = cmd.spawn()?;
    match tokio::time::timeout(
        std::time::Duration::from_millis(timeout_ms),
        child.wait_with_output(),
    )
    .await
    {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(e)) => Err(ResourceError::SpawnError(e)),
        Err(_elapsed) => {
            warn!(command = %command, timeout_ms, "git command timed out");
            Err(ResourceError::Timeout {
                command,
                timeout_ms,
            })
        }
    }
}

fn git_failure(action: &str, output: &std::process::Output) -> ResourceError {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let detail = if !stderr.trim().is_empty() {
        stderr.trim().to_string()
    } else {
        stdout.trim().to_string()
    };
    ResourceError::Other(format!("{action} failed: {detail}"))
}

/// Fetch remotes in a managed clone. Callers should ignore offline failures.
pub async fn git_fetch(repo_path: &Path) -> Result<(), ResourceError> {
    let output = run_git(
        &["fetch", "--all", "--prune"],
        Some(repo_path),
        GIT_FETCH_TIMEOUT_MS,
    )
    .await?;
    if output.status.success() {
        Ok(())
    } else {
        Err(git_failure("git fetch", &output))
    }
}

pub async fn git_ref_exists(repo_path: &Path, spec: &str) -> Result<bool, ResourceError> {
    let output = run_git(
        &["show-ref", "--verify", "--quiet", spec],
        Some(repo_path),
        GIT_QUERY_TIMEOUT_MS,
    )
    .await?;
    Ok(output.status.success())
}

fn git_output_detail(output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    if stderr.trim().is_empty() {
        stdout.trim().to_string()
    } else {
        stderr.trim().to_string()
    }
}

/// Fetch origin and fast-forward the hosted clone to `origin/<default_branch>`.
/// Fails when the working tree is dirty or the update is not a fast-forward.
pub async fn git_sync_hosted_clone(
    repo_path: &Path,
    default_branch: &str,
) -> Result<(), ResourceError> {
    let branch = default_branch.trim();
    if branch.is_empty() {
        return Err(ResourceError::Other("主分支不能为空".into()));
    }
    if branch.contains("..") || branch.starts_with('-') || branch.contains(char::is_whitespace) {
        return Err(ResourceError::Other("主分支名称不合法".into()));
    }

    git_fetch(repo_path).await?;
    if git_has_changes(repo_path).await? {
        return Err(ResourceError::Other(
            "托管副本有未提交改动，无法同步".into(),
        ));
    }

    let origin_ref = format!("refs/remotes/origin/{branch}");
    if !git_ref_exists(repo_path, &origin_ref).await? {
        return Err(ResourceError::Other(format!(
            "远程不存在 origin/{branch}，无法同步"
        )));
    }

    let local_ref = format!("refs/heads/{branch}");
    let origin_branch = format!("origin/{branch}");
    if git_ref_exists(repo_path, &local_ref).await? {
        let checkout = run_git(&["checkout", branch], Some(repo_path), GIT_QUERY_TIMEOUT_MS).await?;
        if !checkout.status.success() {
            return Err(ResourceError::Other(format!(
                "git checkout failed: {}",
                git_output_detail(&checkout)
            )));
        }
        let merge = run_git(
            &["merge", "--ff-only", &origin_branch],
            Some(repo_path),
            GIT_QUERY_TIMEOUT_MS,
        )
        .await?;
        if !merge.status.success() {
            return Err(ResourceError::Other(format!(
                "git merge --ff-only failed: {}",
                git_output_detail(&merge)
            )));
        }
    } else {
        let checkout = run_git(
            &["checkout", "-B", branch, &origin_branch],
            Some(repo_path),
            GIT_QUERY_TIMEOUT_MS,
        )
        .await?;
        if !checkout.status.success() {
            return Err(ResourceError::Other(format!(
                "git checkout failed: {}",
                git_output_detail(&checkout)
            )));
        }
    }
    Ok(())
}

/// `git status --porcelain` in a worktree.
pub async fn git_status_porcelain(repo_path: &Path) -> Result<String, ResourceError> {
    let output = run_git(
        &["status", "--porcelain"],
        Some(repo_path),
        GIT_QUERY_TIMEOUT_MS,
    )
    .await?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(git_failure("git status", &output))
    }
}

/// True when `git status --porcelain` is non-empty.
pub async fn git_has_changes(repo_path: &Path) -> Result<bool, ResourceError> {
    let porcelain = git_status_porcelain(repo_path).await?;
    Ok(!porcelain.trim().is_empty())
}

/// Stage all tracked and untracked changes (`git add -A`).
pub async fn git_add_all(repo_path: &Path) -> Result<(), ResourceError> {
    let output = run_git(&["add", "-A"], Some(repo_path), GIT_QUERY_TIMEOUT_MS).await?;
    if output.status.success() {
        Ok(())
    } else {
        Err(git_failure("git add", &output))
    }
}

/// Commit staged changes. Returns `false` when there is nothing to commit.
pub async fn git_commit(repo_path: &Path, message: &str) -> Result<bool, ResourceError> {
    let output = run_git(
        &["commit", "-m", message],
        Some(repo_path),
        GIT_QUERY_TIMEOUT_MS,
    )
    .await?;
    if output.status.success() {
        return Ok(true);
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stderr}\n{stdout}");
    if combined.contains("nothing to commit") {
        return Ok(false);
    }
    Err(git_failure("git commit", &output))
}

/// Push a branch and set upstream (`git push -u <remote> <branch>`).
pub async fn git_push_set_upstream(
    repo_path: &Path,
    remote: &str,
    branch: &str,
) -> Result<(), ResourceError> {
    let output = run_git(
        &["push", "-u", remote, branch],
        Some(repo_path),
        GIT_PUSH_TIMEOUT_MS,
    )
    .await?;
    if output.status.success() {
        Ok(())
    } else {
        Err(git_failure("git push", &output))
    }
}

/// Current checkout of a managed clone (`git rev-parse --abbrev-ref HEAD`).
pub async fn git_current_branch(repo_path: &Path) -> Result<String, ResourceError> {
    let output = run_git(
        &["rev-parse", "--abbrev-ref", "HEAD"],
        Some(repo_path),
        GIT_QUERY_TIMEOUT_MS,
    )
    .await?;
    if !output.status.success() {
        return Err(git_failure("git rev-parse", &output));
    }
    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if branch.is_empty() {
        return Err(ResourceError::Other(
            "git rev-parse returned empty branch".into(),
        ));
    }
    Ok(branch)
}

/// Normalize `git for-each-ref` names into unique short branch names.
pub fn normalize_git_ref_names<I, S>(refs: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut seen = std::collections::BTreeSet::new();
    for raw in refs {
        let name = raw.as_ref().trim();
        if name.is_empty() {
            continue;
        }
        let branch = if let Some(rest) = name.strip_prefix("refs/heads/") {
            rest
        } else if let Some(rest) = name.strip_prefix("refs/remotes/") {
            match rest.split_once('/') {
                Some((_, "HEAD")) => continue,
                Some((_, branch)) => branch,
                None => continue,
            }
        } else if name == "HEAD" || name.ends_with("/HEAD") {
            continue;
        } else if let Some(rest) = name.strip_prefix("origin/") {
            rest
        } else {
            name
        };
        if branch.is_empty() || branch == "HEAD" {
            continue;
        }
        seen.insert(branch.to_string());
    }
    seen.into_iter().collect()
}

/// List unique local + remote branch names in a managed clone.
pub async fn git_list_branches(repo_path: &Path) -> Result<Vec<String>, ResourceError> {
    let output = run_git(
        &[
            "for-each-ref",
            "--format=%(refname)",
            "refs/heads",
            "refs/remotes",
        ],
        Some(repo_path),
        GIT_QUERY_TIMEOUT_MS,
    )
    .await?;
    if !output.status.success() {
        return Err(git_failure("git for-each-ref", &output));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(normalize_git_ref_names(stdout.lines()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_exec_simple_command() {
        let input = TerminalExecInput {
            command: "echo hello".into(),
            cwd: None,
            env: None,
            timeout_ms: None,
        };
        let result = exec(input).await.unwrap();
        assert_eq!(result.code, 0);
        assert_eq!(result.stdout.trim(), "hello");
    }

    #[tokio::test]
    async fn test_exec_with_cwd() {
        let input = TerminalExecInput {
            command: "pwd".into(),
            cwd: Some("/tmp".into()),
            env: None,
            timeout_ms: None,
        };
        let result = exec(input).await.unwrap();
        assert_eq!(result.code, 0);
        // macOS resolves /tmp -> /private/tmp
        assert!(
            result.stdout.trim() == "/tmp" || result.stdout.trim() == "/private/tmp",
            "unexpected cwd: {}",
            result.stdout.trim()
        );
    }

    #[tokio::test]
    async fn test_exec_with_env() {
        let mut env = HashMap::new();
        env.insert("MY_VAR".into(), "my_value".into());
        let input = TerminalExecInput {
            command: "echo $MY_VAR".into(),
            cwd: None,
            env: Some(env),
            timeout_ms: None,
        };
        let result = exec(input).await.unwrap();
        assert_eq!(result.code, 0);
        assert_eq!(result.stdout.trim(), "my_value");
    }

    #[tokio::test]
    async fn test_exec_timeout() {
        let input = TerminalExecInput {
            command: "sleep 10".into(),
            cwd: None,
            env: None,
            timeout_ms: Some(100),
        };
        let result = exec(input).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ResourceError::Timeout {
                command,
                timeout_ms,
            } => {
                assert_eq!(command, "sleep 10");
                assert_eq!(timeout_ms, 100);
            }
            other => panic!("expected Timeout error, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_exec_nonzero_exit_code() {
        let input = TerminalExecInput {
            command: "exit 42".into(),
            cwd: None,
            env: None,
            timeout_ms: None,
        };
        let result = exec(input).await.unwrap();
        assert_eq!(result.code, 42);
    }

    #[tokio::test]
    async fn test_exec_captures_stderr() {
        let input = TerminalExecInput {
            command: "echo err >&2".into(),
            cwd: None,
            env: None,
            timeout_ms: None,
        };
        let result = exec(input).await.unwrap();
        assert_eq!(result.code, 0);
        assert_eq!(result.stderr.trim(), "err");
    }

    #[tokio::test]
    async fn test_git_clone_local_repo() {
        let root = std::env::temp_dir().join(format!(
            "poria-git-clone-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let src = root.join("src");
        let dest = root.join("dest");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("README"), "hi").unwrap();

        let init = exec(TerminalExecInput {
            command: "git init && git config user.email test@example.com && git config user.name test && git add README && git -c commit.gpgsign=false commit -m init".into(),
            cwd: Some(src.to_string_lossy().into_owned()),
            env: None,
            timeout_ms: Some(15_000),
        })
        .await
        .unwrap();
        assert_eq!(init.code, 0, "git init failed: {}", init.stderr);

        git_clone_with_timeout(&src.to_string_lossy(), &dest, 15_000)
            .await
            .unwrap();
        assert!(dest.join("README").exists());

        let branch = git_current_branch(&dest).await.unwrap();
        assert!(!branch.is_empty());
        let branches = git_list_branches(&dest).await.unwrap();
        assert!(
            branches.iter().any(|name| name == &branch),
            "expected {branch} in {branches:?}"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    fn unique_temp(prefix: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "{prefix}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    async fn init_bare_origin_with_master(origin: &std::path::Path) {
        let seed = origin.parent().unwrap().join("seed");
        std::fs::create_dir_all(&seed).unwrap();
        std::fs::write(seed.join("README"), "base").unwrap();
        let init = exec(TerminalExecInput {
            command: "git init -b master && git config user.email test@example.com && git config user.name test && git add README && git -c commit.gpgsign=false commit -m init".into(),
            cwd: Some(seed.to_string_lossy().into_owned()),
            env: None,
            timeout_ms: Some(15_000),
        })
        .await
        .unwrap();
        assert_eq!(init.code, 0, "seed init failed: {}", init.stderr);
        let clone = exec(TerminalExecInput {
            command: format!(
                "git clone --bare {} {}",
                seed.to_string_lossy(),
                origin.to_string_lossy()
            ),
            cwd: None,
            env: None,
            timeout_ms: Some(15_000),
        })
        .await
        .unwrap();
        assert_eq!(clone.code, 0, "bare clone failed: {}", clone.stderr);
        let _ = std::fs::remove_dir_all(&seed);
    }

    #[tokio::test]
    async fn git_sync_hosted_clone_fast_forwards_default_branch() {
        let root = unique_temp("poria-git-sync");
        let origin = root.join("origin.git");
        let hosted = root.join("hosted");
        std::fs::create_dir_all(&root).unwrap();
        init_bare_origin_with_master(&origin).await;

        let clone = exec(TerminalExecInput {
            command: format!(
                "git clone {} {}",
                origin.to_string_lossy(),
                hosted.to_string_lossy()
            ),
            cwd: None,
            env: None,
            timeout_ms: Some(15_000),
        })
        .await
        .unwrap();
        assert_eq!(clone.code, 0, "clone failed: {}", clone.stderr);

        let work = root.join("work");
        let clone_work = exec(TerminalExecInput {
            command: format!(
                "git clone {} {}",
                origin.to_string_lossy(),
                work.to_string_lossy()
            ),
            cwd: None,
            env: None,
            timeout_ms: Some(15_000),
        })
        .await
        .unwrap();
        assert_eq!(clone_work.code, 0, "{}", clone_work.stderr);
        std::fs::write(work.join("NEXT"), "next").unwrap();
        let push = exec(TerminalExecInput {
            command: "git config user.email test@example.com && git config user.name test && git add NEXT && git -c commit.gpgsign=false commit -m next && git push origin master".into(),
            cwd: Some(work.to_string_lossy().into_owned()),
            env: None,
            timeout_ms: Some(15_000),
        })
        .await
        .unwrap();
        assert_eq!(push.code, 0, "push failed: {}", push.stderr);

        git_sync_hosted_clone(&hosted, "master").await.unwrap();
        assert_eq!(git_current_branch(&hosted).await.unwrap(), "master");
        assert!(hosted.join("NEXT").exists());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn git_sync_hosted_clone_fails_when_dirty() {
        let root = unique_temp("poria-git-sync-dirty");
        let origin = root.join("origin.git");
        let hosted = root.join("hosted");
        std::fs::create_dir_all(&root).unwrap();
        init_bare_origin_with_master(&origin).await;
        let clone = exec(TerminalExecInput {
            command: format!(
                "git clone {} {}",
                origin.to_string_lossy(),
                hosted.to_string_lossy()
            ),
            cwd: None,
            env: None,
            timeout_ms: Some(15_000),
        })
        .await
        .unwrap();
        assert_eq!(clone.code, 0, "{}", clone.stderr);
        std::fs::write(hosted.join("DIRTY"), "nope").unwrap();
        let err = git_sync_hosted_clone(&hosted, "master")
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("未提交改动"), "{err}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn normalize_git_ref_names_strips_remotes_and_head() {
        let names = normalize_git_ref_names([
            "refs/heads/main",
            "refs/remotes/origin/HEAD",
            "refs/remotes/origin/main",
            "refs/remotes/origin/feat/foo",
            "HEAD",
            "origin/HEAD",
            "  ",
        ]);
        assert_eq!(names, vec!["feat/foo".to_string(), "main".to_string()]);
    }
}
