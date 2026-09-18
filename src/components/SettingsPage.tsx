import { Alert, App, Button, Flex, Form, Input, InputNumber, Spin, Typography, theme } from "antd";
import { useEffect, useState } from "react";

import { probeClaude, updateConfig } from "../lib/tauri";
import type { AppConfig, ClaudeProbeResult } from "../lib/types";
import { useStore } from "../state/store";
import { PageFrame } from "./PageFrame";
import { PipelineStatsPanel } from "./PipelineStatsPanel";

const { Text } = Typography;

function emptyToNull(value: string | null | undefined): string | null {
  const trimmed = value?.trim();
  return trimmed ? trimmed : null;
}

export function SettingsPage() {
  const { dispatch, state } = useStore();
  const { message } = App.useApp();
  const { token } = theme.useToken();
  const [saving, setSaving] = useState(false);
  const [probing, setProbing] = useState(false);
  const [probe, setProbe] = useState<ClaudeProbeResult | null>(null);
  const [form] = Form.useForm<AppConfig>();
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
    form.setFieldsValue({
      ...config,
      claude_path: config.claude_path ?? "",
    });
  }, [config, form]);

  // Settings stays mounted in a hidden tab. Probe on enter / after save (config
  // change while visible), not at app start and not on a timer.
  useEffect(() => {
    if (!config || view !== "settings") {
      return;
    }
    void runProbe(form.getFieldValue("claude_path") ?? config.claude_path);
  }, [config, form, view]);

  if (!config) {
    return (
      <PageFrame title="设置">
        <div
          style={{ alignItems: "center", display: "flex", height: 240, justifyContent: "center" }}
        >
          <Spin tip="加载设置..." />
        </div>
      </PageFrame>
    );
  }

  async function handleSave() {
    try {
      const values = await form.validateFields();
      const payload: AppConfig = {
        ...values,
        claude_path: emptyToNull(values.claude_path),
      };
      setSaving(true);
      await updateConfig(payload);
      dispatch({ config: payload, type: "configLoaded" });
      message.success("设置已保存");
    } catch (error) {
      if (!(error && typeof error === "object" && "errorFields" in error)) {
        message.error("保存失败");
      }
    } finally {
      setSaving(false);
    }
  }

  const sourceLabel =
    probe?.source === "config" ? "手动配置" : probe?.source === "which" ? "自动 which" : null;

  return (
    <PageFrame title="设置">
      <Form
        form={form}
        initialValues={config}
        layout="vertical"
        style={{ maxWidth: 480 }}
        onFinish={() => {
          void handleSave();
        }}
      >
        <Form.Item label="CR 评分阈值" name="cr_score_threshold">
          <Input />
        </Form.Item>
        <Form.Item label="测试覆盖率 (%)" name="test_coverage_threshold">
          <InputNumber style={{ width: "100%" }} />
        </Form.Item>
        <Form.Item label="最大变更行数" name="max_diff_lines">
          <InputNumber style={{ width: "100%" }} />
        </Form.Item>
        <Form.Item label="Agent 超时 (ms)" name="agent_timeout_ms">
          <InputNumber style={{ width: "100%" }} />
        </Form.Item>
        <Form.Item label="最大重试次数" name="max_retries">
          <InputNumber style={{ width: "100%" }} />
        </Form.Item>
        <Form.Item label="数据目录" name="db_path">
          <Input />
        </Form.Item>
        <Form.Item
          extra="留空则用登录 shell 执行 which claude。填写时必须是绝对路径。"
          label="Claude 路径"
          name="claude_path"
        >
          <Input allowClear placeholder={probe?.resolvedPath ?? "/opt/homebrew/bin/claude"} />
        </Form.Item>
        <Form.Item>
          <Flex gap={token.marginXS} wrap="wrap">
            <Button htmlType="submit" loading={saving} type="primary">
              保存
            </Button>
            <Button
              loading={probing}
              onClick={() => {
                void runProbe(form.getFieldValue("claude_path"));
              }}
            >
              刷新 Claude 状态
            </Button>
          </Flex>
        </Form.Item>
      </Form>
      {probing && !probe ? (
        <Spin tip="正在探测 Claude CLI..." />
      ) : probe ? (
        <Alert
          description={
            <Flex vertical gap={token.marginXXS}>
              {probe.resolvedPath ? <Text>路径：{probe.resolvedPath}</Text> : null}
              {sourceLabel ? <Text type="secondary">来源：{sourceLabel}</Text> : null}
              {probe.version ? <Text>版本：{probe.version}</Text> : null}
              {probe.error ? <Text type="danger">{probe.error}</Text> : null}
            </Flex>
          }
          message={probe.ok ? "Claude CLI 可用" : "Claude CLI 不可用"}
          showIcon
          style={{ maxWidth: 480 }}
          type={probe.ok ? "success" : "error"}
        />
      ) : null}
      <div style={{ marginTop: token.marginLG, maxWidth: 960 }}>
        <PipelineStatsPanel />
      </div>
    </PageFrame>
  );
}
