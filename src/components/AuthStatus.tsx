import { Badge, Typography, theme } from "antd";

import { logout, startLogin } from "../lib/tauri";
import { useStore } from "../state/store";

const { Link, Text } = Typography;

export function AuthStatus() {
  const { state } = useStore();
  const { token } = theme.useToken();
  const { logged_in, username } = state.auth;
  const pad = { padding: `${token.paddingSM}px ${token.paddingMD}px` };

  if (!logged_in) {
    return (
      <div style={{ alignItems: "center", display: "flex", gap: token.marginSM, ...pad }}>
        <Badge status="error" />
        <Link onClick={() => startLogin()}>登录</Link>
      </div>
    );
  }

  return (
    <div style={{ alignItems: "center", display: "flex", gap: token.marginSM, ...pad }}>
      <Badge status="success" />
      <Text type="secondary">{username}</Text>
      <Link type="secondary" onClick={() => logout()}>
        登出
      </Link>
    </div>
  );
}
