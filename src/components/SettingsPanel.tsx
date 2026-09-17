import { Form, Input, InputNumber, Modal } from "antd";
import { useEffect, useState } from "react";

import { updateConfig } from "../lib/tauri";
import type { AppConfig } from "../lib/types";
import { useStore } from "../state/store";

export function SettingsPanel() {
  const { state, dispatch } = useStore();
  const [saving, setSaving] = useState(false);
  const [form] = Form.useForm<AppConfig>();

  const config = state.config;
  const open = state.ui.settingsOpen;

  useEffect(() => {
    if (config && open) {
      form.setFieldsValue(config);
    }
  }, [config, open, form]);

  if (!config) return null;

  function handleClose() {
    form.resetFields();
    dispatch({ type: "settingsToggled", open: false });
  }

  async function handleSave() {
    try {
      const values = await form.validateFields();
      setSaving(true);
      await updateConfig(values);
      dispatch({ type: "configLoaded", config: values });
      dispatch({ type: "settingsToggled", open: false });
    } catch {
      /* validation error */
    } finally {
      setSaving(false);
    }
  }

  return (
    <Modal
      title="设置"
      open={open}
      onCancel={handleClose}
      onOk={handleSave}
      okText="保存"
      cancelText="取消"
      confirmLoading={saving}
      destroyOnClose
    >
      <Form form={form} layout="vertical" initialValues={config}>
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
      </Form>
    </Modal>
  );
}
