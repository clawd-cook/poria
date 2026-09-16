import { useEffect, useRef } from "react";

import type { PipelineEvent } from "../lib/types";

function EventKindBadge({ kind }: { kind: string }) {
  const colors: Record<string, string> = {
    stage_started: "bg-blue-600/30 text-blue-300",
    stage_completed: "bg-emerald-600/30 text-emerald-300",
    stage_failed: "bg-red-600/30 text-red-300",
    gate_passed: "bg-emerald-600/30 text-emerald-300",
    gate_failed: "bg-red-600/30 text-red-300",
    human_loop: "bg-amber-600/30 text-amber-300",
  };

  return (
    <span
      className={`rounded px-1.5 py-0.5 text-xs font-medium ${colors[kind] ?? "bg-slate-700 text-slate-400"}`}
    >
      {kind}
    </span>
  );
}

function formatTime(dateStr: string): string {
  try {
    return new Date(dateStr).toLocaleTimeString("zh-CN", {
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });
  } catch {
    return dateStr;
  }
}

export function EventStream({ events }: { events: PipelineEvent[] }) {
  const endRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    endRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [events.length]);

  if (events.length === 0) {
    return <div className="py-8 text-center text-sm text-slate-500">暂无事件</div>;
  }

  return (
    <div className="max-h-64 overflow-y-auto rounded-lg bg-slate-800/50 p-3">
      <div className="space-y-2">
        {events.map((event) => (
          <div key={event.seq} className="flex items-start gap-2 text-sm">
            <span className="shrink-0 text-xs text-slate-600">{formatTime(event.created_at)}</span>
            <EventKindBadge kind={event.kind} />
            <span className="min-w-0 truncate text-slate-300">{event.payload}</span>
          </div>
        ))}
        <div ref={endRef} />
      </div>
    </div>
  );
}
