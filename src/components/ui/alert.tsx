import { cva, type VariantProps } from "class-variance-authority";
import { CircleAlert, CircleCheck, Info, TriangleAlert } from "lucide-react";
import type { HTMLAttributes } from "react";

import { cn } from "@/lib/utils";

const alertVariants = cva("relative w-full rounded-lg border p-4 text-sm", {
  defaultVariants: {
    variant: "default",
  },
  variants: {
    variant: {
      default: "border-border bg-card text-foreground",
      destructive: "border-destructive/40 bg-destructive/10 text-destructive",
      success: "border-success/40 bg-success/10 text-success",
      warning: "border-warning/40 bg-warning/10 text-warning",
    },
  },
});

const ICONS = {
  default: Info,
  destructive: CircleAlert,
  success: CircleCheck,
  warning: TriangleAlert,
};

function Alert({
  className,
  variant = "default",
  ...props
}: HTMLAttributes<HTMLDivElement> & VariantProps<typeof alertVariants>) {
  return (
    <div
      className={cn(alertVariants({ className, variant }))}
      data-slot="alert"
      role="alert"
      {...props}
    />
  );
}

function AlertTitle({ className, ...props }: HTMLAttributes<HTMLHeadingElement>) {
  return (
    <h5
      className={cn("mb-1 font-medium leading-none", className)}
      data-slot="alert-title"
      {...props}
    />
  );
}

function AlertDescription({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return (
    <div className={cn("text-sm opacity-90", className)} data-slot="alert-description" {...props} />
  );
}

function AlertIcon({ variant = "default" }: { variant?: keyof typeof ICONS }) {
  const Icon = ICONS[variant];
  return <Icon aria-hidden className="mt-0.5 size-4 shrink-0" />;
}

export { Alert, AlertDescription, AlertIcon, AlertTitle };
