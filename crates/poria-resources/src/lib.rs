//! poria-resources -- Resource layer for the Poria platform.
//!
//! Provides:
//! - `TerminalResource` -- shell command execution with timeout
//! - `WorktreeResource` -- git worktree lifecycle management
//! - `ClaudeAgentPool` -- Claude CLI subprocess pool with concurrency limits
//! - `OutputGuard` -- validates agent output (file scope, diff size, dependencies)
//! - `SessionTracker` -- tracks active agent sessions

pub mod error;
pub mod terminal;
pub mod worktree;
pub mod claude;

// Re-export primary types for convenience
pub use error::ResourceError;

pub use terminal::{TerminalResource, TerminalExecInput, TerminalExecResult};
pub use worktree::{WorktreeResource, WorktreeCreateInput, WorktreeCreateResult};

pub use claude::agent_pool::{
    ClaudeAgentPool, StageAgentConfig, AgentSdk, AgentQueryOptions, SdkMessage,
    stage_agent_config,
};
pub use claude::cli_sdk::ClaudeCliSdk;
pub use claude::output_guard::{
    OutputGuard, OutputGuardConfig, AgentOutput, GuardResult,
    Violation, ViolationSeverity, ViolationType, DependencyEntry,
};
pub use claude::session_tracker::SessionTracker;
