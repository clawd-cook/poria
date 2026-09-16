import { Settings } from "lucide-react";
import { useStore } from "../state/store";
import { SubmitBar } from "./SubmitBar";
import { PipelineSidebar } from "./PipelineSidebar";
import { PipelineDetail } from "./PipelineDetail";
import { AuthStatus } from "./AuthStatus";
import { SettingsPanel } from "./SettingsPanel";

export function Shell() {
  const { state, dispatch } = useStore();

  return (
    <div className="flex h-screen flex-col bg-slate-900 text-slate-200">
      <div className="border-b border-slate-700">
        <SubmitBar />
      </div>

      <div className="flex min-h-0 flex-1">
        <aside className="flex w-80 flex-col border-r border-slate-700 bg-slate-800">
          <div className="min-h-0 flex-1">
            <PipelineSidebar />
          </div>
          <div className="border-t border-slate-700">
            <div className="flex items-center justify-between">
              <AuthStatus />
              <div className="flex items-center gap-1 pr-2">
                {state.sidecar.running && (
                  <span className="h-2 w-2 rounded-full bg-emerald-400" />
                )}
                {state.sidecar.error && (
                  <span className="text-xs text-red-400">
                    {state.sidecar.error}
                  </span>
                )}
                <button
                  onClick={() =>
                    dispatch({ type: "settingsToggled", open: true })
                  }
                  className="rounded p-1 text-slate-400 transition-colors hover:bg-slate-700 hover:text-slate-200"
                >
                  <Settings className="h-4 w-4" />
                </button>
              </div>
            </div>
          </div>
        </aside>

        <main className="min-h-0 flex-1">
          <PipelineDetail />
        </main>
      </div>

      {state.ui.settingsOpen && <SettingsPanel />}
    </div>
  );
}
