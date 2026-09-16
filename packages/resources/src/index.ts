// @poria/resources — Resource layer barrel export
// Terminal, Worktree, Claude Agent Pool, Output Guard, Session Tracker

export { TerminalResource, TimeoutError } from "./terminal/index.js";
export type { TerminalExecInput, TerminalExecResult } from "./terminal/index.js";

export { WorktreeResource } from "./worktree/index.js";
export type { WorktreeCreateInput, WorktreeCreateResult } from "./worktree/index.js";

export { ClaudeAgentPool, STAGE_AGENT_CONFIG } from "./claude/agent-pool.js";
export type { StageAgentConfig } from "./claude/agent-pool.js";

export { OutputGuard } from "./claude/output-guard.js";
export type {
  OutputGuardConfig,
  AgentOutput,
  GuardResult,
  Violation,
  ViolationSeverity,
  ViolationType,
} from "./claude/output-guard.js";

export { SessionTracker } from "./claude/session-tracker.js";
