import {
  CloseCircleOutlined,
  EditOutlined,
  ForwardOutlined,
  LoginOutlined,
  ReloadOutlined,
  WarningOutlined,
} from "@ant-design/icons";
import { Alert, App, Button, Descriptions, Flex, Modal, Space, Typography, theme } from "antd";
import { useState } from "react";

import { isAuthExpiredIssue, isAuthExpiredMessage } from "../lib/auth";
import { invokeErrorMessage } from "../lib/errors";
import { isP0UnansweredMessage, isRequirementAmbiguousIssue } from "../lib/prdReview";
import { humanLoopRespond, startLogin } from "../lib/tauri";
import type { PrdReviewStatus } from "../lib/types";
import { useStore } from "../state/store";
import { PrdReviewEditor } from "./PrdReviewEditor";

interface HumanLoopCardProps {
  demandCode: string;
  demandId: number;
  detail: string;
  issueClass: string;
  pipelineId: string;
  stage: string;
}

export function HumanLoopCard({
  demandCode,
  demandId,
  detail,
  issueClass,
  pipelineId,
  stage,
}: HumanLoopCardProps) {
  const { dispatch } = useStore();
  const { message } = App.useApp();
  const { token } = theme.useToken();
  const [loading, setLoading] = useState<string | null>(null);
  const [editorOpen, setEditorOpen] = useState(false);
  const authExpired = isAuthExpiredIssue(issueClass) || isAuthExpiredMessage(detail);
  const p0Blocked = isRequirementAmbiguousIssue(issueClass) || isP0UnansweredMessage(detail);

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

  async function handleRelogin() {
    setLoading("login");
    try {
      await startLogin();
      message.info("请在浏览器完成登录，成功后将从当前阶段继续");
    } catch (error) {
      message.error(invokeErrorMessage(error, "打开登录页失败"));
    } finally {
      setLoading(null);
    }
  }

  async function handleSaved(status: PrdReviewStatus | null) {
    if (!status?.p0_done) {
      return;
    }
    setEditorOpen(false);
    await handleAction("resume");
  }

  return (
    <>
      <Alert
        type="warning"
        showIcon
        icon={<WarningOutlined />}
        message={
          authExpired
            ? "SSO 已过期，流水线已挂起"
            : p0Blocked
              ? "P0 未答，无法进入设计"
              : "Pipeline 需要协助"
        }
        description={
          <Space direction="vertical" style={{ width: "100%" }}>
            <Descriptions column={1} size="small">
              <Descriptions.Item label="问题">{issueClass}</Descriptions.Item>
              <Descriptions.Item label="阶段">{stage}</Descriptions.Item>
            </Descriptions>
            <Typography.Text type="secondary" style={{ fontSize: token.fontSizeSM }}>
              {detail}
            </Typography.Text>
            <Flex gap={token.marginXS} wrap="wrap">
              {authExpired ? (
                <Button
                  disabled={loading !== null}
                  icon={<LoginOutlined />}
                  loading={loading === "login"}
                  onClick={() => void handleRelogin()}
                  type="primary"
                >
                  重新登录
                </Button>
              ) : null}
              {p0Blocked ? (
                <Button
                  disabled={loading !== null}
                  icon={<EditOutlined />}
                  onClick={() => setEditorOpen(true)}
                  type="primary"
                >
                  填写 P0 答案
                </Button>
              ) : null}
              <Button
                disabled={loading !== null}
                icon={<ReloadOutlined />}
                loading={loading === "resume"}
                onClick={() => void handleAction("resume")}
                type={authExpired || p0Blocked ? "default" : "primary"}
              >
                {authExpired ? "登录后继续" : p0Blocked ? "答完后继续" : "修复并重试"}
              </Button>
              <Button
                icon={<ForwardOutlined />}
                loading={loading === "skip"}
                disabled={loading !== null}
                onClick={() => void handleAction("skip")}
              >
                跳过
              </Button>
              <Button
                danger
                icon={<CloseCircleOutlined />}
                loading={loading === "cancel"}
                disabled={loading !== null}
                onClick={() => void handleAction("cancel")}
              >
                取消 Pipeline
              </Button>
            </Flex>
          </Space>
        }
      />
      <Modal
        destroyOnHidden
        footer={null}
        onCancel={() => setEditorOpen(false)}
        open={editorOpen}
        title="填写 PRD_REVIEW.md P0 答案"
        width={720}
      >
        <PrdReviewEditor demandCode={demandCode} demandId={demandId} onSaved={handleSaved} />
      </Modal>
    </>
  );
}
