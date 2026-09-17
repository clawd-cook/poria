import { LinkOutlined, WarningOutlined } from "@ant-design/icons";
import { Card, Descriptions, Divider, Empty, Spin, Space, Tag, Typography } from "antd";

import { usePipeline } from "../hooks/usePipeline";
import { useStore } from "../state/store";
import { EventStream } from "./EventStream";
import { GateResults } from "./GateResults";
import { HumanLoopCard } from "./HumanLoopCard";
import { StageProgress } from "./StageProgress";
import { StatusBadge } from "./StatusBadge";

const { Title, Text } = Typography;

export function PipelineDetail() {
  const { state } = useStore();
  const { detail, events, humanRequest } = usePipeline();

  if (!state.selectedPipelineId) {
    return (
      <div style={{ display: "flex", height: "100%", alignItems: "center", justifyContent: "center" }}>
        <Empty description="选择一个 Pipeline 查看详情" />
      </div>
    );
  }

  if (!detail) {
    return (
      <div style={{ display: "flex", height: "100%", alignItems: "center", justifyContent: "center" }}>
        <Spin tip="加载中..." />
      </div>
    );
  }

  return (
    <div style={{ height: "100%", overflowY: "auto", padding: 24 }}>
      <Space align="center" style={{ marginBottom: 8 }}>
        <Title level={4} style={{ margin: 0 }}>{detail.demand_name}</Title>
        <StatusBadge status={detail.status} />
        {detail.has_regressed && <Tag icon={<WarningOutlined />} color="warning">已回退</Tag>}
      </Space>

      <Descriptions size="small" column={4} style={{ marginBottom: 16 }}>
        <Descriptions.Item label="需求">{detail.demand_code}</Descriptions.Item>
        <Descriptions.Item label="操作人">{detail.operator || "-"}</Descriptions.Item>
        <Descriptions.Item label="行云链接">
          <a href={detail.raw_link} target="_blank" rel="noreferrer">
            <Space size={4}><LinkOutlined />查看</Space>
          </a>
        </Descriptions.Item>
      </Descriptions>

      <StageProgress stages={detail.stages} pipelineId={detail.id} />

      {humanRequest && humanRequest.pipelineId === detail.id && (
        <div style={{ margin: "16px 0" }}>
          <HumanLoopCard
            pipelineId={humanRequest.pipelineId}
            stage={humanRequest.stage}
            issueClass={humanRequest.issueClass}
            detail={humanRequest.detail}
          />
        </div>
      )}

      <Divider />

      <Title level={5}>事件流</Title>
      <EventStream events={events} />

      <GateResults stages={detail.stages} />

      <Divider />
      <Title level={5}>阶段详情</Title>
      <Space direction="vertical" style={{ width: "100%" }} size={8}>
        {detail.stages.map((stage) => (
          <Card key={stage.name} size="small">
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <Text strong>{stage.name}</Text>
              <Space size={12}>
                {stage.retry_count > 0 && <Text type="secondary">重试 {stage.retry_count} 次</Text>}
                {stage.started_at && (
                  <Text type="secondary">
                    {new Date(stage.started_at).toLocaleTimeString("zh-CN")}
                  </Text>
                )}
              </Space>
            </div>
            {stage.output_summary && (
              <Text type="secondary" style={{ fontSize: 12, display: "block", marginTop: 4 }}>
                {stage.output_summary}
              </Text>
            )}
            {stage.issue && (
              <Text type="danger" style={{ fontSize: 12, display: "block", marginTop: 4 }}>
                {stage.issue}
              </Text>
            )}
          </Card>
        ))}
      </Space>
    </div>
  );
}
