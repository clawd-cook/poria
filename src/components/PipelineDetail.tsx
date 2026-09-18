import { LinkOutlined, WarningOutlined } from "@ant-design/icons";
import {
  Card,
  Descriptions,
  Divider,
  Empty,
  Flex,
  Space,
  Spin,
  Tag,
  Typography,
  theme,
} from "antd";

import { usePipeline } from "../hooks/usePipeline";
import { useStore } from "../state/store";
import { EventStream } from "./EventStream";
import { GateResults } from "./GateResults";
import { HumanLoopCard } from "./HumanLoopCard";
import { PageFrame, PageSectionTitle } from "./PageFrame";
import { StageProgress } from "./StageProgress";
import { StatusBadge } from "./StatusBadge";

const { Link, Text } = Typography;

export function PipelineDetail() {
  const { state } = useStore();
  const { token } = theme.useToken();
  const { detail, events, humanRequest } = usePipeline();

  if (!state.selectedPipelineId) {
    return (
      <Flex align="center" justify="center" style={{ height: "100%" }}>
        <Empty description="选择一个 Pipeline 查看详情" />
      </Flex>
    );
  }

  if (!detail) {
    return (
      <Flex align="center" justify="center" style={{ height: "100%" }}>
        <Spin tip="加载中..." />
      </Flex>
    );
  }

  return (
    <PageFrame
      extra={
        <Flex align="center" gap={token.marginSM}>
          <StatusBadge status={detail.status} />
          {detail.has_regressed ? (
            <Tag color="warning" icon={<WarningOutlined />}>
              已回退
            </Tag>
          ) : null}
        </Flex>
      }
      title={detail.demand_name}
    >
      <Descriptions column={4} size="small" style={{ marginBottom: token.margin }}>
        <Descriptions.Item label="需求">{detail.demand_code}</Descriptions.Item>
        <Descriptions.Item label="操作人">{detail.operator || "-"}</Descriptions.Item>
        <Descriptions.Item label="行云链接">
          <Link href={detail.raw_link} rel="noreferrer" target="_blank">
            <Space size={token.marginXXS}>
              <LinkOutlined />
              查看
            </Space>
          </Link>
        </Descriptions.Item>
      </Descriptions>

      <StageProgress pipelineId={detail.id} stages={detail.stages} />

      {humanRequest && humanRequest.pipelineId === detail.id ? (
        <div style={{ margin: `${token.margin}px 0` }}>
          <HumanLoopCard
            detail={humanRequest.detail}
            issueClass={humanRequest.issueClass}
            pipelineId={humanRequest.pipelineId}
            stage={humanRequest.stage}
          />
        </div>
      ) : null}

      <Divider />

      <PageSectionTitle>事件流</PageSectionTitle>
      <EventStream events={events} />

      <GateResults stages={detail.stages} />

      <Divider />
      <PageSectionTitle>阶段详情</PageSectionTitle>
      <Space direction="vertical" size={token.marginXS} style={{ width: "100%" }}>
        {detail.stages.map((stage) => (
          <Card key={stage.name} size="small">
            <Flex align="center" justify="space-between">
              <Text strong>{stage.name}</Text>
              <Space size={token.marginSM}>
                {stage.retry_count > 0 ? (
                  <Text type="secondary">重试 {stage.retry_count} 次</Text>
                ) : null}
                {stage.started_at ? (
                  <Text type="secondary">
                    {new Date(stage.started_at).toLocaleTimeString("zh-CN")}
                  </Text>
                ) : null}
              </Space>
            </Flex>
            {stage.output_summary ? (
              <Text
                type="secondary"
                style={{ display: "block", fontSize: token.fontSizeSM, marginTop: token.marginXXS }}
              >
                {stage.output_summary}
              </Text>
            ) : null}
            {stage.issue ? (
              <Text
                type="danger"
                style={{ display: "block", fontSize: token.fontSizeSM, marginTop: token.marginXXS }}
              >
                {stage.issue}
              </Text>
            ) : null}
          </Card>
        ))}
      </Space>
    </PageFrame>
  );
}
