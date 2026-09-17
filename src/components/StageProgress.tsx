import {
  CheckCircleOutlined,
  CloseCircleOutlined,
  ExclamationCircleOutlined,
  ForwardOutlined,
  LoadingOutlined,
  MinusCircleOutlined,
  PlayCircleOutlined,
} from "@ant-design/icons";
import { App, Button, Card, Space, Steps, Typography } from "antd";
import { useState } from "react";

import { invokeErrorMessage } from "../lib/errors";
import { executeStage, skipStage } from "../lib/tauri";
import type { StageDetail, StageStatus } from "../lib/types";
import { STAGE_LABELS, STAGE_ORDER } from "../lib/types";
import { useStore } from "../state/store";
import { StreamOutput } from "./StreamOutput";

const { Text } = Typography;

type StepsStatus = "finish" | "process" | "wait" | "error";

function mapStatus(status: StageStatus): StepsStatus {
  switch (status) {
    case "completed":
      return "finish";
    case "running":
      return "process";
    case "failed":
      return "error";
    case "blocked":
      return "error";
    case "skipped":
      return "finish";
    default:
      return "wait";
  }
}

function stageIcon(status: StageStatus) {
  switch (status) {
    case "completed":
      return <CheckCircleOutlined />;
    case "running":
      return <LoadingOutlined spin />;
    case "failed":
      return <CloseCircleOutlined />;
    case "blocked":
      return <ExclamationCircleOutlined />;
    case "skipped":
      return <MinusCircleOutlined />;
    default:
      return undefined;
  }
}

export function StageProgress({
  stages,
  pipelineId,
}: {
  stages?: StageDetail[];
  pipelineId?: string | null;
}) {
  const { state } = useStore();
  const { message } = App.useApp();
  const [executing, setExecuting] = useState(false);
  const stageMap = new Map(stages?.map((s) => [s.name, s]));

  const orderedStages: StageDetail[] = STAGE_ORDER.map(
    (name) =>
      stageMap.get(name) ?? {
        name,
        status: "pending" as const,
        retry_count: 0,
        output_summary: null,
        gate_results: null,
        issue: null,
        started_at: null,
        completed_at: null,
      },
  );

  const firstActionableIdx = orderedStages.findIndex(
    (s) => s.status === "pending" || s.status === "failed",
  );
  const runningStage = orderedStages.find((s) => s.status === "running");
  const pid = pipelineId ?? state.selectedPipelineId;
  const hasRunning = Boolean(runningStage);

  async function handleExecute() {
    if (!pid) {
      return;
    }
    setExecuting(true);
    try {
      await executeStage(pid);
    } catch (err) {
      message.error(invokeErrorMessage(err, "阶段执行失败"));
    } finally {
      setExecuting(false);
    }
  }

  const currentIdx = orderedStages.findIndex(
    (s) => s.status === "running" || s.status === "pending" || s.status === "failed",
  );

  const items = orderedStages.map((stage, i) => ({
    title: STAGE_LABELS[stage.name],
    status: mapStatus(stage.status) as StepsStatus,
    icon: stageIcon(stage.status),
    description:
      stage.status === "pending" || stage.status === "failed" ? (
        i === firstActionableIdx && pid && !hasRunning ? (
          <Space size={4} style={{ marginTop: 4 }}>
            <Button
              disabled={executing}
              icon={<PlayCircleOutlined />}
              loading={executing}
              onClick={() => void handleExecute()}
              size="small"
              type="primary"
            >
              {stage.status === "failed" ? "重试" : "执行"}
            </Button>
            {stage.status === "pending" ? (
              <Button
                icon={<ForwardOutlined />}
                onClick={() => void skipStage(pid, stage.name)}
                size="small"
              >
                跳过
              </Button>
            ) : null}
          </Space>
        ) : undefined
      ) : undefined,
  }));

  return (
    <div>
      <Steps
        current={currentIdx >= 0 ? currentIdx : orderedStages.length}
        size="small"
        items={items}
        style={{ padding: "16px 0" }}
      />
      {runningStage && pid && (
        <Card size="small" style={{ marginTop: 8 }}>
          {runningStage.name === "init" ? (
            <Text type="secondary" style={{ fontSize: 12 }}>
              正在从 JoySpace 导出 PRD / 后端 TRD…
            </Text>
          ) : (
            <>
              <Text type="secondary" style={{ fontSize: 12, display: "block", marginBottom: 4 }}>
                {STAGE_LABELS[runningStage.name]} - Claude 输出
              </Text>
              <StreamOutput pipelineId={pid} />
            </>
          )}
        </Card>
      )}
    </div>
  );
}
