import { Alert, App, Button, Flex, Input, Spin, Tabs, Typography, theme } from "antd";
import { useEffect, useState } from "react";

import { invokeErrorMessage } from "../lib/errors";
import { readDemandProjectFile, writeDemandProjectFile } from "../lib/tauri";
import type { PrdReviewStatus } from "../lib/types";

const { Text } = Typography;

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
  const { message } = App.useApp();
  const { token } = theme.useToken();
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [draft, setDraft] = useState("");
  const [saved, setSaved] = useState("");
  const [status, setStatus] = useState<PrdReviewStatus | null>(null);
  const [Markdown, setMarkdown] = useState<
    typeof import("@ant-design/x-markdown").XMarkdown | null
  >(null);

  useEffect(() => {
    let cancelled = false;
    void import("@ant-design/x-markdown").then((mod) => {
      if (!cancelled) {
        setMarkdown(() => mod.XMarkdown);
      }
    });
    return () => {
      cancelled = true;
    };
  }, []);

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
      message.success("已保存 PRD_REVIEW.md");
      onSaved?.(file.review_status ?? null);
    } catch (err) {
      message.error(invokeErrorMessage(err, "保存失败"));
    } finally {
      setSaving(false);
    }
  }

  if (loading) {
    return (
      <div style={{ padding: 48, textAlign: "center" }}>
        <Spin />
      </div>
    );
  }

  if (error && !draft) {
    return <Text type="danger">{error}</Text>;
  }

  return (
    <Flex gap={token.marginSM} vertical>
      {hint.p0 ? <Alert showIcon type="warning" message={hint.p0} /> : null}
      {hint.warn ? <Alert showIcon type="info" message={hint.warn} /> : null}
      {status?.p0_done ? <Alert showIcon type="success" message="P0 已填写，可以继续设计" /> : null}
      <Tabs
        items={[
          {
            children: (
              <Input.TextArea
                autoSize={{ maxRows: 28, minRows: 16 }}
                onChange={(event) => setDraft(event.target.value)}
                value={draft}
              />
            ),
            key: "edit",
            label: "填写答案",
          },
          {
            children: Markdown ? <Markdown content={draft} openLinksInNewTab /> : <Spin />,
            key: "preview",
            label: "预览",
          },
        ]}
      />
      <Flex justify="flex-end">
        <Button disabled={!dirty} loading={saving} onClick={() => void handleSave()} type="primary">
          保存答案
        </Button>
      </Flex>
    </Flex>
  );
}
