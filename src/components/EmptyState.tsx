import type { ReactNode } from "react";

export function EmptyState({ action, description }: { action?: ReactNode; description: string }) {
  return (
    <div className="flex flex-col items-center justify-center gap-3 px-4 py-10 text-center">
      <p className="text-muted-foreground text-sm">{description}</p>
      {action}
    </div>
  );
}
