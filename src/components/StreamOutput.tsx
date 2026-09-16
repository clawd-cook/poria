import { useEffect, useRef } from "react";

import type { StreamChunk } from "../lib/types";
import { useStore } from "../state/store";

export function StreamOutput({ pipelineId }: { pipelineId: string }) {
  const { state } = useStore();
  const chunks = state.streamOutput[pipelineId] ?? [];
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [chunks.length]);

  if (chunks.length === 0) {
    return <div className="py-4 text-sm italic text-slate-500">等待 Claude 输出...</div>;
  }

  return (
    <div className="max-h-96 space-y-1 overflow-y-auto font-mono text-sm">
      {chunks.map((chunk, i) => (
        <ChunkLine key={i} chunk={chunk} />
      ))}
      <div ref={bottomRef} />
    </div>
  );
}

function ChunkLine({ chunk }: { chunk: StreamChunk }) {
  switch (chunk.type) {
    case "text":
      return <div className="whitespace-pre-wrap text-slate-200">{chunk.content}</div>;
    case "tool_use":
      return (
        <details className="text-blue-400">
          <summary className="cursor-pointer">{"🔧"} {chunk.tool_name ?? "tool"}</summary>
          <pre className="overflow-x-auto pl-4 text-xs text-slate-400">{chunk.content}</pre>
        </details>
      );
    case "tool_result":
      return (
        <details className="text-green-400">
          <summary className="cursor-pointer">{"📋"} Result</summary>
          <pre className="overflow-x-auto pl-4 text-xs text-slate-400">{chunk.content}</pre>
        </details>
      );
    case "result":
      return <div className="font-semibold text-emerald-400">{"✓"} {chunk.content}</div>;
    default:
      return <div className="text-slate-500">{chunk.content}</div>;
  }
}
