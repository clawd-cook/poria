import { Plug } from "lucide-react";
import { useEffect } from "react";

import { listChannels } from "@/lib/tauri";
import type { ChannelInfo } from "@/lib/types";
import { useStore } from "@/state/store";

import { EmptyState } from "./EmptyState";
import { PageFrame } from "./PageFrame";
import { Badge } from "./ui/badge";
import { Card, CardContent } from "./ui/card";

function ChannelCard({ channel }: { channel: ChannelInfo }) {
  return (
    <Card>
      <CardContent className="p-4">
        <div className="mb-2 flex items-start justify-between gap-2">
          <p className="flex items-center gap-2 font-semibold">
            <Plug aria-hidden className="text-success size-4" />
            {channel.name}
          </p>
          <div className="flex items-center gap-2">
            <span aria-hidden className="bg-success size-2 rounded-full" />
            <Badge variant="secondary">v{channel.version}</Badge>
          </div>
        </div>
        <p className="text-muted-foreground mb-1 text-sm">{channel.description || "暂无描述"}</p>
        <p className="text-muted-foreground text-xs">ID: {channel.id}</p>
      </CardContent>
    </Card>
  );
}

export function ChannelsPage() {
  const { dispatch, state } = useStore();

  useEffect(() => {
    listChannels()
      .then((channels) => dispatch({ channels, type: "channelsLoaded" }))
      .catch(() => {});
  }, [dispatch]);

  return (
    <PageFrame description={`已注册 ${state.channels.length} 个渠道`} title="渠道">
      {state.channels.length === 0 ? (
        <EmptyState description="暂无已注册的渠道" />
      ) : (
        <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
          {state.channels.map((channel) => (
            <ChannelCard channel={channel} key={channel.id} />
          ))}
        </div>
      )}
    </PageFrame>
  );
}
