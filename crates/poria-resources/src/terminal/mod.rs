use std::collections::HashMap;

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
}
