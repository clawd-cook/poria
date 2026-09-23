import { pipelineHitlLane } from "./hitlLane";
import {
  stageLabel,
  type PipelineDetail,
  type PipelineStatus,
  type PipelineSummary,
  type WorkbenchTab,
} from "./types";

export type PipelineMode =
  | "awaiting_human"
  | "done"
  | "failed"
  | "running"
  | "setup"
  | "waiting_merge";

export type PrimaryCtaAction = "back_to_list" | "open_confirm" | "open_docs" | "open_mr" | "none";

export interface PrimaryCta {
  action: PrimaryCtaAction;
  label: string;
}

export interface PipelineViewModel {
  badges: { attentionCount: number; hasActiveHitl: boolean };
  mode: PipelineMode;
  primaryCta: PrimaryCta;
  stageLabel: string | null;
  suggestedTab: WorkbenchTab;
  title: string;
}

export function countAttentionPipelines(
  pipelines: PipelineSummary[],
  humanRequestPipelineId: string | null,
): number {
  const ids = new Set<string>();
  for (const pipeline of pipelines) {
    if (pipelineHitlLane(pipeline) != null) {
      ids.add(pipeline.id);
    }
  }
  if (humanRequestPipelineId) {
    ids.add(humanRequestPipelineId);
  }
  return ids.size;
}

function modeFromStatus(status: PipelineStatus | undefined, hasHitl: boolean): PipelineMode {
  if (hasHitl || status === "blocked") {
    return "awaiting_human";
  }
  switch (status) {
    case "waiting_merge":
      return "waiting_merge";
    case "completed":
    case "cancelled":
      return "done";
    case "failed":
      return "failed";
    case "created":
      return "setup";
    case "running":
    default:
      return status ? "running" : "setup";
  }
}

export function derivePipelineViewModel(input: {
  detail: PipelineDetail | null;
  humanRequestPipelineId: string | null;
  pipelines: PipelineSummary[];
  selectedPipelineId: string | null;
}): PipelineViewModel {
  const selected =
    input.pipelines.find((pipeline) => pipeline.id === input.selectedPipelineId) ?? null;
  const status = input.detail?.status ?? selected?.status;
  const hasHitl =
    (input.humanRequestPipelineId != null &&
      input.humanRequestPipelineId === input.selectedPipelineId) ||
    (selected != null && pipelineHitlLane(selected) != null) ||
    status === "blocked" ||
    status === "waiting_merge";

  const mode = modeFromStatus(status, hasHitl);
  const title = input.detail?.demand_name ?? selected?.demand_name ?? "需求工作台";
  const currentStage =
    input.detail?.stages.find((stage) => stage.status === "running" || stage.status === "blocked")
      ?.name ??
    selected?.current_stage ??
    null;

  let primaryCta: PrimaryCta;
  let suggestedTab: WorkbenchTab = "trajectory";

  switch (mode) {
    case "awaiting_human":
      primaryCta = { action: "open_confirm", label: "去对话确认" };
      suggestedTab = "confirm";
      break;
    case "waiting_merge":
      primaryCta = { action: "open_confirm", label: "确认可合并" };
      suggestedTab = "confirm";
      break;
    case "failed":
      primaryCta = { action: "open_confirm", label: "查看问题" };
      suggestedTab = "confirm";
      break;
    case "done":
      primaryCta = { action: "back_to_list", label: "返回列表" };
      break;
    case "setup":
      primaryCta = { action: "open_docs", label: "查看文档" };
      suggestedTab = "docs";
      break;
    case "running":
    default:
      primaryCta = { action: "none", label: "运行中" };
      break;
  }

  return {
    badges: {
      attentionCount: countAttentionPipelines(input.pipelines, input.humanRequestPipelineId),
      hasActiveHitl: hasHitl,
    },
    mode,
    primaryCta,
    stageLabel: currentStage ? stageLabel(currentStage) : null,
    suggestedTab,
    title,
  };
}
