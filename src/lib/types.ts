export interface PipelineSummary {
  id: string;
  demand_name: string;
  demand_code: string;
  status: PipelineStatus;
  current_stage: string | null;
  created_at: string;
  updated_at: string;
}

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

export type StageEnum =
  | "init"
  | "review_prd"
  | "design"
  | "workspace"
  | "dev"
  | "cr"
  | "deploy";

export const STAGE_ORDER: StageEnum[] = [
  "init",
  "review_prd",
  "design",
  "workspace",
  "dev",
  "cr",
  "deploy",
];

export const STAGE_LABELS: Record<StageEnum, string> = {
  init: "初始化",
  review_prd: "需求评审",
  design: "技术设计",
  workspace: "工作区",
  dev: "开发",
  cr: "代码审查",
  deploy: "部署",
};

export interface StageDetail {
  name: StageEnum;
  status: StageStatus;
  retry_count: number;
  output_summary: string | null;
  gate_results: string | null;
  issue: string | null;
  started_at: string | null;
  completed_at: string | null;
}

export interface PipelineDetail {
  id: string;
  demand_id: number;
  demand_code: string;
  demand_name: string;
  status: PipelineStatus;
  raw_link: string;
  operator: string;
  has_regressed: boolean;
  stages: StageDetail[];
  created_at: string;
  updated_at: string;
}

export interface PipelineEvent {
  seq: number;
  kind: string;
  payload: string;
  created_at: string;
}

export interface AuthStatus {
  logged_in: boolean;
  username: string | null;
  cookie_valid: boolean;
}

export interface AppConfig {
  cr_score_threshold: string;
  test_coverage_threshold: number;
  max_diff_lines: number;
  agent_timeout_ms: number;
  max_retries: number;
  db_path: string;
}

export interface SkillInfo {
  id: string;
  name: string;
  description: string;
  version: string;
}

export interface ChannelInfo {
  id: string;
  name: string;
  description: string;
  version: string;
}

export type ViewType = "pipeline" | "skills" | "channels";
