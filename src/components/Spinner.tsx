import { LoaderCircle } from "lucide-react";

import { cn } from "@/lib/utils";

export function Spinner({ className, label }: { className?: string; label?: string }) {
  return (
    <div
      className={cn(
        "flex items-center justify-center gap-2 p-6 text-sm text-muted-foreground",
        className,
      )}
      role="status"
    >
      <LoaderCircle aria-hidden className="size-4 animate-spin" />
      {label ? <span>{label}</span> : <span className="sr-only">加载中</span>}
    </div>
  );
}
