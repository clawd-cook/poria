//! poria-resources -- Resource layer for the Poria platform.
//!
//! Provides:
//! - `TerminalResource` -- shell command execution with timeout
//! - `WorktreeResource` -- git worktree lifecycle management
//! - `ClaudeAgentPool` -- Claude CLI subprocess pool with concurrency limits
//! - `OutputGuard` -- validates agent output (file scope, diff size, dependencies)
//! - `SessionTracker` -- tracks active agent sessions

pub mod claude;
pub mod error;
pub mod terminal;
pub mod worktree;

// Re-export primary types for convenience
pub use error::ResourceError;

pub use terminal::{
    git_add_all, git_clone, git_commit, git_current_branch, git_fetch, git_has_changes,
    git_list_branches, git_push_set_upstream, git_status_porcelain, git_sync_hosted_clone,
    TerminalExecInput, TerminalExecResult, TerminalResource,
};
pub use worktree::{WorktreeCreateInput, WorktreeCreateResult, WorktreeResource};

pub use claude::agent_pool::{
    stage_agent_config, AgentQueryOptions, AgentSdk, ClaudeAgentPool, SdkMessage, StageAgentConfig,
};
pub use claude::cli_sdk::ClaudeCliSdk;
pub use claude::output_guard::{
    AgentOutput, DependencyEntry, GuardResult, OutputGuard, OutputGuardConfig, Violation,
    ViolationSeverity, ViolationType,
};
pub use claude::path::{
    parse_which_stdout, probe_claude_cli, resolve_claude_path, ClaudePathSource, ClaudeProbeResult,
    ResolvedClaudePath,
};
pub use claude::session_tracker::SessionTracker;
