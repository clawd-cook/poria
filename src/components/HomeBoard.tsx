import { ArrowLeftOutlined } from "@ant-design/icons";
import { Button, Card, Flex, Tag, Typography, theme } from "antd";

import {
  STAGE_LABELS,
  type PipelineStatus,
  type PipelineSummary,
  type StageEnum,
} from "../lib/types";
import { useStore } from "../state/store";
import { PipelineDetail } from "./PipelineDetail";
import { StatusBadge } from "./StatusBadge";

const { Text } = Typography;

const KANBAN_COLUMNS: { key: string; label: string; statuses: PipelineStatus[] }[] = [
  { key: "running", label: "运行中", statuses: ["running"] },
  { key: "blocked", label: "阻塞", statuses: ["blocked"] },
  { key: "waiting_merge", label: "待合并", statuses: ["waiting_merge"] },
  { key: "completed", label: "完成", statuses: ["completed"] },
  { key: "failed", label: "失败", statuses: ["failed"] },
  { key: "other", label: "其他", statuses: ["created", "cancelled"] },
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

function stageLabel(stage: string | null): string | null {
  if (!stage) return null;
  if (stage in STAGE_LABELS) {
    return STAGE_LABELS[stage as StageEnum];
  }
  return stage;
}

export function HomeBoard() {
  const { state, dispatch } = useStore();
  const { token } = theme.useToken();

  if (state.selectedPipelineId) {
    return (
      <Flex vertical style={{ height: "100%" }}>
        <div style={{ padding: "8px 16px 0" }}>
          <Button
            type="text"
            icon={<ArrowLeftOutlined />}
            onClick={() => dispatch({ type: "pipelineSelected", id: null })}
          >
            返回看板
          </Button>
        </div>
        <div style={{ flex: 1, minHeight: 0, overflow: "auto" }}>
          <PipelineDetail />
        </div>
      </Flex>
    );
  }

  const columns = KANBAN_COLUMNS.map((column) => ({
    ...column,
    items: state.pipelines.filter((pipeline) => column.statuses.includes(pipeline.status)),
  }));

  return (
    <Flex gap={12} style={{ height: "100%", overflowX: "auto", padding: 16 }}>
      {columns.map((column) => (
        <Flex
          key={column.key}
          vertical
          gap={8}
          style={{
            background: token.colorFillAlter,
            borderRadius: token.borderRadiusLG,
            flex: "0 0 260px",
            minWidth: 260,
            padding: 12,
          }}
        >
          <Text strong>
            {column.label} ({column.items.length})
          </Text>
          <Flex vertical gap={8} style={{ flex: 1, minHeight: 0, overflowY: "auto" }}>
            {column.items.map((pipeline) => (
              <KanbanCard
                key={pipeline.id}
                onSelect={() => dispatch({ type: "pipelineSelected", id: pipeline.id })}
                pipeline={pipeline}
              />
            ))}
          </Flex>
        </Flex>
      ))}
    </Flex>
  );
}

function KanbanCard({ pipeline, onSelect }: { pipeline: PipelineSummary; onSelect: () => void }) {
  const stage = stageLabel(pipeline.current_stage);

  return (
    <Card hoverable size="small" onClick={onSelect} styles={{ body: { padding: 12 } }}>
      <Flex align="flex-start" justify="space-between" gap={8}>
        <Text ellipsis strong style={{ flex: 1 }}>
          {pipeline.demand_name}
        </Text>
        <StatusBadge status={pipeline.status} />
      </Flex>
      <Flex align="center" gap={8} style={{ marginTop: 8 }}>
        {stage ? <Tag style={{ fontSize: 11, margin: 0 }}>{stage}</Tag> : null}
        <Text type="secondary" style={{ fontSize: 11 }}>
          {pipeline.demand_code} · {timeAgo(pipeline.updated_at)}
        </Text>
      </Flex>
    </Card>
  );
}
