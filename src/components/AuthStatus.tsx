import { Badge, Typography, theme } from "antd";

import { logout, startLogin } from "../lib/tauri";
import { useStore } from "../state/store";

const { Link, Text } = Typography;

export function AuthStatus() {
  const { state } = useStore();
  const { token } = theme.useToken();
  const { cookie_valid, logged_in, username } = state.auth;
  const pad = { padding: `${token.paddingSM}px ${token.paddingMD}px` };

  if (!logged_in) {
    return (
      <div style={{ alignItems: "center", display: "flex", gap: token.marginSM, ...pad }}>
        <Badge status="error" />
        <Link onClick={() => void startLogin()}>登录</Link>
      </div>
    );
  }

  if (!cookie_valid) {
    return (
      <div style={{ alignItems: "center", display: "flex", gap: token.marginSM, ...pad }}>
        <Badge status="warning" />
        <Text type="warning">{username}</Text>
        <Link onClick={() => void startLogin()}>重新登录</Link>
      </div>
    );
  }

  return (
    <div style={{ alignItems: "center", display: "flex", gap: token.marginSM, ...pad }}>
      <Badge status="success" />
      <Text type="secondary">{username}</Text>
      <Link type="secondary" onClick={() => void logout()}>
        登出
      </Link>
    </div>
  );
}
