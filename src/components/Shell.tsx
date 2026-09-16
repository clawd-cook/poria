import { Settings, GitBranch, Puzzle, Radio } from "lucide-react";

import type { ViewType } from "../lib/types";
import { useStore } from "../state/store";
import { AuthStatus } from "./AuthStatus";
import { ChannelsPage } from "./ChannelsPage";
import { PipelineDetail } from "./PipelineDetail";
import { PipelineSidebar } from "./PipelineSidebar";
import { SettingsPanel } from "./SettingsPanel";
import { SkillsPage } from "./SkillsPage";
import { SubmitBar } from "./SubmitBar";

const NAV_ITEMS: { key: ViewType; label: string; icon: typeof GitBranch }[] = [
  { key: "pipeline", label: "Pipeline", icon: GitBranch },
  { key: "skills", label: "技能", icon: Puzzle },
  { key: "channels", label: "渠道", icon: Radio },
];

export function Shell() {
  const { state, dispatch } = useStore();
  const currentView = state.ui.view;

  return (
    <div className="flex h-screen flex-col bg-slate-900 text-slate-200">
      <div className="flex items-center border-b border-slate-700">
        <nav className="flex items-center gap-1 px-4 py-2">
          {NAV_ITEMS.map((item) => {
            const Icon = item.icon;
            const active = currentView === item.key;
            return (
              <button
                key={item.key}
                onClick={() => dispatch({ type: "viewChanged", view: item.key })}
                className={`flex items-center gap-1.5 rounded px-3 py-1.5 text-sm transition-colors ${
                  active
                    ? "bg-slate-700 text-slate-200"
                    : "text-slate-400 hover:bg-slate-800 hover:text-slate-300"
                }`}
              >
                <Icon className="h-4 w-4" />
                {item.label}
              </button>
            );
          })}
        </nav>
        {currentView === "pipeline" && (
          <div className="flex-1">
            <SubmitBar />
          </div>
        )}
      </div>

      {currentView === "pipeline" && (
        <div className="flex min-h-0 flex-1">
          <aside className="flex w-80 flex-col border-r border-slate-700 bg-slate-800">
            <div className="min-h-0 flex-1">
              <PipelineSidebar />
            </div>
            <div className="border-t border-slate-700">
              <div className="flex items-center justify-between">
                <AuthStatus />
                <div className="flex items-center gap-1 pr-2">
                  <button
                    onClick={() => dispatch({ type: "settingsToggled", open: true })}
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
      )}

      {currentView === "skills" && (
        <div className="min-h-0 flex-1">
          <SkillsPage />
        </div>
      )}

      {currentView === "channels" && (
        <div className="min-h-0 flex-1">
          <ChannelsPage />
        </div>
      )}

      {state.ui.settingsOpen && <SettingsPanel />}
    </div>
  );
}
