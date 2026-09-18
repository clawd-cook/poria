import {
  CheckCircleOutlined,
  CloseCircleOutlined,
  ExclamationCircleOutlined,
  InfoCircleOutlined,
  PlayCircleOutlined,
} from "@ant-design/icons";
import { Empty, Tag, Timeline, theme } from "antd";
import { useEffect, useRef } from "react";

import type { PipelineEvent } from "../lib/types";

const KIND_CONFIG: Record<string, { color: string; icon: React.ReactNode }> = {
  stage_started: { color: "processing", icon: <PlayCircleOutlined /> },
  stage_completed: { color: "success", icon: <CheckCircleOutlined /> },
  stage_failed: { color: "error", icon: <CloseCircleOutlined /> },
  gate_passed: { color: "success", icon: <CheckCircleOutlined /> },
  gate_failed: { color: "error", icon: <CloseCircleOutlined /> },
  human_loop: { color: "warning", icon: <ExclamationCircleOutlined /> },
};

function formatTime(dateStr: string): string {
  try {
    return new Date(dateStr).toLocaleTimeString("zh-CN", {
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });
  } catch {
    return dateStr;
  }
}

export function EventStream({ events }: { events: PipelineEvent[] }) {
  const endRef = useRef<HTMLDivElement>(null);
  const { token } = theme.useToken();

  useEffect(() => {
    endRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [events.length]);

  if (events.length === 0) {
    return <Empty description="暂无事件" image={Empty.PRESENTED_IMAGE_SIMPLE} />;
  }

  const items = events.map((event) => {
    const cfg = KIND_CONFIG[event.kind] ?? { color: "default", icon: <InfoCircleOutlined /> };
    return {
      dot: cfg.icon,
      color: cfg.color as string,
      children: (
        <div>
          <Tag color={cfg.color as string} style={{ marginRight: token.marginXS }}>
            {event.kind}
          </Tag>
          <span
            style={{
              color: token.colorTextTertiary,
              fontSize: token.fontSizeSM,
              marginRight: token.marginXS,
            }}
          >
            {formatTime(event.created_at)}
          </span>
          <span style={{ fontSize: token.fontSize }}>{event.payload}</span>
        </div>
      ),
    };
  });

  return (
    <div style={{ maxHeight: 256, overflowY: "auto" }}>
      <Timeline items={items} />
      <div ref={endRef} />
    </div>
  );
}
