import { SendOutlined } from "@ant-design/icons";
import { App, Button, Flex, Input } from "antd";
import { useState } from "react";

import { submitPipeline } from "../lib/tauri";
import { useStore } from "../state/store";

export function SubmitBar() {
  const { dispatch } = useStore();
  const { message } = App.useApp();
  const [link, setLink] = useState("");
  const [loading, setLoading] = useState(false);

  async function handleSubmit() {
    const trimmed = link.trim();
    if (!trimmed) return;

    setLoading(true);
    try {
      await submitPipeline(trimmed);
      setLink("");
      dispatch({ type: "filterChanged", filter: null });
    } catch (err) {
      message.error(typeof err === "string" ? err : "提交失败");
    } finally {
      setLoading(false);
    }
  }

  return (
    <Flex align="center" gap={8} style={{ padding: "8px 16px" }}>
      <Input
        value={link}
        onChange={(e) => setLink(e.target.value)}
        placeholder="粘贴行云卡片链接..."
        disabled={loading}
        onPressEnter={handleSubmit}
        style={{ flex: 1 }}
      />
      <Button
        type="primary"
        icon={<SendOutlined />}
        loading={loading}
        disabled={!link.trim()}
        onClick={handleSubmit}
      >
        提交
      </Button>
    </Flex>
  );
}
