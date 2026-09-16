import type { RepoConfig } from "./repo.js";
import type { GateRule } from "./gate.js";
import type { RollbackInstruction } from "./rollback.js";

export type StageEnum =
  | "init"
  | "review_prd"
  | "design"
  | "workspace"
  | "dev"
  | "cr"
  | "deploy";

export const STAGE_ORDER: readonly StageEnum[] = [
  "init",
  "review_prd",
  "design",
  "workspace",
  "dev",
  "cr",
  "deploy",
] as const;

export type PipelineStatus =
  | "created"
  | "running"
  | "waiting_merge"
  | "blocked"
  | "completed"
  | "failed"
  | "cancelled";

export type StageStatus =
  | "pending"
  | "running"
  | "completed"
  | "failed"
  | "blocked"
  | "skipped";

export interface Stage {
  id?: number;
  pipelineId: string;
  name: StageEnum;
  status: StageStatus;
  skillId?: string;
  retryCount: number;
  maxRetries: number;
  input?: Record<string, unknown>;
  output?: Record<string, unknown>;
  gateResults?: Record<string, unknown>;
  issue?: StageIssue;
  rollback?: RollbackInstruction;
  agentSessionId?: string;
  startedAt?: Date;
  completedAt?: Date;
}

export interface StageIssue {
  class: string;
  message: string;
  retryable: boolean;
}

export interface Pipeline {
  id: string;
  demandId: number;
  demandCode: string;
  demandName?: string;
  status: PipelineStatus;
  rawLink: string;
  operator: string;
  hasRegressed: boolean;
  config: PipelineConfig;
  stages: Stage[];
  repos: RepoConfig[];
  createdAt: Date;
  updatedAt: Date;
}

export interface PipelineConfig {
  gates: GateRule[];
  trdScope: string[];
  repos: RepoConfig[];
}

export interface StageOutput {
  [key: string]: unknown;
}

export interface SkillInput {
  stage: Stage;
  pipeline: Pipeline;
  [key: string]: unknown;
}

export interface SkillOutput {
  output: StageOutput;
  gatesPass?: boolean;
}
