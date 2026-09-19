import * as LabelPrimitive from "@radix-ui/react-label";
import type { ComponentProps, ReactNode } from "react";

import { cn } from "@/lib/utils";

function Label({ className, ...props }: ComponentProps<typeof LabelPrimitive.Root>) {
  return (
    <LabelPrimitive.Root
      className={cn(
        "text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70",
        className,
      )}
      data-slot="label"
      {...props}
    />
  );
}

function Field({
  children,
  error,
  hint,
  label,
}: {
  children: ReactNode;
  error?: string;
  hint?: string;
  label: string;
}) {
  return (
    <div className="grid gap-1.5">
      <Label>{label}</Label>
      {children}
      {hint ? <p className="text-muted-foreground text-xs">{hint}</p> : null}
      {error ? <p className="text-destructive text-xs">{error}</p> : null}
    </div>
  );
}

export { Field, Label };
