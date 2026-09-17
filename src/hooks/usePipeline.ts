import { listen } from "@tauri-apps/api/event";
import { useEffect } from "react";

import { getPipeline } from "../lib/tauri";
import { useStore } from "../state/store";

export function usePipeline() {
  const { state, dispatch } = useStore();
  const selectedId = state.selectedPipelineId;

  useEffect(() => {
    if (!selectedId) {
      return;
    }
    const pipelineId = selectedId;

    let cancelled = false;

    function load() {
      getPipeline(pipelineId)
        .then((detail) => {
          if (!cancelled) {
            dispatch({ type: "detailLoaded", detail });
          }
        })
        .catch(() => {});
    }

    load();

    const unlisten = Promise.all([
      listen<{ id: string }>("pipeline:updated", (event) => {
        if (event.payload.id === pipelineId) {
          load();
        }
      }),
      listen("pipeline:list-changed", load),
    ]);

    return () => {
      cancelled = true;
      unlisten.then((fns) => fns.forEach((fn) => fn()));
    };
  }, [dispatch, selectedId]);

  return {
    detail: state.pipelineDetail,
    events: state.events,
    humanRequest: state.humanRequest,
  };
}
