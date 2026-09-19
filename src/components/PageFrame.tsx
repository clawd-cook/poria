import type { ReactNode } from "react";

export function PageFrame({
  children,
  description,
  extra,
  title,
}: {
  children?: ReactNode;
  description?: ReactNode;
  extra?: ReactNode;
  title: string;
}) {
  return (
    <div className="bg-background min-h-full p-6">
      <div className="mb-6 flex items-start justify-between gap-4">
        <div>
          <h1 className="font-serif text-2xl font-bold tracking-tight">{title}</h1>
          {description ? <p className="text-muted-foreground mt-1 text-sm">{description}</p> : null}
        </div>
        {extra ? <div>{extra}</div> : null}
      </div>
      {children}
    </div>
  );
}

export function PageSectionTitle({ children }: { children: ReactNode }) {
  return <h2 className="mb-2 font-serif text-base font-bold">{children}</h2>;
}
