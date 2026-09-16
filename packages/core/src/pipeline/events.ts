import type { StageEnum, StageOutput } from "../types/pipeline.js";
import type { DemandMetadata } from "../types/demand.js";
import type { GateResult } from "../types/gate.js";
import type { IssueClass } from "../types/issue.js";

interface EventBase {
  pipelineId: string;
  timestamp: Date;
}

export type PipelineEvent =
  | PipelineCreated
  | PipelineStarted
  | PipelineCompleted
  | PipelineFailed
  | PipelineCancelled
  | PipelineWaitingMerge
  | StageStarted
  | StageCompleted
  | StageFailed
  | StageBlocked
  | StageResumed
  | StageRegressed
  | AgentDispatched
  | AgentProgress
  | AgentCompleted
  | AgentFailed
  | GateEvaluated
  | GateRegressTriggered
  | HumanAssistRequested
  | HumanAssistReceived
  | HumanAssistEscalated
  | CredentialRefreshed
  | CredentialExpired
  | GitCommit
  | GitPush
  | MrCreated
  | MrMerged
  | WorktreeCreated
  | WorktreeCleaned
  | RollbackExecuted;

interface PipelineCreated extends EventBase { kind: "pipeline_created"; demandRef: DemandMetadata }
interface PipelineStarted extends EventBase { kind: "pipeline_started" }
interface PipelineCompleted extends EventBase { kind: "pipeline_completed" }
interface PipelineFailed extends EventBase { kind: "pipeline_failed"; reason: string }
interface PipelineCancelled extends EventBase { kind: "pipeline_cancelled"; operator: string }
interface PipelineWaitingMerge extends EventBase { kind: "pipeline_waiting_merge"; mrUrls: string[] }

interface StageStarted extends EventBase { kind: "stage_started"; stage: StageEnum }
interface StageCompleted extends EventBase { kind: "stage_completed"; stage: StageEnum; output: StageOutput }
interface StageFailed extends EventBase { kind: "stage_failed"; stage: StageEnum; error: string; retryCount: number }
interface StageBlocked extends EventBase { kind: "stage_blocked"; stage: StageEnum; issueClass: IssueClass }
interface StageResumed extends EventBase { kind: "stage_resumed"; stage: StageEnum; resolution: string }
interface StageRegressed extends EventBase { kind: "stage_regressed"; from: StageEnum; to: StageEnum; reason: string }

interface AgentDispatched extends EventBase { kind: "agent_dispatched"; stage: StageEnum; sessionId: string }
interface AgentProgress extends EventBase { kind: "agent_progress"; stage: StageEnum; message: string }
interface AgentCompleted extends EventBase { kind: "agent_completed"; stage: StageEnum; costUsd: number }
interface AgentFailed extends EventBase { kind: "agent_failed"; stage: StageEnum; error: string }

interface GateEvaluated extends EventBase { kind: "gate_evaluated"; stage: StageEnum; results: GateResult[] }
interface GateRegressTriggered extends EventBase { kind: "gate_regress_triggered"; rule: string; from: StageEnum; to: StageEnum }

interface HumanAssistRequested extends EventBase { kind: "human_assist_requested"; issueClass: IssueClass; target: string }
interface HumanAssistReceived extends EventBase { kind: "human_assist_received"; action: string; message: string }
interface HumanAssistEscalated extends EventBase { kind: "human_assist_escalated"; level: number }

interface CredentialRefreshed extends EventBase { kind: "credential_refreshed" }
interface CredentialExpired extends EventBase { kind: "credential_expired"; stage: StageEnum }

interface GitCommit extends EventBase { kind: "git_commit"; repo: string; hash: string }
interface GitPush extends EventBase { kind: "git_push"; repo: string; branch: string }
interface MrCreated extends EventBase { kind: "mr_created"; repo: string; url: string; iid: number }
interface MrMerged extends EventBase { kind: "mr_merged"; repo: string; url: string }
interface WorktreeCreated extends EventBase { kind: "worktree_created"; repo: string; path: string }
interface WorktreeCleaned extends EventBase { kind: "worktree_cleaned"; repo: string; reason: string }
interface RollbackExecuted extends EventBase { kind: "rollback_executed"; type: string; detail: string }

function base(pipelineId: string): EventBase {
  return { pipelineId, timestamp: new Date() };
}

export function pipelineCreatedEvent(pipelineId: string, demandRef: DemandMetadata): PipelineCreated {
  return { ...base(pipelineId), kind: "pipeline_created", demandRef };
}
export function pipelineStartedEvent(pipelineId: string): PipelineStarted {
  return { ...base(pipelineId), kind: "pipeline_started" };
}
export function pipelineCompletedEvent(pipelineId: string): PipelineCompleted {
  return { ...base(pipelineId), kind: "pipeline_completed" };
}
export function pipelineFailedEvent(pipelineId: string, reason: string): PipelineFailed {
  return { ...base(pipelineId), kind: "pipeline_failed", reason };
}
export function pipelineCancelledEvent(pipelineId: string, operator: string): PipelineCancelled {
  return { ...base(pipelineId), kind: "pipeline_cancelled", operator };
}
export function pipelineWaitingMergeEvent(pipelineId: string, mrUrls: string[]): PipelineWaitingMerge {
  return { ...base(pipelineId), kind: "pipeline_waiting_merge", mrUrls };
}

export function stageStartedEvent(pipelineId: string, stage: StageEnum): StageStarted {
  return { ...base(pipelineId), kind: "stage_started", stage };
}
export function stageCompletedEvent(pipelineId: string, stage: StageEnum, output: StageOutput): StageCompleted {
  return { ...base(pipelineId), kind: "stage_completed", stage, output };
}
export function stageFailedEvent(pipelineId: string, stage: StageEnum, error: string, retryCount: number): StageFailed {
  return { ...base(pipelineId), kind: "stage_failed", stage, error, retryCount };
}
export function stageBlockedEvent(pipelineId: string, stage: StageEnum, issueClass: IssueClass): StageBlocked {
  return { ...base(pipelineId), kind: "stage_blocked", stage, issueClass };
}
export function stageResumedEvent(pipelineId: string, stage: StageEnum, resolution: string): StageResumed {
  return { ...base(pipelineId), kind: "stage_resumed", stage, resolution };
}
export function stageRegressedEvent(pipelineId: string, from: StageEnum, to: StageEnum, reason: string): StageRegressed {
  return { ...base(pipelineId), kind: "stage_regressed", from, to, reason };
}

export function agentDispatchedEvent(pipelineId: string, stage: StageEnum, sessionId: string): AgentDispatched {
  return { ...base(pipelineId), kind: "agent_dispatched", stage, sessionId };
}
export function agentProgressEvent(pipelineId: string, stage: StageEnum, message: string): AgentProgress {
  return { ...base(pipelineId), kind: "agent_progress", stage, message };
}
export function agentCompletedEvent(pipelineId: string, stage: StageEnum, costUsd: number): AgentCompleted {
  return { ...base(pipelineId), kind: "agent_completed", stage, costUsd };
}
export function agentFailedEvent(pipelineId: string, stage: StageEnum, error: string): AgentFailed {
  return { ...base(pipelineId), kind: "agent_failed", stage, error };
}

export function gateEvaluatedEvent(pipelineId: string, stage: StageEnum, results: GateResult[]): GateEvaluated {
  return { ...base(pipelineId), kind: "gate_evaluated", stage, results };
}
export function gateRegressTriggeredEvent(pipelineId: string, rule: string, from: StageEnum, to: StageEnum): GateRegressTriggered {
  return { ...base(pipelineId), kind: "gate_regress_triggered", rule, from, to };
}

export function humanAssistRequestedEvent(pipelineId: string, issueClass: IssueClass, target: string): HumanAssistRequested {
  return { ...base(pipelineId), kind: "human_assist_requested", issueClass, target };
}
export function humanAssistReceivedEvent(pipelineId: string, action: string, message: string): HumanAssistReceived {
  return { ...base(pipelineId), kind: "human_assist_received", action, message };
}
export function humanAssistEscalatedEvent(pipelineId: string, level: number): HumanAssistEscalated {
  return { ...base(pipelineId), kind: "human_assist_escalated", level };
}

export function credentialRefreshedEvent(pipelineId: string): CredentialRefreshed {
  return { ...base(pipelineId), kind: "credential_refreshed" };
}
export function credentialExpiredEvent(pipelineId: string, stage: StageEnum): CredentialExpired {
  return { ...base(pipelineId), kind: "credential_expired", stage };
}

export function gitCommitEvent(pipelineId: string, repo: string, hash: string): GitCommit {
  return { ...base(pipelineId), kind: "git_commit", repo, hash };
}
export function gitPushEvent(pipelineId: string, repo: string, branch: string): GitPush {
  return { ...base(pipelineId), kind: "git_push", repo, branch };
}
export function mrCreatedEvent(pipelineId: string, repo: string, url: string, iid: number): MrCreated {
  return { ...base(pipelineId), kind: "mr_created", repo, url, iid };
}
export function mrMergedEvent(pipelineId: string, repo: string, url: string): MrMerged {
  return { ...base(pipelineId), kind: "mr_merged", repo, url };
}
export function worktreeCreatedEvent(pipelineId: string, repo: string, path: string): WorktreeCreated {
  return { ...base(pipelineId), kind: "worktree_created", repo, path };
}
export function worktreeCleanedEvent(pipelineId: string, repo: string, reason: string): WorktreeCleaned {
  return { ...base(pipelineId), kind: "worktree_cleaned", repo, reason };
}
export function rollbackExecutedEvent(pipelineId: string, type: string, detail: string): RollbackExecuted {
  return { ...base(pipelineId), kind: "rollback_executed", type, detail };
}
