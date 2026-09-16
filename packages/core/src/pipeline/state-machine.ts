import type { PipelineStatus, StageStatus } from "../types/pipeline.js";

export class InvalidTransitionError extends Error {
  constructor(
    public readonly entityType: "pipeline" | "stage",
    public readonly from: string,
    public readonly to: string,
  ) {
    super(`Invalid ${entityType} transition: ${from} → ${to}`);
    this.name = "InvalidTransitionError";
  }
}

const PIPELINE_TRANSITIONS: Record<PipelineStatus, readonly PipelineStatus[]> = {
  created: ["running", "cancelled"],
  running: ["waiting_merge", "blocked", "failed", "cancelled"],
  blocked: ["running", "cancelled"],
  waiting_merge: ["completed", "failed", "cancelled"],
  completed: [],
  failed: ["cancelled"],
  cancelled: [],
};

const STAGE_TRANSITIONS: Record<StageStatus, readonly StageStatus[]> = {
  pending: ["running", "skipped"],
  running: ["completed", "failed", "blocked"],
  completed: [],
  failed: ["running", "blocked"],
  blocked: ["running"],
  skipped: [],
};

export function canPipelineTransition(from: PipelineStatus, to: PipelineStatus): boolean {
  return PIPELINE_TRANSITIONS[from].includes(to);
}

export function canStageTransition(from: StageStatus, to: StageStatus): boolean {
  return STAGE_TRANSITIONS[from].includes(to);
}

export function transitionPipeline(
  pipeline: { status: PipelineStatus },
  to: PipelineStatus,
): void {
  if (!canPipelineTransition(pipeline.status, to)) {
    throw new InvalidTransitionError("pipeline", pipeline.status, to);
  }
  pipeline.status = to;
}

export function transitionStage(
  stage: { status: StageStatus },
  to: StageStatus,
): void {
  if (!canStageTransition(stage.status, to)) {
    throw new InvalidTransitionError("stage", stage.status, to);
  }
  stage.status = to;
}
