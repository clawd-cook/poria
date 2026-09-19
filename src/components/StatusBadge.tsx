import type { PipelineStatus } from "@/lib/types";

import { Badge } from "./ui/badge";

const STATUS_CONFIG: Record<
  PipelineStatus,
  {
    label: string;
    variant: "default" | "destructive" | "outline" | "secondary" | "success" | "warning";
  }
> = {
  blocked: { label: "已阻塞", variant: "destructive" },
  cancelled: { label: "已取消", variant: "secondary" },
  completed: { label: "已完成", variant: "success" },
  created: { label: "未开始", variant: "secondary" },
  failed: { label: "已失败", variant: "destructive" },
  running: { label: "运行中", variant: "default" },
  waiting_merge: { label: "待合并", variant: "warning" },
};

export function StatusBadge({ status }: { status: PipelineStatus }) {
  const config = STATUS_CONFIG[status];
  return <Badge variant={config.variant}>{config.label}</Badge>;
}
