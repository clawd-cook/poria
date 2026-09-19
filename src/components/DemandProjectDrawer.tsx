import { useEffect, useState } from "react";

import { invokeErrorMessage } from "@/lib/errors";
import { listDemandProject, readDemandProjectFile } from "@/lib/tauri";
import type { DemandListItem, DemandProjectFile } from "@/lib/types";
import { cn } from "@/lib/utils";

import { EmptyState } from "./EmptyState";
import { MarkdownView } from "./MarkdownView";
import { PrdReviewEditor } from "./PrdReviewEditor";
import { Spinner } from "./Spinner";
import { Sheet, SheetContent, SheetHeader, SheetTitle } from "./ui/sheet";

function fileLabel(name: string): string {
  switch (name) {
    case "PRD.md":
      return "PRD";
    case "BACKEND_TRD.md":
      return "后端 TRD";
    case "PRD_REVIEW.md":
      return "需求评审";
    case "TRD.md":
      return "前端 TRD";
    default:
      return name.replace(/\.md$/i, "");
  }
}

export function DemandProjectDrawer({
  demand,
  onClose,
}: {
  demand: DemandListItem | null;
  onClose: () => void;
}) {
  const open = demand !== null;
  const demandCode = demand?.demand_code ?? "";
  const demandId = demand?.id;

  const [files, setFiles] = useState<DemandProjectFile[]>([]);
  const [exists, setExists] = useState(false);
  const [projectDir, setProjectDir] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [activeFile, setActiveFile] = useState<string | null>(null);
  const [content, setContent] = useState("");
  const [contentLoading, setContentLoading] = useState(false);

  useEffect(() => {
    if (!open || !demandCode) {
      return;
    }

    let cancelled = false;
    setLoading(true);
    setError(null);
    setFiles([]);
    setActiveFile(null);
    setContent("");
    setExists(false);

    void (async () => {
      try {
        const project = await listDemandProject({ demandCode, demandId });
        if (cancelled) {
          return;
        }
        setExists(project.exists);
        setFiles(project.files);
        setProjectDir(project.project_dir);
        const review = project.files.find((file) => file.name === "PRD_REVIEW.md");
        setActiveFile(review?.name ?? project.files[0]?.name ?? null);
      } catch (err) {
        if (!cancelled) {
          setError(invokeErrorMessage(err, "项目文档加载失败"));
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [demandCode, demandId, open]);

  useEffect(() => {
    if (!open || !demandCode || !activeFile || activeFile === "PRD_REVIEW.md") {
      return;
    }

    let cancelled = false;
    setContentLoading(true);
    void (async () => {
      try {
        const file = await readDemandProjectFile({
          demandCode,
          demandId,
          fileName: activeFile,
        });
        if (!cancelled) {
          setContent(file.content);
        }
      } catch (err) {
        if (!cancelled) {
          setContent("");
          setError(invokeErrorMessage(err, "文档读取失败"));
        }
      } finally {
        if (!cancelled) {
          setContentLoading(false);
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [activeFile, demandCode, demandId, open]);

  return (
    <Sheet
      onOpenChange={(next) => {
        if (!next) {
          onClose();
        }
      }}
      open={open}
    >
      <SheetContent className="overflow-hidden">
        <SheetHeader>
          <SheetTitle>{demand ? `${demand.name} 文档` : "项目文档"}</SheetTitle>
        </SheetHeader>
        <div className="min-h-0 flex-1 overflow-auto">
          {loading ? (
            <Spinner />
          ) : error && files.length === 0 ? (
            <EmptyState description={error} />
          ) : !exists ? (
            <EmptyState description="尚未导出文档，请先在流水线执行初始化" />
          ) : files.length === 0 ? (
            <EmptyState description="项目目录为空" />
          ) : (
            <div className="flex min-h-[360px] gap-4">
              <nav aria-label="项目文档" className="w-40 shrink-0">
                {files.map((file) => (
                  <button
                    className={cn(
                      "mb-1 flex h-10 w-full cursor-pointer items-center rounded-md px-3 text-left text-sm transition-colors duration-200 focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none",
                      activeFile === file.name
                        ? "bg-primary text-primary-foreground"
                        : "hover:bg-muted",
                    )}
                    key={file.name}
                    onClick={() => {
                      setError(null);
                      setActiveFile(file.name);
                    }}
                    type="button"
                  >
                    {fileLabel(file.name)}
                  </button>
                ))}
              </nav>
              <div className="min-w-0 flex-1 overflow-auto">
                {projectDir ? (
                  <p className="text-muted-foreground mb-2 text-xs">{projectDir}</p>
                ) : null}
                {activeFile === "PRD_REVIEW.md" ? (
                  <PrdReviewEditor demandCode={demandCode} demandId={demandId} />
                ) : error ? (
                  <p className="text-destructive text-sm">{error}</p>
                ) : contentLoading ? (
                  <Spinner />
                ) : (
                  <MarkdownView content={content} />
                )}
              </div>
            </div>
          )}
        </div>
      </SheetContent>
    </Sheet>
  );
}
