import { AlertTriangle, Check, Circle, Loader2, Minus, Play, SkipForward, X } from "lucide-react";

import type { StageDetail, StageStatus } from "../lib/types";
import { STAGE_LABELS, STAGE_ORDER } from "../lib/types";
import { executeStage, skipStage } from "../lib/tauri";
import { useStore } from "../state/store";
import { StreamOutput } from "./StreamOutput";

const STATUS_CONFIG: Record<
  StageStatus,
  { icon: typeof Check; bg: string; ring: string; animate?: boolean }
> = {
  completed: { icon: Check, bg: "bg-emerald-500", ring: "ring-emerald-500/30" },
  running: {
    icon: Loader2,
    bg: "bg-blue-500",
    ring: "ring-blue-500/30",
    animate: true,
  },
  pending: { icon: Circle, bg: "bg-slate-600", ring: "ring-slate-600/30" },
  failed: { icon: X, bg: "bg-red-500", ring: "ring-red-500/30" },
  blocked: {
    icon: AlertTriangle,
    bg: "bg-amber-500",
    ring: "ring-amber-500/30",
  },
  skipped: { icon: Minus, bg: "bg-slate-600", ring: "ring-slate-600/30" },
};

function StageNode({
  stage,
  isLast,
  pipelineId,
  isNextPending,
}: {
  stage: StageDetail;
  isLast: boolean;
  pipelineId: string | null;
  isNextPending: boolean;
}) {
  const config = STATUS_CONFIG[stage.status];
  const Icon = config.icon;

  return (
    <div className="flex items-center">
      <div className="flex flex-col items-center gap-1">
        <div
          className={`flex h-8 w-8 items-center justify-center rounded-full ring-2 ${config.bg} ${config.ring}`}
        >
          <Icon className={`h-4 w-4 text-white ${config.animate ? "animate-spin" : ""}`} />
        </div>
        <span
          className={`text-xs ${
            stage.status === "running"
              ? "font-medium text-blue-400"
              : stage.status === "completed"
                ? "text-emerald-400"
                : stage.status === "failed"
                  ? "text-red-400"
                  : stage.status === "blocked"
                    ? "text-amber-400"
                    : "text-slate-500"
          }`}
        >
          {STAGE_LABELS[stage.name]}
        </span>
        {stage.status === "pending" && isNextPending && pipelineId && (
          <div className="flex gap-1">
            <button
              type="button"
              onClick={() => executeStage(pipelineId)}
              className="flex items-center gap-0.5 rounded bg-blue-600 px-1.5 py-0.5 text-[10px] text-white hover:bg-blue-500"
            >
              <Play className="h-2.5 w-2.5" /> 执行
            </button>
            <button
              type="button"
              onClick={() => skipStage(pipelineId, stage.name)}
              className="flex items-center gap-0.5 rounded bg-slate-600 px-1.5 py-0.5 text-[10px] text-slate-300 hover:bg-slate-500"
            >
              <SkipForward className="h-2.5 w-2.5" /> 跳过
            </button>
          </div>
        )}
      </div>
      {!isLast && (
        <div
          className={`mx-1 h-0.5 w-8 ${
            stage.status === "completed" ? "bg-emerald-500" : "bg-slate-700"
          }`}
        />
      )}
    </div>
  );
}

export function StageProgress({
  stages,
  pipelineId,
}: {
  stages?: StageDetail[];
  pipelineId?: string | null;
}) {
  const { state } = useStore();
  const stageMap = new Map(stages?.map((s) => [s.name, s]));

  const orderedStages: StageDetail[] = STAGE_ORDER.map(
    (name) =>
      stageMap.get(name) ?? {
        name,
        status: "pending" as const,
        retry_count: 0,
        output_summary: null,
        gate_results: null,
        issue: null,
        started_at: null,
        completed_at: null,
      },
  );

  const firstPendingIdx = orderedStages.findIndex((s) => s.status === "pending");
  const runningStage = orderedStages.find((s) => s.status === "running");
  const pid = pipelineId ?? state.selectedPipelineId;

  return (
    <div>
      <div className="flex items-start justify-center gap-0 py-4">
        {orderedStages.map((stage, i) => (
          <StageNode
            key={stage.name}
            stage={stage}
            isLast={i === orderedStages.length - 1}
            pipelineId={pid}
            isNextPending={i === firstPendingIdx}
          />
        ))}
      </div>
      {runningStage && pid && (
        <div className="mt-2 rounded-lg bg-slate-800/60 p-3">
          <div className="mb-1 text-xs font-medium text-blue-400">
            {STAGE_LABELS[runningStage.name]} - Claude 输出
          </div>
          <StreamOutput pipelineId={pid} />
        </div>
      )}
    </div>
  );
}
