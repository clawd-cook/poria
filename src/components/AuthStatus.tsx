import { startLogin, logout } from "@/lib/tauri";
import { useStore } from "@/state/store";

import { Button } from "./ui/button";

export function AuthStatus() {
  const { state } = useStore();
  const { cookie_valid, logged_in, username } = state.auth;

  if (!logged_in) {
    return (
      <div className="flex items-center gap-2 px-4 py-3">
        <span aria-hidden className="bg-destructive size-2 shrink-0 rounded-full" />
        <Button onClick={() => void startLogin()} size="sm" variant="link">
          登录
        </Button>
      </div>
    );
  }

  if (!cookie_valid) {
    return (
      <div className="flex items-center gap-2 px-4 py-3">
        <span aria-hidden className="bg-warning size-2 shrink-0 rounded-full" />
        <span className="text-warning truncate text-sm">{username}</span>
        <Button onClick={() => void startLogin()} size="sm" variant="link">
          重新登录
        </Button>
      </div>
    );
  }

  return (
    <div className="flex items-center gap-2 px-4 py-3">
      <span aria-hidden className="bg-success size-2 shrink-0 rounded-full" />
      <span className="text-muted-foreground truncate text-sm">{username}</span>
      <Button onClick={() => void logout()} size="sm" variant="link">
        登出
      </Button>
    </div>
  );
}
