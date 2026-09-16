import { useStore } from "../state/store";

export function AuthStatus() {
  const { state } = useStore();
  const { logged_in, username } = state.auth;

  return (
    <div className="flex items-center gap-2 px-4 py-2 text-sm">
      <span
        className={`h-2 w-2 rounded-full ${logged_in ? "bg-emerald-400" : "bg-red-400"}`}
      />
      <span className="text-slate-400">
        {logged_in ? username : "未登录"}
      </span>
    </div>
  );
}
