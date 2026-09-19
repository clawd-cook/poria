import type { TextareaHTMLAttributes } from "react";

import { cn } from "@/lib/utils";

function Textarea({ className, ...props }: TextareaHTMLAttributes<HTMLTextAreaElement>) {
  return (
    <textarea
      className={cn(
        "flex min-h-24 w-full rounded-md border border-input bg-card px-3 py-2 text-sm transition-colors duration-200 placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50",
        className,
      )}
      data-slot="textarea"
      {...props}
    />
  );
}

export { Textarea };
