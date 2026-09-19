export interface PipelineSummary {
  id: string;
  demand_id: number;
  demand_name: string;
  demand_code: string;
  status: PipelineStatus;
  current_stage: string | null;
  created_at: string;
  updated_at: string;
  /** Blocked-stage issue class, or `waiting_merge` while polling MR. */
  issue_class: string | null;
  issue_detail: string | null;
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

export type StageEnum = "init" | "review_prd" | "design" | "dev" | "cr" | "deploy";

export const STAGE_ORDER: StageEnum[] = ["init", "review_prd", "design", "dev", "cr", "deploy"];

export const STAGE_LABELS: Record<StageEnum, string> = {
  cr: "代码审查",
  deploy: "部署",
  design: "技术设计",
  dev: "开发",
  init: "初始化",
  review_prd: "需求评审",
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
  workspace_path: string | null;
  cost_usd: number | null;
  duration_ms: number | null;
  hitl_count: number;
  created_at: string;
  updated_at: string;
}

export interface StageObservability {
  stage: StageEnum | string;
  completed: number;
  failed: number;
  blocked: number;
  success_rate: number | null;
  duration_p50_ms: number | null;
  duration_p95_ms: number | null;
  cost_usd: number;
}

export interface ObservabilitySummary {
  pipeline_total: number;
  pipeline_completed: number;
  pipeline_failed: number;
  pipeline_cancelled: number;
  pipeline_blocked: number;
  pipeline_waiting_merge: number;
  pipeline_running: number;
  success_rate: number | null;
  hitl_count: number;
  hitl_pipelines: number;
  hitl_rate: number | null;
  cost_usd_total: number;
  stages: StageObservability[];
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
  /** Newline-separated Dev verify commands. Null/empty → package.json convention. */
  dev_verify_commands: string | null;
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
  description: string;
  id: string;
  name: string;
  version: string;
}

export interface SkillDetail {
  description: string;
  id: string;
  markdown: string | null;
  name: string;
  version: string;
}

export interface ChannelInfo {
  id: string;
  name: string;
  description: string;
  version: string;
}

export type ViewType =
  | "channels"
  | "demands"
  | "home"
  | "repos"
  | "settings"
  | "skills"
  | "workspace";

export type CloneStatus = "cloning" | "failed" | "ready";

export type RepoSyncStatus = "failed" | "idle" | "synced" | "syncing";

export interface RegisteredRepo {
  clone_status: CloneStatus;
  created_at: string;
  default_branch: string;
  error: string | null;
  git_url: string;
  id: string;
  last_synced_at: string | null;
  local_path: string;
  name: string;
  normalized_url: string;
  scope: string;
  sync_error: string | null;
  sync_status: RepoSyncStatus;
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

export interface PrdReviewStatus {
  p0_done: boolean;
  p0_unanswered: string[];
  p1_done: boolean;
  p1_unanswered: string[];
  p2_done: boolean;
  p2_unanswered: string[];
}

export interface DemandProjectFileContent {
  content: string;
  name: string;
  review_status?: PrdReviewStatus | null;
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
