import type { PipelineStatus } from "../lib/types";

const STATUS_STYLES: Record<PipelineStatus, string> = {
  created: "bg-slate-600 text-slate-200",
  running: "bg-blue-600 text-blue-100",
  waiting_merge: "bg-amber-600 text-amber-100",
  blocked: "bg-red-600 text-red-100",
  completed: "bg-emerald-600 text-emerald-100",
  failed: "bg-red-700 text-red-100",
  cancelled: "bg-slate-600 text-slate-300",
};

const STATUS_LABELS: Record<PipelineStatus, string> = {
  created: "已创建",
  running: "运行中",
  waiting_merge: "待合并",
  blocked: "已阻塞",
  completed: "已完成",
  failed: "已失败",
  cancelled: "已取消",
};

export function StatusBadge({ status }: { status: PipelineStatus }) {
  return (
    <span
      className={`inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium ${STATUS_STYLES[status]}`}
    >
      {STATUS_LABELS[status]}
    </span>
  );
}
