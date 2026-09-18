import { FolderOpenOutlined } from "@ant-design/icons";
import { App, Button, Empty, Typography } from "antd";
import { useState } from "react";

import { usePipeline } from "../hooks/usePipeline";
import { invokeErrorMessage } from "../lib/errors";
import { openWorkspace } from "../lib/tauri";
import { useStore } from "../state/store";
import { PageFrame } from "./PageFrame";

const { Text } = Typography;

export function WorkspacePage() {
  const { state } = useStore();
  const { detail } = usePipeline();
  const { message } = App.useApp();
  const [opening, setOpening] = useState(false);
  const selectedId = state.selectedPipelineId;
  const workspacePath = detail?.workspace_path?.trim() || null;

  async function handleOpen() {
    if (!selectedId || !workspacePath) {
      return;
    }
    setOpening(true);
    try {
      await openWorkspace(selectedId);
    } catch (error) {
      message.error(invokeErrorMessage(error, "无法打开工作区"));
    } finally {
      setOpening(false);
    }
  }

  if (!selectedId) {
    return (
      <PageFrame title="工作区">
        <Empty description="请先从看板打开一条流水线" />
      </PageFrame>
    );
  }

  if (!detail) {
    return (
      <PageFrame title="工作区">
        <Empty description="正在读取工作区路径" />
      </PageFrame>
    );
  }

  return (
    <PageFrame
      description="Claude 阶段的 cwd 是流水线工作区根。文档是指向 projects 的软链；前后端 git worktree 在该目录下。"
      extra={
        workspacePath ? (
          <Button
            icon={<FolderOpenOutlined />}
            loading={opening}
            onClick={() => void handleOpen()}
            type="primary"
          >
            在 Finder 中打开
          </Button>
        ) : null
      }
      title="工作区"
    >
      {workspacePath ? (
        <Text code copyable>
          {workspacePath}
        </Text>
      ) : (
        <Empty description="请先完成初始化，生成工作区" />
      )}
    </PageFrame>
  );
}
