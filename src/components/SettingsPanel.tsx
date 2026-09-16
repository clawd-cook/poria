import { X, Save } from "lucide-react";
import { useState } from "react";

import { updateConfig } from "../lib/tauri";
import type { AppConfig } from "../lib/types";
import { useStore } from "../state/store";

export function SettingsPanel() {
  const { state, dispatch } = useStore();
  const [saving, setSaving] = useState(false);
  const [draft, setDraft] = useState<AppConfig | null>(null);

  const config = draft ?? state.config;
  if (!config) return null;

  function handleChange(field: keyof AppConfig, value: string | number) {
    const updated = { ...(draft ?? state.config!), [field]: value };
    setDraft(updated);
  }

  async function handleSave() {
    if (!draft) return;
    setSaving(true);
    try {
      await updateConfig(draft);
      dispatch({ type: "configLoaded", config: draft });
      setDraft(null);
    } catch {
      /* noop */
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="w-full max-w-md rounded-xl bg-slate-800 shadow-2xl">
        <div className="flex items-center justify-between border-b border-slate-700 px-6 py-4">
          <h2 className="text-lg font-medium text-slate-200">设置</h2>
          <button
            onClick={() => {
              setDraft(null);
              dispatch({ type: "settingsToggled", open: false });
            }}
            className="text-slate-400 transition-colors hover:text-slate-200"
          >
            <X className="h-5 w-5" />
          </button>
        </div>

        <div className="space-y-4 p-6">
          <SettingField
            label="CR 评分阈值"
            value={config.cr_score_threshold}
            onChange={(v) => handleChange("cr_score_threshold", v)}
          />
          <SettingField
            label="测试覆盖率 (%)"
            value={String(config.test_coverage_threshold)}
            type="number"
            onChange={(v) => handleChange("test_coverage_threshold", Number(v))}
          />
          <SettingField
            label="最大变更行数"
            value={String(config.max_diff_lines)}
            type="number"
            onChange={(v) => handleChange("max_diff_lines", Number(v))}
          />
          <SettingField
            label="Agent 超时 (ms)"
            value={String(config.agent_timeout_ms)}
            type="number"
            onChange={(v) => handleChange("agent_timeout_ms", Number(v))}
          />
          <SettingField
            label="最大重试次数"
            value={String(config.max_retries)}
            type="number"
            onChange={(v) => handleChange("max_retries", Number(v))}
          />
          <SettingField
            label="数据目录"
            value={config.db_path}
            onChange={(v) => handleChange("db_path", v)}
          />
        </div>

        <div className="flex justify-end border-t border-slate-700 px-6 py-4">
          <button
            onClick={handleSave}
            disabled={saving || !draft}
            className="flex items-center gap-1.5 rounded-lg bg-blue-600 px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-blue-500 disabled:opacity-50"
          >
            <Save className="h-4 w-4" />
            保存
          </button>
        </div>
      </div>
    </div>
  );
}

function SettingField({
  label,
  value,
  type = "text",
  onChange,
}: {
  label: string;
  value: string;
  type?: string;
  onChange: (v: string) => void;
}) {
  return (
    <div>
      <label className="mb-1 block text-sm text-slate-400">{label}</label>
      <input
        type={type}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="w-full rounded-lg border border-slate-600 bg-slate-900 px-3 py-1.5 text-sm text-slate-200 outline-none focus:border-blue-500"
      />
    </div>
  );
}
