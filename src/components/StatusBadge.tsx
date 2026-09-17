import { Tag } from "antd";

import type { PipelineStatus } from "../lib/types";

const STATUS_CONFIG: Record<PipelineStatus, { color: string; label: string }> = {
  created: { color: "default", label: "已创建" },
  running: { color: "processing", label: "运行中" },
  waiting_merge: { color: "warning", label: "待合并" },
  blocked: { color: "error", label: "已阻塞" },
  completed: { color: "success", label: "已完成" },
  failed: { color: "error", label: "已失败" },
  cancelled: { color: "default", label: "已取消" },
};

export function StatusBadge({ status }: { status: PipelineStatus }) {
  const config = STATUS_CONFIG[status];
  return (
    <Tag color={config.color} style={{ margin: 0 }}>
      {config.label}
    </Tag>
  );
}
