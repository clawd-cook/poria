import { CircleAlert, CircleCheck, CirclePlay, CircleX, Info } from "lucide-react";
import { useEffect, useRef, type ReactNode } from "react";

import type { PipelineEvent } from "@/lib/types";

import { EmptyState } from "./EmptyState";
import { Badge } from "./ui/badge";

const KIND_CONFIG: Record<
  string,
  { icon: ReactNode; variant: "default" | "destructive" | "secondary" | "success" | "warning" }
> = {
  gate_failed: {
    icon: <CircleX aria-hidden className="text-destructive size-4" />,
    variant: "destructive",
  },
  gate_passed: {
    icon: <CircleCheck aria-hidden className="text-success size-4" />,
    variant: "success",
  },
  human_loop: {
    icon: <CircleAlert aria-hidden className="text-warning size-4" />,
    variant: "warning",
  },
  stage_completed: {
    icon: <CircleCheck aria-hidden className="text-success size-4" />,
    variant: "success",
  },
  stage_failed: {
    icon: <CircleX aria-hidden className="text-destructive size-4" />,
    variant: "destructive",
  },
  stage_started: {
    icon: <CirclePlay aria-hidden className="text-primary size-4" />,
    variant: "default",
  },
};

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
    return <EmptyState description="暂无事件" />;
  }

  return (
    <div className="max-h-64 overflow-y-auto">
      <ol className="grid gap-3">
        {events.map((event) => {
          const cfg = KIND_CONFIG[event.kind] ?? {
            icon: <Info aria-hidden className="text-muted-foreground size-4" />,
            variant: "secondary" as const,
          };
          return (
            <li className="flex gap-3" key={`${event.seq}-${event.created_at}`}>
              <span className="mt-0.5">{cfg.icon}</span>
              <div className="min-w-0">
                <div className="flex flex-wrap items-center gap-2">
                  <Badge variant={cfg.variant}>{event.kind}</Badge>
                  <span className="text-muted-foreground text-xs">
                    {formatTime(event.created_at)}
                  </span>
                </div>
                <p className="text-sm">{event.payload}</p>
              </div>
            </li>
          );
        })}
      </ol>
      <div ref={endRef} />
    </div>
  );
}
