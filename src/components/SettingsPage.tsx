import { useEffect, useState, type FormEvent } from "react";
import { toast } from "sonner";

import { saveHitlAutoNavigate } from "@/lib/hitlPrefs";
import { countAttentionPipelines } from "@/lib/pipelineViewModel";
import { probeClaude, updateConfig } from "@/lib/tauri";
import type { AppConfig, ClaudeProbeResult } from "@/lib/types";
import { useStore } from "@/state/store";

import { PageFrame, PageSectionTitle } from "./PageFrame";
import { PipelineStatsPanel } from "./PipelineStatsPanel";
import { Spinner } from "./Spinner";
import { Alert, AlertDescription, AlertTitle } from "./ui/alert";
import { Button } from "./ui/button";
import { Checkbox } from "./ui/checkbox";
import { Input } from "./ui/input";
import { Field, Label } from "./ui/label";
import { Textarea } from "./ui/textarea";

function emptyToNull(value: string | null | undefined): string | null {
  const trimmed = value?.trim();
  return trimmed ? trimmed : null;
}

export function SettingsPage() {
  const { dispatch, state } = useStore();
  const [saving, setSaving] = useState(false);
  const [probing, setProbing] = useState(false);
  const [probe, setProbe] = useState<ClaudeProbeResult | null>(null);
  const [form, setForm] = useState<AppConfig | null>(null);
  const config = state.config;
  const view = state.ui.view;

  async function runProbe(pathOverride?: string | null) {
    setProbing(true);
    try {
      const result = await probeClaude(pathOverride ?? null);
      setProbe(result);
    } catch (error) {
      const reason = error instanceof Error ? error.message : String(error);
      setProbe({
        error: reason,
        ok: false,
        resolvedPath: null,
        source: null,
        version: null,
      });
    } finally {
      setProbing(false);
    }
  }

  useEffect(() => {
    if (!config) {
      return;
    }
    setForm({
      ...config,
      claude_path: config.claude_path ?? "",
      dev_verify_commands: config.dev_verify_commands ?? "",
    });
  }, [config]);

  useEffect(() => {
    if (!config || view !== "settings") {
      return;
    }
    void runProbe(config.claude_path);
  }, [config, view]);

  if (!config || !form) {
    return (
      <PageFrame title="设置">
        <Spinner label="加载设置..." />
      </PageFrame>
    );
  }

  const settings = form;
  const attentionCount = countAttentionPipelines(
    state.pipelines,
    state.humanRequest?.pipelineId ?? null,
  );
  const hitlTargetId = state.humanRequest?.pipelineId ?? null;

  async function handleSave(event: FormEvent) {
    event.preventDefault();
    const payload: AppConfig = {
      ...settings,
      claude_path: emptyToNull(settings.claude_path),
      dev_verify_commands: emptyToNull(settings.dev_verify_commands),
    };
    setSaving(true);
    try {
      await updateConfig(payload);
      dispatch({ config: payload, type: "configLoaded" });
      toast.success("设置已保存");
    } catch {
      toast.error("保存失败");
    } finally {
      setSaving(false);
    }
  }

  const sourceLabel =
    probe?.source === "config" ? "手动配置" : probe?.source === "which" ? "自动 which" : null;

  return (
    <PageFrame title="设置">
      <div className="mb-6 max-w-xl space-y-3">
        <PageSectionTitle>交付体验</PageSectionTitle>
        <div className="flex items-start gap-2">
          <Checkbox
            checked={state.ui.hitlAutoNavigate}
            id="hitl-auto-nav"
            onCheckedChange={(value) => {
              const enabled = value === true;
              saveHitlAutoNavigate(enabled);
              dispatch({ enabled, type: "hitlAutoNavigateChanged" });
            }}
          />
          <div className="grid gap-1">
            <Label htmlFor="hitl-auto-nav">收到人工确认时自动打开工作台</Label>
            <p className="text-muted-foreground text-xs">
              关闭后仅更新侧栏角标与「待我处理」筛选，不强制跳转。
            </p>
          </div>
        </div>
        {attentionCount > 0 && hitlTargetId ? (
          <Button
            onClick={() => dispatch({ pipelineId: hitlTargetId, type: "openHitlWorkbench" })}
            type="button"
            variant="outline"
          >
            打开待确认工作台（{attentionCount}）
          </Button>
        ) : null}
        <p className="text-muted-foreground text-xs">
          渠道与随包技能已移出主航道；交付仍由六阶段流水线驱动。仓库登记请用侧栏「仓库」。
        </p>
      </div>
      <form className="grid max-w-xl gap-4" onSubmit={(event) => void handleSave(event)}>
        <Field label="CR 评分阈值">
          <Input
            onChange={(event) => setForm({ ...form, cr_score_threshold: event.target.value })}
            value={form.cr_score_threshold}
          />
        </Field>
        <Field label="测试覆盖率 (%)">
          <Input
            onChange={(event) =>
              setForm({ ...form, test_coverage_threshold: Number(event.target.value) })
            }
            type="number"
            value={form.test_coverage_threshold}
          />
        </Field>
        <Field label="最大变更行数">
          <Input
            onChange={(event) => setForm({ ...form, max_diff_lines: Number(event.target.value) })}
            type="number"
            value={form.max_diff_lines}
          />
        </Field>
        <Field label="Agent 超时 (ms)">
          <Input
            onChange={(event) => setForm({ ...form, agent_timeout_ms: Number(event.target.value) })}
            type="number"
            value={form.agent_timeout_ms}
          />
        </Field>
        <Field label="最大重试次数">
          <Input
            onChange={(event) => setForm({ ...form, max_retries: Number(event.target.value) })}
            type="number"
            value={form.max_retries}
          />
        </Field>
        <Field
          hint="Blocked / 失败 / 待合并不占槽。同一 worktree 不会被两条流水线同时写。"
          label="并行流水线数"
        >
          <Input
            max={8}
            min={1}
            onChange={(event) =>
              setForm({ ...form, max_parallel_pipelines: Number(event.target.value) })
            }
            type="number"
            value={form.max_parallel_pipelines}
          />
        </Field>
        <Field label="数据目录">
          <Input
            onChange={(event) => setForm({ ...form, db_path: event.target.value })}
            value={form.db_path}
          />
        </Field>
        <Field
          hint="留空则用登录 shell 执行 which claude。填写时必须是绝对路径。"
          label="Claude 路径"
        >
          <Input
            onChange={(event) => setForm({ ...form, claude_path: event.target.value })}
            placeholder={probe?.resolvedPath ?? "/opt/homebrew/bin/claude"}
            value={form.claude_path ?? ""}
          />
        </Field>
        <Field
          hint="每行一条，在前端 worktree 里跑。留空则按 package.json 约定（至少 typecheck；有非 watch 测试脚本则跑测试）。"
          label="Dev 本地验证命令"
        >
          <Textarea
            onChange={(event) => setForm({ ...form, dev_verify_commands: event.target.value })}
            placeholder={"pnpm typecheck\npnpm exec vitest run"}
            value={form.dev_verify_commands ?? ""}
          />
        </Field>
        <div className="flex flex-wrap gap-2">
          <Button loading={saving} type="submit">
            保存
          </Button>
          <Button
            loading={probing}
            onClick={() => void runProbe(form.claude_path)}
            type="button"
            variant="outline"
          >
            刷新 Claude 状态
          </Button>
        </div>
      </form>
      {probing && !probe ? (
        <Spinner label="正在探测 Claude CLI..." />
      ) : probe ? (
        <Alert className="mt-4 max-w-xl" variant={probe.ok ? "success" : "destructive"}>
          <AlertTitle>{probe.ok ? "Claude CLI 可用" : "Claude CLI 不可用"}</AlertTitle>
          <AlertDescription className="text-foreground grid gap-1">
            {probe.resolvedPath ? <p>路径：{probe.resolvedPath}</p> : null}
            {sourceLabel ? <p className="text-muted-foreground">来源：{sourceLabel}</p> : null}
            {probe.version ? <p>版本：{probe.version}</p> : null}
            {probe.error ? <p className="text-destructive">{probe.error}</p> : null}
          </AlertDescription>
        </Alert>
      ) : null}
      <div className="mt-6 max-w-4xl">
        <PipelineStatsPanel />
      </div>
    </PageFrame>
  );
}
