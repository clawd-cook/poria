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

export type StageStatus = "pending" | "running" | "completed" | "failed" | "blocked" | "skipped";

export type StageEnum = "init" | "review_prd" | "design" | "workspace" | "dev" | "cr" | "deploy";

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
  agent_timeout_ms: number;
  claude_path: string | null;
  cr_score_threshold: string;
  db_path: string;
  max_diff_lines: number;
  max_retries: number;
  test_coverage_threshold: number;
}

export type ClaudePathSource = "config" | "which";

export interface ClaudeProbeResult {
  error: string | null;
  ok: boolean;
  resolvedPath: string | null;
  source: ClaudePathSource | null;
  version: string | null;
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

export type ViewType = "home" | "demands" | "repos" | "settings";

export type CloneStatus = "cloning" | "ready" | "failed";

export interface RegisteredRepo {
  clone_status: CloneStatus;
  created_at: string;
  error: string | null;
  git_url: string;
  id: string;
  local_path: string;
  name: string;
  normalized_url: string;
  scope: string;
  updated_at: string;
}

export interface DemandListItem {
  demand_code: string;
  id: number;
  name: string;
  receiver_erp: string | null;
  receiver_name: string | null;
  status: number | null;
  status_label: string;
}

export interface DemandPage {
  current: number;
  page_size: number;
  records: DemandListItem[];
  total: number;
}

export interface DemandPrdPreview {
  demand_code: string;
  demand_id: number;
  demand_name: string;
  url: string | null;
}

export interface DemandProjectFile {
  name: string;
  size: number;
}

export interface DemandProject {
  demand_code: string;
  exists: boolean;
  files: DemandProjectFile[];
  project_dir: string;
}

export interface DemandProjectFileContent {
  content: string;
  name: string;
}

export interface SubmitPipelineInput {
  backendBranch: string;
  backendRepoId: string;
  backendTrdUrl: string;
  demandCode?: string;
  demandId: number;
  demandName?: string;
  frontendRepoId: string;
  prdUrl: string;
}

export interface StreamChunk {
  type: "text" | "tool_use" | "tool_result" | "result";
  content: string;
  timestamp: string;
  tool_name?: string;
}
