import { Badge, Typography } from "antd";

import { useStore } from "../state/store";

const { Text } = Typography;

export function AuthStatus() {
  const { state } = useStore();
  const { logged_in, username } = state.auth;

  return (
    <div style={{ display: "flex", alignItems: "center", gap: 8, padding: "8px 16px" }}>
      <Badge status={logged_in ? "success" : "error"} />
      <Text type="secondary" style={{ fontSize: 13 }}>
        {logged_in ? username : "未登录"}
      </Text>
    </div>
  );
}
