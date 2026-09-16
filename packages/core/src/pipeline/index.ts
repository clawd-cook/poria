export {
  InvalidTransitionError,
  canPipelineTransition,
  canStageTransition,
  transitionPipeline,
  transitionStage,
} from "./state-machine.js";

export type { PipelineEvent } from "./events.js";
export {
  pipelineCreatedEvent,
  pipelineStartedEvent,
  pipelineCompletedEvent,
  pipelineFailedEvent,
  pipelineCancelledEvent,
  pipelineWaitingMergeEvent,
  stageStartedEvent,
  stageCompletedEvent,
  stageFailedEvent,
  stageBlockedEvent,
  stageResumedEvent,
  stageRegressedEvent,
  agentDispatchedEvent,
  agentProgressEvent,
  agentCompletedEvent,
  agentFailedEvent,
  gateEvaluatedEvent,
  gateRegressTriggeredEvent,
  humanAssistRequestedEvent,
  humanAssistReceivedEvent,
  humanAssistEscalatedEvent,
  credentialRefreshedEvent,
  credentialExpiredEvent,
  gitCommitEvent,
  gitPushEvent,
  mrCreatedEvent,
  mrMergedEvent,
  worktreeCreatedEvent,
  worktreeCleanedEvent,
  rollbackExecutedEvent,
} from "./events.js";

export type { StageResult } from "./gates.js";
export {
  crScoreMeetsThreshold,
  evaluate as evaluateGates,
  DEFAULT_GATES,
} from "./gates.js";

export { createPipelineId } from "./id.js";

export { CircularDependencyError, topologicalSort } from "./multi-repo.js";

export { classifyRisk } from "./risk-classifier.js";
export type { RiskLevel } from "./risk-classifier.js";
