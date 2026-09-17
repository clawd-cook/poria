import { FilterOutlined } from "@ant-design/icons";
import { List, Segmented, Tag, Typography } from "antd";
import { useMemo } from "react";

import type { PipelineStatus, PipelineSummary } from "../lib/types";
import { useStore } from "../state/store";
import { StatusBadge } from "./StatusBadge";

const { Text } = Typography;

const STATUS_GROUPS: { label: string; statuses: PipelineStatus[] }[] = [
  { label: "运行中", statuses: ["running"] },
  { label: "待合并", statuses: ["waiting_merge"] },
  { label: "已阻塞", statuses: ["blocked"] },
  { label: "已完成", statuses: ["completed"] },
  { label: "已失败", statuses: ["failed"] },
  { label: "其他", statuses: ["created", "cancelled"] },
];

const FILTER_OPTIONS = [
  { label: "全部", value: "all" },
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
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div
        style={{
          padding: "8px 12px",
          borderBottom: "1px solid rgba(255,255,255,0.08)",
          display: "flex",
          alignItems: "center",
          gap: 8,
        }}
      >
        <FilterOutlined style={{ fontSize: 12, opacity: 0.45 }} />
        <Segmented
          size="small"
          value={ui.filter ?? "all"}
          onChange={(val) =>
            dispatch({ type: "filterChanged", filter: val === "all" ? null : (val as string) })
          }
          options={FILTER_OPTIONS}
        />
      </div>

      <div style={{ flex: 1, overflow: "auto" }}>
        {grouped.length === 0 && (
          <div style={{ padding: "32px 16px", textAlign: "center" }}>
            <Text type="secondary">暂无 Pipeline</Text>
          </div>
        )}
        {grouped.map((group) => (
          <div key={group.label}>
            <div
              style={{
                padding: "6px 16px",
                fontSize: 12,
                position: "sticky",
                top: 0,
                zIndex: 1,
                background: "inherit",
              }}
            >
              <Text type="secondary" style={{ fontSize: 12 }}>
                {group.label} ({group.items.length})
              </Text>
            </div>
            <List
              dataSource={group.items}
              split
              size="small"
              renderItem={(pipeline: PipelineSummary) => (
                <List.Item
                  onClick={() => dispatch({ type: "pipelineSelected", id: pipeline.id })}
                  style={{
                    padding: "8px 16px",
                    cursor: "pointer",
                    background:
                      pipeline.id === selectedPipelineId
                        ? "rgba(255,255,255,0.08)"
                        : undefined,
                  }}
                >
                  <div style={{ width: "100%" }}>
                    <div
                      style={{
                        display: "flex",
                        justifyContent: "space-between",
                        alignItems: "flex-start",
                        gap: 8,
                      }}
                    >
                      <Text
                        ellipsis
                        style={{ fontSize: 13, fontWeight: 500 }}
                      >
                        {pipeline.demand_name}
                      </Text>
                      <StatusBadge status={pipeline.status} />
                    </div>
                    <div
                      style={{
                        marginTop: 4,
                        display: "flex",
                        alignItems: "center",
                        gap: 8,
                      }}
                    >
                      {pipeline.current_stage && (
                        <Tag style={{ fontSize: 11, margin: 0 }}>
                          {pipeline.current_stage}
                        </Tag>
                      )}
                      <Text type="secondary" style={{ fontSize: 11 }}>
                        {timeAgo(pipeline.created_at)}
                      </Text>
                    </div>
                  </div>
                </List.Item>
              )}
            />
          </div>
        ))}
      </div>
    </div>
  );
}
