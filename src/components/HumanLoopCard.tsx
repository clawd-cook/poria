import {
  CloseCircleOutlined,
  ForwardOutlined,
  ReloadOutlined,
  WarningOutlined,
} from "@ant-design/icons";
import { Alert, App, Button, Descriptions, Flex, Space, Typography, theme } from "antd";
import { useState } from "react";

import { invokeErrorMessage } from "../lib/errors";
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
  const { message } = App.useApp();
  const { token } = theme.useToken();
  const [loading, setLoading] = useState<string | null>(null);

  async function handleAction(action: string) {
    setLoading(action);
    try {
      await humanLoopRespond(pipelineId, action);
      dispatch({ type: "humanRequestDismissed", pipelineId });
    } catch (error) {
      message.error(invokeErrorMessage(error, "处理协助请求失败"));
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
          <Typography.Text type="secondary" style={{ fontSize: token.fontSizeSM }}>
            {detail}
          </Typography.Text>
          <Flex gap={token.marginXS}>
            <Button
              disabled={loading !== null}
              icon={<ReloadOutlined />}
              loading={loading === "resume"}
              onClick={() => handleAction("resume")}
              type="primary"
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
