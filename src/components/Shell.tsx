import { FolderGit2, Home, Settings } from "lucide-react";
import type { ReactNode } from "react";

import { countAttentionPipelines } from "@/lib/pipelineViewModel";
import type { ViewType } from "@/lib/types";
import { cn } from "@/lib/utils";
import { useStore } from "@/state/store";

import { AuthStatus } from "./AuthStatus";
import { HomeBoard } from "./HomeBoard";
import { RepoListPage } from "./RepoListPage";
import { SettingsPage } from "./SettingsPage";

const VIEW_KEYS: ViewType[] = ["home", "repos", "settings"];

const PRIMARY_NAV = [{ icon: Home, key: "home" as const, label: "需求" }];

const RESOURCE_NAV = [{ icon: FolderGit2, key: "repos" as const, label: "仓库" }];

function isViewType(key: string): key is ViewType {
  return VIEW_KEYS.includes(key as ViewType);
}

function PersistentTab({ active, children }: { active: boolean; children: ReactNode }) {
  return (
    <div
      aria-hidden={!active}
      className="absolute inset-0 overflow-auto"
      hidden={!active}
      inert={!active}
    >
      {children}
    </div>
  );
}

function NavButton({
  active,
  badge,
  icon: Icon,
  label,
  onSelect,
}: {
  active: boolean;
  badge?: number;
  icon: typeof Home;
  label: string;
  onSelect: () => void;
}) {
  return (
    <button
      aria-current={active ? "page" : undefined}
      className={cn(
        "flex h-10 w-full cursor-pointer items-center gap-2 rounded-md px-3 text-sm transition-colors duration-200 focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none",
        active ? "text-primary bg-primary/15 font-medium" : "text-foreground hover:bg-black/5",
      )}
      onClick={onSelect}
      type="button"
    >
      <Icon aria-hidden className="size-4 shrink-0" />
      <span className="min-w-0 flex-1 text-left">{label}</span>
      {badge != null && badge > 0 ? (
        <span className="bg-primary text-primary-foreground rounded-md px-1.5 py-0.5 text-[10px] font-medium tabular-nums">
          {badge}
        </span>
      ) : null}
    </button>
  );
}

export function Shell() {
  const { dispatch, state } = useStore();
  const currentView = state.ui.view;
  const attentionCount = countAttentionPipelines(
    state.pipelines,
    state.humanRequest?.pipelineId ?? null,
  );

  function selectView(key: string) {
    if (isViewType(key)) {
      dispatch({ type: "viewChanged", view: key });
    }
  }

  return (
    <div className="bg-background flex h-screen overflow-hidden">
      <a
        className="focus:bg-card focus:ring-ring sr-only focus:not-sr-only focus:absolute focus:z-50 focus:m-3 focus:rounded-md focus:px-3 focus:py-2 focus:ring-2"
        href="#main-content"
      >
        跳到主内容
      </a>
      <aside className="border-border bg-sidebar flex h-full w-56 shrink-0 flex-col border-r">
        <div className="border-border border-b px-5 py-4">
          <p className="font-display text-sm font-bold tracking-[0.22em] uppercase">Poria</p>
          <p className="text-muted-foreground mt-1 text-xs">需求到合并请求</p>
        </div>
        <nav aria-label="主导航" className="flex min-h-0 flex-1 flex-col gap-1 overflow-auto p-3">
          {PRIMARY_NAV.map((item) => (
            <NavButton
              active={currentView === item.key}
              badge={item.key === "home" ? attentionCount : undefined}
              icon={item.icon}
              key={item.key}
              label={item.label}
              onSelect={() => selectView(item.key)}
            />
          ))}
          <p className="text-muted-foreground mt-4 mb-1 px-3 text-xs tracking-wide uppercase">
            资源
          </p>
          {RESOURCE_NAV.map((item) => (
            <NavButton
              active={currentView === item.key}
              icon={item.icon}
              key={item.key}
              label={item.label}
              onSelect={() => selectView(item.key)}
            />
          ))}
          <div className="mt-auto pt-3">
            <NavButton
              active={currentView === "settings"}
              icon={Settings}
              label="设置"
              onSelect={() => selectView("settings")}
            />
          </div>
        </nav>
        <div className="border-border border-t">
          <AuthStatus />
        </div>
      </aside>
      <main className="relative min-h-0 flex-1 overflow-hidden" id="main-content">
        <PersistentTab active={currentView === "home"}>
          <HomeBoard />
        </PersistentTab>
        <PersistentTab active={currentView === "repos"}>
          <RepoListPage />
        </PersistentTab>
        <PersistentTab active={currentView === "settings"}>
          <SettingsPage />
        </PersistentTab>
      </main>
    </div>
  );
}
