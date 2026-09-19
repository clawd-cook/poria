import { ExternalLink, TriangleAlert } from "lucide-react";

import { usePipeline } from "@/hooks/usePipeline";
import { useStore } from "@/state/store";

import { EmptyState } from "./EmptyState";
import { EventStream } from "./EventStream";
import { GateResults } from "./GateResults";
import { HumanLoopCard } from "./HumanLoopCard";
import { PageFrame, PageSectionTitle } from "./PageFrame";
import { Spinner } from "./Spinner";
import { StageProgress } from "./StageProgress";
import { StatusBadge } from "./StatusBadge";
import { Badge } from "./ui/badge";
import { Card, CardContent } from "./ui/card";
import { Separator } from "./ui/separator";

export function PipelineDetail() {
  const { state } = useStore();
  const { detail, events, humanRequest } = usePipeline();

  if (!state.selectedPipelineId) {
    return (
      <div className="flex h-full items-center justify-center">
        <EmptyState description="选择一个 Pipeline 查看详情" />
      </div>
    );
  }

  if (!detail) {
    return (
      <div className="flex h-full items-center justify-center">
        <Spinner label="加载中..." />
      </div>
    );
  }

  return (
    <PageFrame
      extra={
        <div className="flex items-center gap-2">
          <StatusBadge status={detail.status} />
          {detail.has_regressed ? (
            <Badge variant="warning">
              <TriangleAlert aria-hidden className="mr-1 size-3" />
              已回退
            </Badge>
          ) : null}
        </div>
      }
      title={detail.demand_name}
    >
      <dl className="mb-4 grid grid-cols-2 gap-x-6 gap-y-2 text-sm md:grid-cols-4">
        <div>
          <dt className="text-muted-foreground">需求</dt>
          <dd>{detail.demand_code}</dd>
        </div>
        <div>
          <dt className="text-muted-foreground">操作人</dt>
          <dd>{detail.operator || "-"}</dd>
        </div>
        <div>
          <dt className="text-muted-foreground">行云链接</dt>
          <dd>
            <a
              className="text-primary inline-flex items-center gap-1 hover:underline"
              href={detail.raw_link}
              rel="noreferrer"
              target="_blank"
            >
              <ExternalLink aria-hidden className="size-3.5" />
              查看
            </a>
          </dd>
        </div>
        <div>
          <dt className="text-muted-foreground">耗时</dt>
          <dd>{detail.duration_ms != null ? `${Math.round(detail.duration_ms / 1000)} s` : "-"}</dd>
        </div>
        <div>
          <dt className="text-muted-foreground">费用</dt>
          <dd>{detail.cost_usd != null ? `$${detail.cost_usd.toFixed(2)}` : "-"}</dd>
        </div>
        <div>
          <dt className="text-muted-foreground">人工介入</dt>
          <dd>{detail.hitl_count}</dd>
        </div>
      </dl>

      <StageProgress pipelineId={detail.id} stages={detail.stages} />

      {humanRequest && humanRequest.pipelineId === detail.id ? (
        <div className="my-4">
          <HumanLoopCard
            demandCode={detail.demand_code}
            demandId={detail.demand_id}
            detail={humanRequest.detail}
            issueClass={humanRequest.issueClass}
            pipelineId={humanRequest.pipelineId}
            stage={humanRequest.stage}
          />
        </div>
      ) : detail.status === "waiting_merge" ? (
        <div className="my-4">
          <HumanLoopCard
            demandCode={detail.demand_code}
            demandId={detail.demand_id}
            detail="MR 已创建，等待审查人确认后在 Coding 合入（Poria 不会自动点合并）。"
            issueClass="waiting_merge"
            pipelineId={detail.id}
            stage="deploy"
          />
        </div>
      ) : null}

      <Separator className="my-6" />

      <PageSectionTitle>事件流</PageSectionTitle>
      <EventStream events={events} />

      <GateResults stages={detail.stages} />

      <Separator className="my-6" />
      <PageSectionTitle>阶段详情</PageSectionTitle>
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
    </PageFrame>
  );
}
