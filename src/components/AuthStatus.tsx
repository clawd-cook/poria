import { Badge, Typography } from "antd";

import { logout, startLogin } from "../lib/tauri";
import { useStore } from "../state/store";

const { Text, Link } = Typography;

export function AuthStatus() {
  const { state } = useStore();
  const { logged_in, username } = state.auth;

  if (!logged_in) {
    return (
      <div style={{ display: "flex", alignItems: "center", gap: 8, padding: "8px 16px" }}>
        <Badge status="error" />
        <Link style={{ fontSize: 13 }} onClick={() => startLogin()}>
          登录
        </Link>
      </div>
    );
  }

  return (
    <div style={{ display: "flex", alignItems: "center", gap: 8, padding: "8px 16px" }}>
      <Badge status="success" />
      <Text type="secondary" style={{ fontSize: 13 }}>
        {username}
      </Text>
      <Link type="secondary" style={{ fontSize: 12, marginLeft: 4 }} onClick={() => logout()}>
        登出
      </Link>
    </div>
  );
}
