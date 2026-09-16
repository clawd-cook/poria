import { Filter } from "lucide-react";
import { useMemo } from "react";

import type { PipelineStatus, PipelineSummary } from "../lib/types";
import { useStore } from "../state/store";
import { StatusBadge } from "./StatusBadge";

const STATUS_GROUPS: { label: string; statuses: PipelineStatus[] }[] = [
  { label: "运行中", statuses: ["running"] },
  { label: "待合并", statuses: ["waiting_merge"] },
  { label: "已阻塞", statuses: ["blocked"] },
  { label: "已完成", statuses: ["completed"] },
  { label: "已失败", statuses: ["failed"] },
  { label: "其他", statuses: ["created", "cancelled"] },
];

const FILTER_TABS: { label: string; value: string | null }[] = [
  { label: "全部", value: null },
  { label: "运行中", value: "running" },
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

function PipelineItem({
  pipeline,
  selected,
  onSelect,
}: {
  pipeline: PipelineSummary;
  selected: boolean;
  onSelect: () => void;
}) {
  return (
    <button
      onClick={onSelect}
      className={`w-full border-b border-slate-700 px-4 py-3 text-left transition-colors hover:bg-slate-700/50 ${
        selected ? "bg-slate-700" : ""
      }`}
    >
      <div className="flex items-start justify-between gap-2">
        <span className="truncate text-sm font-medium text-slate-200">{pipeline.demand_name}</span>
        <StatusBadge status={pipeline.status} />
      </div>
      <div className="mt-1 flex items-center gap-2 text-xs text-slate-500">
        {pipeline.current_stage && (
          <span className="rounded bg-slate-700 px-1.5 py-0.5">{pipeline.current_stage}</span>
        )}
        <span>{timeAgo(pipeline.created_at)}</span>
      </div>
    </button>
  );
}

export function PipelineSidebar() {
  const { state, dispatch } = useStore();
  const { pipelines, selectedPipelineId, ui } = state;

  const filtered = useMemo(() => {
    if (!ui.filter) return pipelines;
    return pipelines.filter((p) => p.status === ui.filter);
  }, [pipelines, ui.filter]);

  const grouped = useMemo(() => {
    return STATUS_GROUPS.map((group) => ({
      ...group,
      items: filtered.filter((p) => group.statuses.includes(p.status)),
    })).filter((g) => g.items.length > 0);
  }, [filtered]);

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center gap-1 border-b border-slate-700 px-2 py-2">
        <Filter className="h-3.5 w-3.5 text-slate-500" />
        {FILTER_TABS.map((tab) => (
          <button
            key={tab.label}
            onClick={() => dispatch({ type: "filterChanged", filter: tab.value })}
            className={`rounded px-2 py-1 text-xs transition-colors ${
              ui.filter === tab.value
                ? "bg-slate-600 text-slate-200"
                : "text-slate-400 hover:text-slate-300"
            }`}
          >
            {tab.label}
          </button>
        ))}
      </div>

      <div className="flex-1 overflow-y-auto">
        {grouped.length === 0 && (
          <div className="px-4 py-8 text-center text-sm text-slate-500">暂无 Pipeline</div>
        )}
        {grouped.map((group) => (
          <div key={group.label}>
            <div className="sticky top-0 bg-slate-800 px-4 py-1.5 text-xs font-medium text-slate-500">
              {group.label} ({group.items.length})
            </div>
            {group.items.map((p) => (
              <PipelineItem
                key={p.id}
                pipeline={p}
                selected={p.id === selectedPipelineId}
                onSelect={() => dispatch({ type: "pipelineSelected", id: p.id })}
              />
            ))}
          </div>
        ))}
      </div>
    </div>
  );
}
