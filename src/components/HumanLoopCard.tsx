import { AlertTriangle, RotateCcw, SkipForward, XCircle } from "lucide-react";
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
    <div className="rounded-lg border border-amber-600/50 bg-amber-950/30 p-4">
      <div className="mb-3 flex items-center gap-2">
        <AlertTriangle className="h-5 w-5 text-amber-400" />
        <span className="font-medium text-amber-300">Pipeline 需要协助</span>
      </div>

      <div className="mb-4 space-y-1 text-sm">
        <div className="flex gap-2">
          <span className="text-slate-500">问题:</span>
          <span className="text-slate-300">{issueClass}</span>
        </div>
        <div className="flex gap-2">
          <span className="text-slate-500">阶段:</span>
          <span className="text-slate-300">{stage}</span>
        </div>
        <div className="mt-2 rounded bg-slate-800 p-2 text-xs text-slate-400">{detail}</div>
      </div>

      <div className="flex gap-2">
        <button
          onClick={() => handleAction("resume")}
          disabled={loading !== null}
          className="flex items-center gap-1.5 rounded-lg bg-emerald-600 px-3 py-1.5 text-sm font-medium text-white transition-colors hover:bg-emerald-500 disabled:opacity-50"
        >
          <RotateCcw className="h-3.5 w-3.5" />
          修复并重试
        </button>
        <button
          onClick={() => handleAction("skip")}
          disabled={loading !== null}
          className="flex items-center gap-1.5 rounded-lg bg-amber-600 px-3 py-1.5 text-sm font-medium text-white transition-colors hover:bg-amber-500 disabled:opacity-50"
        >
          <SkipForward className="h-3.5 w-3.5" />
          跳过
        </button>
        <button
          onClick={() => handleAction("cancel")}
          disabled={loading !== null}
          className="flex items-center gap-1.5 rounded-lg bg-red-600 px-3 py-1.5 text-sm font-medium text-white transition-colors hover:bg-red-500 disabled:opacity-50"
        >
          <XCircle className="h-3.5 w-3.5" />
          取消 Pipeline
        </button>
      </div>
    </div>
  );
}
