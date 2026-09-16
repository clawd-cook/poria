import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { useStore } from "../state/store";
import { listPipelines } from "../lib/tauri";
import type { PipelineEvent, AuthStatus } from "../lib/types";

export function useTauriEvents() {
  const { dispatch } = useStore();

  useEffect(() => {
    const unlisten = Promise.all([
      listen("pipeline:list-changed", () => {
        listPipelines()
          .then((pipelines) => dispatch({ type: "hydrate", pipelines }))
          .catch(() => {});
      }),
      listen<{ id: string; status: string; currentStage: string }>(
        "pipeline:updated",
        (e) => {
          dispatch({ type: "pipelineUpdated", ...e.payload });
        },
      ),
      listen<PipelineEvent>("stage:progress", (e) => {
        dispatch({ type: "eventReceived", event: e.payload });
      }),
      listen<{
        pipelineId: string;
        stage: string;
        issueClass: string;
        detail: string;
      }>("human:request", (e) => {
        dispatch({ type: "humanRequest", ...e.payload });
      }),
      listen<{ running: boolean; error?: string }>("sidecar:status", (e) => {
        dispatch({ type: "sidecarStatus", ...e.payload });
      }),
      listen<AuthStatus>("auth:status-changed", (e) => {
        dispatch({ type: "authChanged", auth: e.payload });
      }),
    ]);

    return () => {
      unlisten.then((fns) => fns.forEach((fn) => fn()));
    };
  }, [dispatch]);
}
