import {
  CheckCircle2,
  Circle,
  CircleAlert,
  CircleMinus,
  CircleX,
  LoaderCircle,
  Play,
  SkipForward,
} from "lucide-react";
import { useState, type ReactNode } from "react";
import { toast } from "sonner";

import { invokeErrorMessage } from "@/lib/errors";
import { executeStage, skipStage } from "@/lib/tauri";
import type { StageDetail, StageStatus } from "@/lib/types";
import { STAGE_ORDER, stageLabel } from "@/lib/types";
import { cn } from "@/lib/utils";
import { useStore } from "@/state/store";

import { StreamOutput } from "./StreamOutput";
import { Button } from "./ui/button";
import { Card, CardContent } from "./ui/card";

function stageIcon(status: StageStatus): ReactNode {
  switch (status) {
    case "completed":
      return <CheckCircle2 aria-hidden className="text-success size-4" />;
    case "running":
      return <LoaderCircle aria-hidden className="text-primary size-4 animate-spin" />;
    case "failed":
      return <CircleX aria-hidden className="text-destructive size-4" />;
    case "blocked":
      return <CircleAlert aria-hidden className="text-warning size-4" />;
    case "skipped":
      return <CircleMinus aria-hidden className="text-muted-foreground size-4" />;
    default:
      return <Circle aria-hidden className="text-muted-foreground size-4" />;
  }
}

export function StageProgress({
  pipelineId,
  stages,
}: {
  pipelineId?: string | null;
  stages?: StageDetail[];
}) {
  const { state } = useStore();
  const [executing, setExecuting] = useState(false);
  const orderedStages: StageDetail[] =
    stages && stages.length > 0
      ? stages
      : STAGE_ORDER.map((name) => ({
          completed_at: null,
          gate_results: null,
          issue: null,
          name,
          output_summary: null,
          retry_count: 0,
          started_at: null,
          status: "pending" as const,
        }));

  const firstActionableIdx = orderedStages.findIndex(
    (s) => s.status === "pending" || s.status === "failed",
  );
  const runningStage = orderedStages.find((s) => s.status === "running");
  const pid = pipelineId ?? state.selectedPipelineId;
  const hasRunning = Boolean(runningStage);

  async function handleExecute() {
    if (!pid) {
      return;
    }
    setExecuting(true);
    try {
      await executeStage(pid);
    } catch (err) {
      toast.error(invokeErrorMessage(err, "阶段执行失败"));
    } finally {
      setExecuting(false);
    }
  }

  return (
    <div>
      <ol className="flex flex-wrap gap-3 py-4">
        {orderedStages.map((stage, i) => {
          const actionable =
            (stage.status === "pending" || stage.status === "failed") &&
            i === firstActionableIdx &&
            pid &&
            !hasRunning;
          return (
            <li className="flex min-w-36 flex-1 flex-col gap-2" key={stage.name}>
              <div className="flex items-center gap-2">
                {stageIcon(stage.status)}
                <span
                  className={cn(
                    "text-sm font-medium",
                    stage.status === "running" ? "text-primary" : "text-foreground",
                  )}
                >
                  {stageLabel(stage.name)}
                </span>
              </div>
              {actionable ? (
                <div className="flex gap-1">
                  <Button
                    disabled={executing}
                    loading={executing}
                    onClick={() => void handleExecute()}
                    size="sm"
                  >
                    <Play aria-hidden />
                    {stage.status === "failed" ? "重试" : "执行"}
                  </Button>
                  {stage.status === "pending" ? (
                    <Button
                      onClick={() => {
                        void skipStage(pid, stage.name).catch((err) => {
                          toast.error(invokeErrorMessage(err, "跳过阶段失败"));
                        });
                      }}
                      size="sm"
                      variant="outline"
                    >
                      <SkipForward aria-hidden />
                      跳过
                    </Button>
                  ) : null}
                </div>
              ) : null}
            </li>
          );
        })}
      </ol>
      {runningStage && pid ? (
        <Card className="mt-2">
          <CardContent className="p-4">
            {runningStage.name === "init" ? (
              <p className="text-muted-foreground text-xs">正在从 JoySpace 导出 PRD / 后端 TRD…</p>
            ) : (
              <>
                <p className="text-muted-foreground mb-1 text-xs">
                  {stageLabel(runningStage.name)} - Claude 输出
                </p>
                <StreamOutput pipelineId={pid} />
              </>
            )}
          </CardContent>
        </Card>
      ) : null}
    </div>
  );
}
