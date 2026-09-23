import { ArrowLeft, FolderOpen } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";

import { usePipeline } from "@/hooks/usePipeline";
import { invokeErrorMessage } from "@/lib/errors";
import { derivePipelineViewModel } from "@/lib/pipelineViewModel";
import { openWorkspace } from "@/lib/tauri";
import type { WorkbenchTab } from "@/lib/types";
import { useStore } from "@/state/store";

import { DemandProjectBrowser } from "./DemandProjectBrowser";
import { EmptyState } from "./EmptyState";
import { EventStream } from "./EventStream";
import { GateResults } from "./GateResults";
import { HumanLoopCard } from "./HumanLoopCard";
import { Spinner } from "./Spinner";
import { StageProgress } from "./StageProgress";
import { StatusBadge } from "./StatusBadge";
import { Badge } from "./ui/badge";
import { Button } from "./ui/button";
import { Card, CardContent } from "./ui/card";
import { Separator } from "./ui/separator";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./ui/tabs";

function WorkbenchTrajectory() {
  const { detail, events } = usePipeline();

  if (!detail) {
    return (
      <div className="flex h-48 items-center justify-center">
        <Spinner label="加载轨迹..." />
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <dl className="grid grid-cols-2 gap-x-6 gap-y-2 text-sm md:grid-cols-4">
        <div>
          <dt className="text-muted-foreground">需求</dt>
          <dd>{detail.demand_code}</dd>
        </div>
        <div>
          <dt className="text-muted-foreground">操作人</dt>
          <dd>{detail.operator || "-"}</dd>
        </div>
        <div>
          <dt className="text-muted-foreground">耗时</dt>
          <dd>{detail.duration_ms != null ? `${Math.round(detail.duration_ms / 1000)} s` : "-"}</dd>
        </div>
        <div>
          <dt className="text-muted-foreground">费用</dt>
          <dd>{detail.cost_usd != null ? `$${detail.cost_usd.toFixed(2)}` : "-"}</dd>
        </div>
      </dl>

      <StageProgress pipelineId={detail.id} stages={detail.stages} />

      <Separator />
      <h2 className="font-display text-base font-semibold">事件流</h2>
      <EventStream events={events} />
      <GateResults stages={detail.stages} />

      <Separator />
      <h2 className="font-display text-base font-semibold">阶段详情</h2>
      <div className="grid gap-2">
        {detail.stages.map((stage) => (
          <Card key={stage.name}>
            <CardContent className="p-4">
              <div className="flex items-center justify-between gap-3">
                <p className="font-semibold">{stage.name}</p>
                <div className="text-muted-foreground flex gap-3 text-xs">
                  {stage.retry_count > 0 ? <span>重试 {stage.retry_count} 次</span> : null}
                  {stage.started_at ? (
                    <span>{new Date(stage.started_at).toLocaleTimeString("zh-CN")}</span>
                  ) : null}
                </div>
              </div>
              {stage.output_summary ? (
                <p className="text-muted-foreground mt-1 text-xs">{stage.output_summary}</p>
              ) : null}
              {stage.issue ? <p className="text-destructive mt-1 text-xs">{stage.issue}</p> : null}
            </CardContent>
          </Card>
        ))}
      </div>
    </div>
  );
}

function WorkbenchConfirm() {
  const { state } = useStore();
  const { detail, humanRequest } = usePipeline();

  if (!detail) {
    return (
      <div className="flex h-48 items-center justify-center">
        <Spinner label="加载确认项..." />
      </div>
    );
  }

  if (humanRequest && humanRequest.pipelineId === detail.id) {
    return (
      <HumanLoopCard
        demandCode={detail.demand_code}
        demandId={detail.demand_id}
        detail={humanRequest.detail}
        issueClass={humanRequest.issueClass}
        pipelineId={humanRequest.pipelineId}
        stage={humanRequest.stage}
      />
    );
  }

  const summary = state.pipelines.find((pipeline) => pipeline.id === detail.id);
  if (summary?.issue_class === "awaiting_advance") {
    const stage =
      detail.stages.find((item) => item.issue?.includes("awaiting_advance"))?.name ??
      summary.current_stage ??
      "unknown";
    return (
      <HumanLoopCard
        demandCode={detail.demand_code}
        demandId={detail.demand_id}
        detail={summary.issue_detail ?? "阶段已完成，请确认后继续下一阶段。"}
        issueClass="awaiting_advance"
        pipelineId={detail.id}
        stage={stage}
      />
    );
  }

  if (detail.status === "waiting_merge") {
    return (
      <HumanLoopCard
        demandCode={detail.demand_code}
        demandId={detail.demand_id}
        detail="MR 已创建，等待审查人确认后在 Coding 合入（Poria 不会自动点合并）。"
        issueClass="waiting_merge"
        pipelineId={detail.id}
        stage="deploy"
      />
    );
  }

  return <EmptyState description="当前无需人工确认" />;
}

function WorkbenchWorkspace() {
  const { state } = useStore();
  const { detail } = usePipeline();
  const [opening, setOpening] = useState(false);
  const workspacePath = detail?.workspace_path?.trim() || null;

  async function handleOpen() {
    if (!state.selectedPipelineId || !workspacePath) {
      return;
    }
    setOpening(true);
    try {
      await openWorkspace(state.selectedPipelineId);
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

  if (!detail) {
    return (
      <div className="flex h-48 items-center justify-center">
        <Spinner label="读取工作区..." />
      </div>
    );
  }

  if (!workspacePath) {
    return <EmptyState description="请先完成初始化，生成工作区" />;
  }

  return (
    <div className="space-y-4">
      <p className="text-muted-foreground text-sm">
        Claude 阶段的 cwd 是流水线工作区根。文档是指向 projects 的软链；前后端 git worktree
        在该目录下。
      </p>
      <button
        className="bg-muted hover:bg-muted/80 cursor-pointer rounded-md px-2 py-1 font-mono text-sm"
        onClick={() => void copyPath()}
        type="button"
      >
        {workspacePath}
      </button>
      <div>
        <Button loading={opening} onClick={() => void handleOpen()}>
          <FolderOpen aria-hidden />
          在 Finder 中打开
        </Button>
      </div>
    </div>
  );
}

export function DemandWorkbench() {
  const { dispatch, state } = useStore();
  const { detail } = usePipeline();
  const vm = derivePipelineViewModel({
    detail,
    humanRequestPipelineId: state.humanRequest?.pipelineId ?? null,
    pipelines: state.pipelines,
    selectedPipelineId: state.selectedPipelineId,
  });

  function handlePrimaryCta() {
    switch (vm.primaryCta.action) {
      case "open_confirm":
        dispatch({ tab: "confirm", type: "workbenchTabChanged" });
        break;
      case "open_docs":
        dispatch({ tab: "docs", type: "workbenchTabChanged" });
        break;
      case "back_to_list":
        dispatch({ surface: "list", type: "surfaceChanged" });
        break;
      default:
        break;
    }
  }

  if (!state.selectedPipelineId) {
    return (
      <div className="flex h-full items-center justify-center">
        <EmptyState
          action={
            <Button onClick={() => dispatch({ surface: "list", type: "surfaceChanged" })}>
              返回列表
            </Button>
          }
          description="未选中流水线"
        />
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col">
      <header className="border-border flex flex-wrap items-center gap-3 border-b px-4 py-3">
        <Button
          onClick={() => dispatch({ surface: "list", type: "surfaceChanged" })}
          size="sm"
          variant="ghost"
        >
          <ArrowLeft aria-hidden />
          返回列表
        </Button>
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <h1 className="font-display truncate text-lg font-semibold">{vm.title}</h1>
            {detail ? <StatusBadge status={detail.status} /> : null}
            {vm.stageLabel ? <Badge variant="secondary">{vm.stageLabel}</Badge> : null}
          </div>
        </div>
        {vm.primaryCta.action !== "none" ? (
          <Button onClick={handlePrimaryCta} size="sm">
            {vm.primaryCta.label}
          </Button>
        ) : (
          <Badge variant="secondary">{vm.primaryCta.label}</Badge>
        )}
      </header>

      <Tabs
        className="flex min-h-0 flex-1 flex-col gap-0 p-4"
        onValueChange={(value) =>
          dispatch({ tab: value as WorkbenchTab, type: "workbenchTabChanged" })
        }
        value={state.ui.workbenchTab}
      >
        <TabsList className="w-fit shrink-0">
          <TabsTrigger value="trajectory">轨迹</TabsTrigger>
          <TabsTrigger value="docs">文档</TabsTrigger>
          <TabsTrigger value="workspace">工作区</TabsTrigger>
          <TabsTrigger value="confirm">
            对话
            {vm.badges.hasActiveHitl ? (
              <span className="bg-primary ml-1 inline-block size-1.5 rounded-full" />
            ) : null}
          </TabsTrigger>
        </TabsList>
        <div className="min-h-0 flex-1 overflow-auto pt-3">
          <TabsContent className="mt-0" value="trajectory">
            <WorkbenchTrajectory />
          </TabsContent>
          <TabsContent className="mt-0" value="docs">
            {detail ? (
              <DemandProjectBrowser demandCode={detail.demand_code} demandId={detail.demand_id} />
            ) : (
              <Spinner label="加载文档..." />
            )}
          </TabsContent>
          <TabsContent className="mt-0" value="workspace">
            <WorkbenchWorkspace />
          </TabsContent>
          <TabsContent className="mt-0" value="confirm">
            <WorkbenchConfirm />
          </TabsContent>
        </div>
      </Tabs>
    </div>
  );
}
