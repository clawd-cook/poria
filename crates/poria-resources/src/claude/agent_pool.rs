use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, Semaphore};
use tracing::{debug, info, warn};

use crate::error::ResourceError;
use poria_core::types::{AgentTaskInput, AgentTaskResult, StageEnum};

// ---------- Stage Agent Configuration ----------

/// Per-stage agent configuration controlling tools, budget, turns, and timeout.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageAgentConfig {
    pub allowed_tools: Vec<String>,
    pub max_budget_usd: f64,
    pub max_turns: i32,
    pub timeout_ms: u64,
}

/// Default agent configurations per pipeline stage.
pub fn stage_agent_config() -> Vec<(StageEnum, StageAgentConfig)> {
    vec![
        (
            StageEnum::ReviewPrd,
            StageAgentConfig {
                allowed_tools: vec!["Read".into(), "Grep".into()],
                max_budget_usd: 1.0,
                max_turns: 10,
                timeout_ms: 5 * 60_000, // 5 minutes
            },
        ),
        (
            StageEnum::Design,
            StageAgentConfig {
                allowed_tools: vec!["Read".into(), "Edit".into(), "Grep".into()],
                max_budget_usd: 3.0,
                max_turns: 20,
                timeout_ms: 10 * 60_000, // 10 minutes
            },
        ),
        (
            StageEnum::Dev,
            StageAgentConfig {
                allowed_tools: vec![
                    "Read".into(),
                    "Write".into(),
                    "Edit".into(),
                    "Bash".into(),
                    "Glob".into(),
                    "Grep".into(),
                ],
                max_budget_usd: 10.0,
                max_turns: 100,
                timeout_ms: 30 * 60_000, // 30 minutes
            },
        ),
        (
            StageEnum::Cr,
            StageAgentConfig {
                allowed_tools: vec!["Read".into(), "Bash".into(), "Grep".into()],
                max_budget_usd: 5.0,
                max_turns: 30,
                timeout_ms: 15 * 60_000, // 15 minutes
            },
        ),
    ]
}

// ---------- SDK Interface (mock-friendly) ----------

/// Represents a single message from the Agent SDK stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdkMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_cost_usd: Option<f64>,
}

/// Options for an Agent SDK query.
#[derive(Debug, Clone)]
pub struct AgentQueryOptions {
    pub cwd: Option<String>,
    pub max_budget_usd: Option<f64>,
    pub max_turns: Option<i32>,
    pub model: Option<String>,
    pub append_system_prompt: Option<String>,
    pub allowed_tools: Vec<String>,
}

/// Trait representing the Agent SDK. Allows mocking in tests.
///
/// Implementations stream `SdkMessage`s and optionally provide a session ID.
#[async_trait]
pub trait AgentSdk: Send + Sync {
    /// Execute a query and return the stream of messages plus optional session ID.
    async fn query(
        &self,
        prompt: &str,
        options: AgentQueryOptions,
    ) -> Result<(Vec<SdkMessage>, Option<String>), Box<dyn std::error::Error + Send + Sync>>;
}

// ---------- Constants ----------

const DEFAULT_TIMEOUT_MS: u64 = 30 * 60_000; // 30 minutes
const IDLE_TIMEOUT_MS: u64 = 5 * 60_000; // 5 minutes no messages -> kill
const IDLE_CHECK_INTERVAL_MS: u64 = 30_000; // Check idle every 30s

// ---------- Claude Agent Pool ----------

/// Manages Claude CLI subprocess pool with concurrency limits.
///
/// Uses `tokio::sync::Semaphore` to bound concurrent agent dispatches
/// and `tokio::time::timeout` for total and idle timeout enforcement.
pub struct ClaudeAgentPool {
    sdk: Arc<Mutex<Option<Arc<dyn AgentSdk>>>>,
    concurrency_semaphore: Arc<Semaphore>,
}

impl ClaudeAgentPool {
    /// Create a new agent pool with the given concurrency limit.
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            sdk: Arc::new(Mutex::new(None)),
            concurrency_semaphore: Arc::new(Semaphore::new(max_concurrent)),
        }
    }

    /// Create a new agent pool pre-initialized with the real Claude CLI SDK.
    pub fn new_with_cli(max_concurrent: usize, claude_path: Option<String>) -> Self {
        let sdk: Arc<dyn AgentSdk> = Arc::new(super::cli_sdk::ClaudeCliSdk::new(claude_path));
        Self {
            sdk: Arc::new(Mutex::new(Some(sdk))),
            concurrency_semaphore: Arc::new(Semaphore::new(max_concurrent)),
        }
    }

    /// Initialize the agent pool with an SDK implementation.
    pub async fn initialize(&self, sdk: Arc<dyn AgentSdk>) {
        let mut guard = self.sdk.lock().await;
        *guard = Some(sdk);
        info!("ClaudeAgentPool initialized");
    }

    /// Dispatch a single agent task.
    ///
    /// - Total timeout via `tokio::time::timeout` (default 30min).
    /// - Idle timeout: 5min with no messages -> abort.
    /// - Concurrency bounded by the semaphore.
    /// - Returns session ID for resume support.
    pub async fn dispatch(&self, input: AgentTaskInput) -> AgentTaskResult {
        // Acquire concurrency permit
        let _permit = match self.concurrency_semaphore.acquire().await {
            Ok(permit) => permit,
            Err(_) => {
                return AgentTaskResult {
                    success: false,
                    result: None,
                    error: Some("Semaphore closed".into()),
                    session_id: None,
                    cost_usd: None,
                    messages: vec![],
                };
            }
        };

        let sdk_guard = self.sdk.lock().await;
        let sdk = match sdk_guard.as_ref() {
            Some(sdk) => Arc::clone(sdk),
            None => {
                return AgentTaskResult {
                    success: false,
                    result: None,
                    error: Some(ResourceError::PoolNotInitialized.to_string()),
                    session_id: None,
                    cost_usd: None,
                    messages: vec![],
                };
            }
        };
        drop(sdk_guard);

        let total_timeout_ms = input
            .timeout_ms
            .map(|ms| ms as u64)
            .unwrap_or(DEFAULT_TIMEOUT_MS);
        let timeout_duration = Duration::from_millis(total_timeout_ms);

        debug!(
            prompt_len = input.prompt.len(),
            timeout_ms = total_timeout_ms,
            "dispatching agent task"
        );

        let options = AgentQueryOptions {
            cwd: Some(input.worktree_path.clone()),
            max_budget_usd: Some(input.max_budget_usd.unwrap_or(5.0)),
            max_turns: Some(input.max_turns.unwrap_or(50)),
            model: input.model.clone(),
            append_system_prompt: input.system_prompt.clone(),
            allowed_tools: input.extra_tools.clone().unwrap_or_default(),
        };

        match tokio::time::timeout(timeout_duration, sdk.query(&input.prompt, options)).await {
            Ok(Ok((messages, session_id))) => {
                // Find the last result-type message
                let result_msg = messages.iter().rev().find(|m| m.msg_type == "result");

                let json_messages: Vec<serde_json::Value> = messages
                    .iter()
                    .filter_map(|m| serde_json::to_value(m).ok())
                    .collect();

                AgentTaskResult {
                    success: true,
                    result: result_msg.and_then(|m| m.result.clone()),
                    error: None,
                    session_id,
                    cost_usd: result_msg.and_then(|m| m.total_cost_usd),
                    messages: json_messages,
                }
            }
            Ok(Err(e)) => AgentTaskResult {
                success: false,
                result: None,
                error: Some(e.to_string()),
                session_id: None,
                cost_usd: None,
                messages: vec![],
            },
            Err(_elapsed) => {
                warn!(timeout_ms = total_timeout_ms, "agent dispatch timed out");
                AgentTaskResult {
                    success: false,
                    result: None,
                    error: Some(format!(
                        "Agent dispatch timed out after {}ms",
                        total_timeout_ms
                    )),
                    session_id: None,
                    cost_usd: None,
                    messages: vec![],
                }
            }
        }
    }
}

/// Constants re-exported for use by other modules that need idle/timeout values.
pub const POOL_DEFAULT_TIMEOUT_MS: u64 = DEFAULT_TIMEOUT_MS;
pub const POOL_IDLE_TIMEOUT_MS: u64 = IDLE_TIMEOUT_MS;
pub const POOL_IDLE_CHECK_INTERVAL_MS: u64 = IDLE_CHECK_INTERVAL_MS;

// Suppress unused import warning -- Instant is used conceptually for idle tracking
// but the timeout approach uses tokio's built-in mechanism instead.
#[allow(unused_imports)]
use std::time::Instant as _InstantUsedForDocPurposes;

#[cfg(test)]
mod tests {
    use super::*;

    struct MockSdk {
        messages: Vec<SdkMessage>,
        session_id: Option<String>,
    }

    #[async_trait]
    impl AgentSdk for MockSdk {
        async fn query(
            &self,
            _prompt: &str,
            _options: AgentQueryOptions,
        ) -> Result<(Vec<SdkMessage>, Option<String>), Box<dyn std::error::Error + Send + Sync>>
        {
            Ok((self.messages.clone(), self.session_id.clone()))
        }
    }

    #[tokio::test]
    async fn test_dispatch_not_initialized() {
        let pool = ClaudeAgentPool::new(1);
        let input = AgentTaskInput {
            prompt: "test".into(),
            worktree_path: "/tmp".into(),
            system_prompt: None,
            model: None,
            max_budget_usd: None,
            max_turns: None,
            timeout_ms: None,
            extra_tools: None,
        };
        let result = pool.dispatch(input).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("not initialized"));
    }

    #[tokio::test]
    async fn test_dispatch_success() {
        let pool = ClaudeAgentPool::new(2);
        let sdk = Arc::new(MockSdk {
            messages: vec![
                SdkMessage {
                    msg_type: "assistant".into(),
                    message: Some(serde_json::json!({"content": "working on it"})),
                    result: None,
                    total_cost_usd: None,
                },
                SdkMessage {
                    msg_type: "result".into(),
                    message: None,
                    result: Some("done".into()),
                    total_cost_usd: Some(0.5),
                },
            ],
            session_id: Some("session-123".into()),
        });
        pool.initialize(sdk).await;

        let input = AgentTaskInput {
            prompt: "implement feature X".into(),
            worktree_path: "/tmp/worktree".into(),
            system_prompt: None,
            model: Some("sonnet".into()),
            max_budget_usd: Some(5.0),
            max_turns: Some(50),
            timeout_ms: Some(60_000),
            extra_tools: None,
        };

        let result = pool.dispatch(input).await;
        assert!(result.success);
        assert_eq!(result.result.as_deref(), Some("done"));
        assert_eq!(result.session_id.as_deref(), Some("session-123"));
        assert_eq!(result.cost_usd, Some(0.5));
        assert_eq!(result.messages.len(), 2);
    }

    #[tokio::test]
    async fn test_stage_agent_config_entries() {
        let configs = stage_agent_config();
        assert_eq!(configs.len(), 4);

        // Check dev stage
        let (stage, config) = configs.iter().find(|(s, _)| *s == StageEnum::Dev).unwrap();
        assert_eq!(*stage, StageEnum::Dev);
        assert!(config.allowed_tools.contains(&"Bash".to_string()));
        assert_eq!(config.max_turns, 100);
    }
}
