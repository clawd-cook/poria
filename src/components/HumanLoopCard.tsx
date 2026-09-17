import {
  CloseCircleOutlined,
  ForwardOutlined,
  ReloadOutlined,
  WarningOutlined,
} from "@ant-design/icons";
import { Alert, Button, Descriptions, Flex, Space, Typography } from "antd";
import { useState } from "react";

import { humanLoopRespond } from "../lib/tauri";
import { useStore } from "../state/store";

interface HumanLoopCardProps {
  pipelineId: string;
  stage: string;
  issueClass: string;
  detail: string;
}

export function HumanLoopCard({ pipelineId, stage, issueClass, detail }: HumanLoopCardProps) {
  const { dispatch } = useStore();
  const [loading, setLoading] = useState<string | null>(null);

  async function handleAction(action: string) {
    setLoading(action);
    try {
      await humanLoopRespond(pipelineId, action);
      dispatch({ type: "humanRequestDismissed", pipelineId });
    } catch {
      /* noop */
    } finally {
      setLoading(null);
    }
  }

  return (
    <Alert
      type="warning"
      showIcon
      icon={<WarningOutlined />}
      message="Pipeline 需要协助"
      description={
        <Space direction="vertical" style={{ width: "100%" }}>
          <Descriptions column={1} size="small">
            <Descriptions.Item label="问题">{issueClass}</Descriptions.Item>
            <Descriptions.Item label="阶段">{stage}</Descriptions.Item>
          </Descriptions>
          <Typography.Text type="secondary" style={{ fontSize: 12 }}>
            {detail}
          </Typography.Text>
          <Flex gap={8}>
            <Button
              type="primary"
              icon={<ReloadOutlined />}
              loading={loading === "resume"}
              disabled={loading !== null}
              onClick={() => handleAction("resume")}
              style={{ backgroundColor: "#52c41a", borderColor: "#52c41a" }}
            >
              修复并重试
            </Button>
            <Button
              icon={<ForwardOutlined />}
              loading={loading === "skip"}
              disabled={loading !== null}
              onClick={() => handleAction("skip")}
            >
              跳过
            </Button>
            <Button
              danger
              icon={<CloseCircleOutlined />}
              loading={loading === "cancel"}
              disabled={loading !== null}
              onClick={() => handleAction("cancel")}
            >
              取消 Pipeline
            </Button>
          </Flex>
        </Space>
      }
    />
  );
}
