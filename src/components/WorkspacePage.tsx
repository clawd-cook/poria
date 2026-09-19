import { FolderOpen } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";

import { usePipeline } from "@/hooks/usePipeline";
import { invokeErrorMessage } from "@/lib/errors";
import { openWorkspace } from "@/lib/tauri";
import { useStore } from "@/state/store";

import { EmptyState } from "./EmptyState";
import { PageFrame } from "./PageFrame";
import { Button } from "./ui/button";

export function WorkspacePage() {
  const { state } = useStore();
  const { detail } = usePipeline();
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
      toast.error(invokeErrorMessage(error, "无法打开工作区"));
    } finally {
      setOpening(false);
    }
  }

  async function copyPath() {
    if (!workspacePath) {
      return;
    }
    try {
      await navigator.clipboard.writeText(workspacePath);
      toast.success("已复制路径");
    } catch {
      toast.error("复制失败");
    }
  }

  if (!selectedId) {
    return (
      <PageFrame title="工作区">
        <EmptyState description="请先从看板打开一条流水线" />
      </PageFrame>
    );
  }

  if (!detail) {
    return (
      <PageFrame title="工作区">
        <EmptyState description="正在读取工作区路径" />
      </PageFrame>
    );
  }

  return (
    <PageFrame
      description="Claude 阶段的 cwd 是流水线工作区根。文档是指向 projects 的软链；前后端 git worktree 在该目录下。"
      extra={
        workspacePath ? (
          <Button loading={opening} onClick={() => void handleOpen()}>
            <FolderOpen aria-hidden />
            在 Finder 中打开
          </Button>
        ) : null
      }
      title="工作区"
    >
      {workspacePath ? (
        <button
          className="bg-muted hover:bg-muted/80 cursor-pointer rounded-md px-2 py-1 font-mono text-sm"
          onClick={() => void copyPath()}
          type="button"
        >
          {workspacePath}
        </button>
      ) : (
        <EmptyState description="请先完成初始化，生成工作区" />
      )}
    </PageFrame>
  );
}
