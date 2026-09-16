import { useEffect } from "react";
import { useStore } from "../state/store";
import { getPipeline } from "../lib/tauri";

export function usePipeline() {
  const { state, dispatch } = useStore();

  useEffect(() => {
    if (!state.selectedPipelineId) return;

    getPipeline(state.selectedPipelineId)
      .then((detail) => dispatch({ type: "detailLoaded", detail }))
      .catch(() => {});
  }, [state.selectedPipelineId, dispatch]);

  return {
    detail: state.pipelineDetail,
    events: state.events,
    humanRequest: state.humanRequest,
  };
}
