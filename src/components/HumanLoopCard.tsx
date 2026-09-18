import {
  CheckOutlined,
  CloseCircleOutlined,
  EditOutlined,
  FileTextOutlined,
  ForwardOutlined,
  LoginOutlined,
  ReloadOutlined,
  WarningOutlined,
} from "@ant-design/icons";
import {
  Alert,
  App,
  Button,
  Descriptions,
  Flex,
  Modal,
  Space,
  Spin,
  Typography,
  theme,
} from "antd";
import { useState } from "react";

import { isAuthExpiredIssue, isAuthExpiredMessage } from "../lib/auth";
import { invokeErrorMessage } from "../lib/errors";
import { issueClassLabel, issueClassTitle } from "../lib/issueClass";
import { isOutputGuardBlock } from "../lib/outputGuard";
import { isP0UnansweredMessage, isRequirementAmbiguousIssue } from "../lib/prdReview";
import { isQualityGateBlock, qualityGateTitle } from "../lib/qualityGates";
import { confirmTrd, humanLoopRespond, readDemandProjectFile, startLogin } from "../lib/tauri";
import { isTrdUnconfirmedIssue, isTrdUnconfirmedMessage } from "../lib/trd";
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
  const [trdOpen, setTrdOpen] = useState(false);
  const [trdContent, setTrdContent] = useState("");
  const [trdLoading, setTrdLoading] = useState(false);
  const [Markdown, setMarkdown] = useState<
    typeof import("@ant-design/x-markdown").XMarkdown | null
  >(null);
  const authExpired = isAuthExpiredIssue(issueClass) || isAuthExpiredMessage(detail);
  const p0Blocked = isRequirementAmbiguousIssue(issueClass) || isP0UnansweredMessage(detail);
  const trdBlocked = isTrdUnconfirmedIssue(issueClass) || isTrdUnconfirmedMessage(detail);
  const outputGuardBlocked = isOutputGuardBlock(issueClass, detail);
  const qualityGateBlocked =
    !authExpired &&
    !p0Blocked &&
    !trdBlocked &&
    !outputGuardBlocked &&
    isQualityGateBlock(issueClass, detail);
  const routedTitle = issueClassTitle(issueClass, detail);

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

  async function handleConfirmTrd(skipped: boolean) {
    setLoading(skipped ? "skip-confirm" : "confirm");
    try {
      await confirmTrd(pipelineId, skipped);
      dispatch({ type: "humanRequestDismissed", pipelineId });
    } catch (error) {
      message.error(invokeErrorMessage(error, "确认 TRD 失败"));
    } finally {
      setLoading(null);
    }
  }

  async function handleOpenTrd() {
    setTrdOpen(true);
    setTrdLoading(true);
    if (!Markdown) {
      const mod = await import("@ant-design/x-markdown");
      setMarkdown(() => mod.XMarkdown);
    }
    try {
      const file = await readDemandProjectFile({
        demandCode,
        demandId,
        fileName: "TRD.md",
      });
      setTrdContent(file.content);
    } catch (error) {
      message.error(invokeErrorMessage(error, "读取 TRD.md 失败"));
      setTrdContent("");
    } finally {
      setTrdLoading(false);
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
              : trdBlocked
                ? "请确认前端 TRD 后再进入开发"
                : outputGuardBlocked
                  ? "代码超出 TRD 允许范围，无法进入 CR"
                  : qualityGateBlocked
                    ? qualityGateTitle(issueClass, detail)
                    : (routedTitle ?? "Pipeline 需要协助")
        }
        description={
          <Space direction="vertical" style={{ width: "100%" }}>
            <Descriptions column={1} size="small">
              <Descriptions.Item label="问题">
                {issueClassLabel(issueClass)}
                <Typography.Text type="secondary" style={{ marginLeft: token.marginXS }}>
                  ({issueClass})
                </Typography.Text>
              </Descriptions.Item>
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
              {trdBlocked ? (
                <>
                  <Button
                    disabled={loading !== null}
                    icon={<FileTextOutlined />}
                    onClick={() => void handleOpenTrd()}
                  >
                    查看 TRD.md
                  </Button>
                  <Button
                    disabled={loading !== null}
                    icon={<CheckOutlined />}
                    loading={loading === "confirm"}
                    onClick={() => void handleConfirmTrd(false)}
                    type="primary"
                  >
                    确认 TRD
                  </Button>
                </>
              ) : null}
              <Button
                disabled={loading !== null}
                icon={<ReloadOutlined />}
                loading={loading === "resume"}
                onClick={() => void handleAction("resume")}
                type={
                  authExpired || p0Blocked || trdBlocked || outputGuardBlocked || qualityGateBlocked
                    ? "default"
                    : "primary"
                }
              >
                {authExpired
                  ? "登录后继续"
                  : p0Blocked
                    ? "答完后继续"
                    : trdBlocked
                      ? "确认后继续"
                      : outputGuardBlocked
                        ? "收回越界改动后继续"
                        : qualityGateBlocked
                          ? "补齐报告后继续"
                          : "修复并重试"}
              </Button>
              <Button
                icon={<ForwardOutlined />}
                loading={loading === "skip" || loading === "skip-confirm"}
                disabled={loading !== null}
                onClick={() => void (trdBlocked ? handleConfirmTrd(true) : handleAction("skip"))}
              >
                {trdBlocked ? "跳过确认" : "跳过"}
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
      <Modal
        destroyOnHidden
        footer={null}
        onCancel={() => setTrdOpen(false)}
        open={trdOpen}
        title="前端 TRD.md"
        width={720}
      >
        {trdLoading ? (
          <div style={{ padding: 48, textAlign: "center" }}>
            <Spin />
          </div>
        ) : Markdown ? (
          <Markdown content={trdContent} openLinksInNewTab />
        ) : (
          <Typography.Paragraph>{trdContent}</Typography.Paragraph>
        )}
      </Modal>
    </>
  );
}
