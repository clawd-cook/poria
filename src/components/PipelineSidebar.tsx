import { Filter } from "lucide-react";
import { useMemo } from "react";

import { pipelineHitlLane } from "@/lib/hitlLane";
import type { PipelineSummary } from "@/lib/types";
import { cn } from "@/lib/utils";
import { useStore } from "@/state/store";

import { StatusBadge } from "./StatusBadge";
import { Badge } from "./ui/badge";
import { Button } from "./ui/button";

const STATUS_GROUPS: {
  label: string;
  match: (pipeline: PipelineSummary) => boolean;
}[] = [
  { label: "运行中", match: (pipeline) => pipeline.status === "running" },
  { label: "待确认", match: (pipeline) => pipelineHitlLane(pipeline) === "confirm" },
  { label: "待验收", match: (pipeline) => pipelineHitlLane(pipeline) === "review" },
  { label: "已阻塞", match: (pipeline) => pipelineHitlLane(pipeline) === "blocked" },
  { label: "已完成", match: (pipeline) => pipeline.status === "completed" },
  { label: "已失败", match: (pipeline) => pipeline.status === "failed" },
  {
    label: "其他",
    match: (pipeline) => pipeline.status === "created" || pipeline.status === "cancelled",
  },
];

const FILTER_OPTIONS = [
  { label: "全部", value: "all" },
  { label: "运行中", value: "running" },
  { label: "待确认", value: "confirm" },
  { label: "待验收", value: "review" },
  { label: "已阻塞", value: "blocked" },
  { label: "已完成", value: "completed" },
  { label: "已失败", value: "failed" },
];

function timeAgo(dateStr: string): string {
  const diff = Date.now() - new Date(dateStr).getTime();
  const minutes = Math.floor(diff / 60000);
  if (minutes < 1) return "刚刚";
  if (minutes < 60) return `${minutes}分钟前`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}小时前`;
  return `${Math.floor(hours / 24)}天前`;
}

export function PipelineSidebar() {
  const { dispatch, state } = useStore();
  const { pipelines, selectedPipelineId, ui } = state;

  const filtered = useMemo(() => {
    if (!ui.filter) return pipelines;
    if (ui.filter === "confirm" || ui.filter === "review" || ui.filter === "blocked") {
      return pipelines.filter((pipeline) => pipelineHitlLane(pipeline) === ui.filter);
    }
    return pipelines.filter((p) => p.status === ui.filter);
  }, [pipelines, ui.filter]);

  const grouped = useMemo(() => {
    return STATUS_GROUPS.map((group) => ({
      ...group,
      items: filtered.filter((pipeline) => group.match(pipeline)),
    })).filter((g) => g.items.length > 0);
  }, [filtered]);

  return (
    <div className="flex h-full flex-col">
      <div className="border-border flex items-center gap-2 border-b px-3 py-2">
        <Filter aria-hidden className="size-3 opacity-45" />
        <div className="flex flex-wrap gap-1">
          {FILTER_OPTIONS.map((option) => {
            const active = (ui.filter ?? "all") === option.value;
            return (
              <Button
                aria-pressed={active}
                key={option.value}
                onClick={() =>
                  dispatch({
                    filter: option.value === "all" ? null : option.value,
                    type: "filterChanged",
                  })
                }
                size="sm"
                variant={active ? "default" : "ghost"}
              >
                {option.label}
              </Button>
            );
          })}
        </div>
      </div>

      <div className="flex-1 overflow-auto">
        {grouped.length === 0 ? (
          <p className="text-muted-foreground px-4 py-8 text-center text-sm">暂无 Pipeline</p>
        ) : null}
        {grouped.map((group) => (
          <div key={group.label}>
            <div className="bg-card sticky top-0 z-[1] px-4 py-1.5">
              <p className="text-muted-foreground text-xs">
                {group.label} ({group.items.length})
              </p>
            </div>
            <ul>
              {group.items.map((pipeline: PipelineSummary) => (
                <li key={pipeline.id}>
                  <button
                    className={cn(
                      "flex w-full cursor-pointer flex-col gap-1 px-4 py-2 text-left transition-colors duration-200 hover:bg-muted",
                      pipeline.id === selectedPipelineId ? "bg-primary/10" : undefined,
                    )}
                    onClick={() => dispatch({ id: pipeline.id, type: "pipelineSelected" })}
                    type="button"
                  >
                    <div className="flex items-start justify-between gap-2">
                      <span className="truncate text-[13px] font-medium">
                        {pipeline.demand_name}
                      </span>
                      <StatusBadge status={pipeline.status} />
                    </div>
                    <div className="flex items-center gap-2">
                      {pipeline.current_stage ? (
                        <Badge variant="outline">{pipeline.current_stage}</Badge>
                      ) : null}
                      <span className="text-muted-foreground text-[11px]">
                        {timeAgo(pipeline.created_at)}
                      </span>
                    </div>
                  </button>
                </li>
              ))}
            </ul>
          </div>
        ))}
      </div>
    </div>
  );
}
