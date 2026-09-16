import { useEffect } from "react";
import { Radio } from "lucide-react";
import { useStore } from "../state/store";
import { listChannels } from "../lib/tauri";
import type { ChannelInfo } from "../lib/types";

function ChannelCard({ channel }: { channel: ChannelInfo }) {
  return (
    <div className="rounded-lg border border-slate-700 bg-slate-800/50 p-4 transition-colors hover:bg-slate-800">
      <div className="mb-2 flex items-start justify-between gap-2">
        <div className="flex items-center gap-2">
          <Radio className="h-4 w-4 shrink-0 text-emerald-400" />
          <span className="text-sm font-medium text-slate-200">
            {channel.name}
          </span>
        </div>
        <div className="flex items-center gap-2">
          <span className="h-2 w-2 rounded-full bg-emerald-400" />
          <span className="shrink-0 rounded bg-slate-700 px-1.5 py-0.5 text-xs text-slate-400">
            v{channel.version}
          </span>
        </div>
      </div>
      <p className="mb-3 text-xs leading-relaxed text-slate-400">
        {channel.description || "暂无描述"}
      </p>
      <div className="text-xs text-slate-500">
        ID: {channel.id}
      </div>
    </div>
  );
}

export function ChannelsPage() {
  const { state, dispatch } = useStore();

  useEffect(() => {
    listChannels()
      .then((channels) => dispatch({ type: "channelsLoaded", channels }))
      .catch(() => {});
  }, [dispatch]);

  return (
    <div className="h-full overflow-y-auto p-6">
      <div className="mb-6">
        <h2 className="text-xl font-semibold text-slate-100">渠道管理</h2>
        <p className="mt-1 text-sm text-slate-400">
          已注册 {state.channels.length} 个渠道
        </p>
      </div>

      {state.channels.length === 0 ? (
        <div className="flex h-64 items-center justify-center text-sm text-slate-500">
          暂无已注册的渠道
        </div>
      ) : (
        <div className="grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-3">
          {state.channels.map((channel) => (
            <ChannelCard key={channel.id} channel={channel} />
          ))}
        </div>
      )}
    </div>
  );
}
