use thiserror::Error;

/// Top-level error type for poria-resources.
#[derive(Debug, Error)]
pub enum ResourceError {
    #[error("Command timed out after {timeout_ms}ms: {command}")]
    Timeout { command: String, timeout_ms: u64 },

    #[error("Command failed to spawn: {0}")]
    SpawnError(#[from] std::io::Error),

    #[error("Agent pool not initialized -- call initialize() first")]
    PoolNotInitialized,

    #[error("Agent dispatch error: {0}")]
    AgentDispatch(String),

    #[error("{0}")]
    Other(String),
}
