import { ExternalLink } from "lucide-react";

import { usePipeline } from "../hooks/usePipeline";
import { useStore } from "../state/store";
import { EventStream } from "./EventStream";
import { GateResults } from "./GateResults";
import { HumanLoopCard } from "./HumanLoopCard";
import { StageProgress } from "./StageProgress";
import { StatusBadge } from "./StatusBadge";

export function PipelineDetail() {
  const { state } = useStore();
  const { detail, events, humanRequest } = usePipeline();

  if (!state.selectedPipelineId) {
    return (
      <div className="flex h-full items-center justify-center text-slate-500">
        选择一个 Pipeline 查看详情
      </div>
    );
  }

  if (!detail) {
    return <div className="flex h-full items-center justify-center text-slate-500">加载中...</div>;
  }

  return (
    <div className="h-full overflow-y-auto p-6">
      <div className="mb-6">
        <div className="mb-2 flex items-center gap-3">
          <h2 className="text-xl font-semibold text-slate-100">{detail.demand_name}</h2>
          <StatusBadge status={detail.status} />
        </div>
        <div className="flex flex-wrap gap-4 text-sm text-slate-400">
          <span>需求: {detail.demand_code}</span>
          <span>操作人: {detail.operator}</span>
          {detail.has_regressed && <span className="text-amber-400">已回退</span>}
          <a
            href={detail.raw_link}
            className="flex items-center gap-1 text-blue-400 hover:text-blue-300"
            target="_blank"
            rel="noreferrer"
          >
            行云链接 <ExternalLink className="h-3.5 w-3.5" />
          </a>
        </div>
      </div>

      <StageProgress stages={detail.stages} />

      {humanRequest && humanRequest.pipelineId === detail.id && (
        <div className="my-4">
          <HumanLoopCard
            pipelineId={humanRequest.pipelineId}
            stage={humanRequest.stage}
            issueClass={humanRequest.issueClass}
            detail={humanRequest.detail}
          />
        </div>
      )}

      <div className="mt-6 space-y-6">
        <div>
          <h3 className="mb-2 text-sm font-medium text-slate-300">事件流</h3>
          <EventStream events={events} />
        </div>

        <GateResults stages={detail.stages} />

        <div>
          <h3 className="mb-2 text-sm font-medium text-slate-300">阶段详情</h3>
          <div className="space-y-2">
            {detail.stages.map((stage) => (
              <div key={stage.name} className="rounded-lg bg-slate-800/50 px-4 py-3">
                <div className="flex items-center justify-between">
                  <span className="text-sm font-medium text-slate-200">{stage.name}</span>
                  <div className="flex items-center gap-3 text-xs text-slate-500">
                    {stage.retry_count > 0 && <span>重试 {stage.retry_count} 次</span>}
                    {stage.started_at && (
                      <span>{new Date(stage.started_at).toLocaleTimeString("zh-CN")}</span>
                    )}
                  </div>
                </div>
                {stage.output_summary && (
                  <p className="mt-1 text-xs text-slate-400">{stage.output_summary}</p>
                )}
                {stage.issue && <p className="mt-1 text-xs text-red-400">{stage.issue}</p>}
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
