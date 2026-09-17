import { App, Button, Form, Input, InputNumber, Spin, Typography } from "antd";
import { useEffect, useState } from "react";

import { updateConfig } from "../lib/tauri";
import type { AppConfig } from "../lib/types";
import { useStore } from "../state/store";

const { Title } = Typography;

export function SettingsPage() {
  const { state, dispatch } = useStore();
  const { message } = App.useApp();
  const [saving, setSaving] = useState(false);
  const [form] = Form.useForm<AppConfig>();
  const config = state.config;

  useEffect(() => {
    if (config) {
      form.setFieldsValue(config);
    }
  }, [config, form]);

  if (!config) {
    return (
      <div
        style={{ display: "flex", height: "100%", alignItems: "center", justifyContent: "center" }}
      >
        <Spin tip="加载设置..." />
      </div>
    );
  }

  async function handleSave() {
    try {
      const values = await form.validateFields();
      setSaving(true);
      await updateConfig(values);
      dispatch({ type: "configLoaded", config: values });
      message.success("设置已保存");
    } catch (error) {
      if (!(error && typeof error === "object" && "errorFields" in error)) {
        message.error("保存失败");
      }
    } finally {
      setSaving(false);
    }
  }

  return (
    <div style={{ height: "100%", overflow: "auto", padding: 24 }}>
      <Title level={4}>设置</Title>
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
        <Form.Item>
          <Button htmlType="submit" loading={saving} type="primary">
            保存
          </Button>
        </Form.Item>
      </Form>
    </div>
  );
}
