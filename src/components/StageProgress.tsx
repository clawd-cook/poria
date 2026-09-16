import {
  Check,
  X,
  Loader2,
  AlertTriangle,
  Circle,
  Minus,
} from "lucide-react";
import type { StageDetail, StageStatus } from "../lib/types";
import { STAGE_ORDER, STAGE_LABELS } from "../lib/types";

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
}: {
  stage: StageDetail;
  isLast: boolean;
}) {
  const config = STATUS_CONFIG[stage.status];
  const Icon = config.icon;

  return (
    <div className="flex items-center">
      <div className="flex flex-col items-center gap-1">
        <div
          className={`flex h-8 w-8 items-center justify-center rounded-full ring-2 ${config.bg} ${config.ring}`}
        >
          <Icon
            className={`h-4 w-4 text-white ${config.animate ? "animate-spin" : ""}`}
          />
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

export function StageProgress({ stages }: { stages?: StageDetail[] }) {
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

  return (
    <div className="flex items-start justify-center gap-0 py-4">
      {orderedStages.map((stage, i) => (
        <StageNode
          key={stage.name}
          stage={stage}
          isLast={i === orderedStages.length - 1}
        />
      ))}
    </div>
  );
}
