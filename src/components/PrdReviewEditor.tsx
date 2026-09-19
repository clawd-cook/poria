import { useEffect, useState } from "react";
import { toast } from "sonner";

import { invokeErrorMessage } from "@/lib/errors";
import { readDemandProjectFile, writeDemandProjectFile } from "@/lib/tauri";
import type { PrdReviewStatus } from "@/lib/types";

import { MarkdownView } from "./MarkdownView";
import { Spinner } from "./Spinner";
import { Alert, AlertDescription } from "./ui/alert";
import { Button } from "./ui/button";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./ui/tabs";
import { Textarea } from "./ui/textarea";

function unansweredHint(status: PrdReviewStatus | null): {
  p0: string | null;
  warn: string | null;
} {
  if (!status) {
    return { p0: null, warn: null };
  }
  const p0 = status.p0_done
    ? null
    : status.p0_unanswered.length > 0
      ? `P0 未答：${status.p0_unanswered.join("、")}，不填无法进入设计`
      : "P0 尚未回答，不填无法进入设计";
  const warns: string[] = [];
  if (!status.p1_done && status.p1_unanswered.length > 0) {
    warns.push(`P1 未答 ${status.p1_unanswered.join("、")}（不阻塞）`);
  }
  if (!status.p2_done && status.p2_unanswered.length > 0) {
    warns.push(`P2 未答 ${status.p2_unanswered.join("、")}（不阻塞）`);
  }
  return { p0, warn: warns.length > 0 ? warns.join("；") : null };
}

export function PrdReviewEditor({
  demandCode,
  demandId,
  onSaved,
}: {
  demandCode: string;
  demandId?: number;
  onSaved?: (status: PrdReviewStatus | null) => void;
}) {
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [draft, setDraft] = useState("");
  const [saved, setSaved] = useState("");
  const [status, setStatus] = useState<PrdReviewStatus | null>(null);

  useEffect(() => {
    if (!demandCode) {
      return;
    }
    let cancelled = false;
    setLoading(true);
    setError(null);
    void (async () => {
      try {
        const file = await readDemandProjectFile({
          demandCode,
          demandId,
          fileName: "PRD_REVIEW.md",
        });
        if (cancelled) {
          return;
        }
        setDraft(file.content);
        setSaved(file.content);
        setStatus(file.review_status ?? null);
      } catch (err) {
        if (!cancelled) {
          setError(invokeErrorMessage(err, "文档读取失败"));
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
  }, [demandCode, demandId]);

  const dirty = draft !== saved;
  const hint = unansweredHint(status);

  async function handleSave() {
    setSaving(true);
    try {
      const file = await writeDemandProjectFile({
        content: draft,
        demandCode,
        demandId,
        fileName: "PRD_REVIEW.md",
      });
      setSaved(file.content);
      setDraft(file.content);
      setStatus(file.review_status ?? null);
      setError(null);
      toast.success("已保存 PRD_REVIEW.md");
      onSaved?.(file.review_status ?? null);
    } catch (err) {
      toast.error(invokeErrorMessage(err, "保存失败"));
    } finally {
      setSaving(false);
    }
  }

  if (loading) {
    return <Spinner />;
  }

  if (error && !draft) {
    return <p className="text-destructive text-sm">{error}</p>;
  }

  return (
    <div className="grid gap-3">
      {hint.p0 ? (
        <Alert variant="warning">
          <AlertDescription>{hint.p0}</AlertDescription>
        </Alert>
      ) : null}
      {hint.warn ? (
        <Alert>
          <AlertDescription>{hint.warn}</AlertDescription>
        </Alert>
      ) : null}
      {status?.p0_done ? (
        <Alert variant="success">
          <AlertDescription>P0 已填写，可以继续设计</AlertDescription>
        </Alert>
      ) : null}
      <Tabs defaultValue="edit">
        <TabsList>
          <TabsTrigger value="edit">填写答案</TabsTrigger>
          <TabsTrigger value="preview">预览</TabsTrigger>
        </TabsList>
        <TabsContent value="edit">
          <Textarea
            className="min-h-80"
            onChange={(event) => setDraft(event.target.value)}
            value={draft}
          />
        </TabsContent>
        <TabsContent value="preview">
          <MarkdownView content={draft} />
        </TabsContent>
      </Tabs>
      <div className="flex justify-end">
        <Button disabled={!dirty} loading={saving} onClick={() => void handleSave()}>
          保存答案
        </Button>
      </div>
    </div>
  );
}
