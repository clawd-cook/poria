import { useState } from "react";
import { Send } from "lucide-react";
import { useStore } from "../state/store";
import { submitPipeline } from "../lib/tauri";

export function SubmitBar() {
  const { dispatch } = useStore();
  const [link, setLink] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    const trimmed = link.trim();
    if (!trimmed) return;

    setLoading(true);
    setError(null);
    try {
      await submitPipeline(trimmed);
      setLink("");
      dispatch({ type: "filterChanged", filter: null });
    } catch (err) {
      setError(typeof err === "string" ? err : "提交失败");
    } finally {
      setLoading(false);
    }
  }

  return (
    <form onSubmit={handleSubmit} className="flex items-center gap-2 p-4">
      <input
        type="text"
        value={link}
        onChange={(e) => setLink(e.target.value)}
        placeholder="粘贴行云卡片链接..."
        className="flex-1 rounded-lg border border-slate-600 bg-slate-800 px-4 py-2 text-sm text-slate-200 placeholder-slate-500 outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500"
        disabled={loading}
      />
      <button
        type="submit"
        disabled={loading || !link.trim()}
        className="flex items-center gap-1.5 rounded-lg bg-blue-600 px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-blue-500 disabled:cursor-not-allowed disabled:opacity-50"
      >
        <Send className="h-4 w-4" />
        提交
      </button>
      {error && <span className="text-xs text-red-400">{error}</span>}
    </form>
  );
}
