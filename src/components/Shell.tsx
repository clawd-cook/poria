import { FolderGit2, Home, Plug, Settings, Sparkles, SquareTerminal } from "lucide-react";
import type { ReactNode } from "react";

import type { ViewType } from "@/lib/types";
import { cn } from "@/lib/utils";
import { useStore } from "@/state/store";

import { AuthStatus } from "./AuthStatus";
import { ChannelsPage } from "./ChannelsPage";
import { HomeBoard } from "./HomeBoard";
import { RepoListPage } from "./RepoListPage";
import { SettingsPage } from "./SettingsPage";
import { SkillsPage } from "./SkillsPage";
import { WorkspacePage } from "./WorkspacePage";

const VIEW_KEYS: ViewType[] = ["channels", "home", "repos", "settings", "skills", "workspace"];

const PRIMARY_NAV = [{ icon: Home, key: "home" as const, label: "看板" }];

const RESOURCE_NAV = [
  { icon: Plug, key: "channels" as const, label: "渠道" },
  { icon: Sparkles, key: "skills" as const, label: "技能" },
  { icon: FolderGit2, key: "repos" as const, label: "仓库" },
  { icon: SquareTerminal, key: "workspace" as const, label: "工作区" },
];

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
  icon: Icon,
  label,
  onSelect,
}: {
  active: boolean;
  icon: typeof Home;
  label: string;
  onSelect: () => void;
}) {
  return (
    <button
      aria-current={active ? "page" : undefined}
      className={cn(
        "flex h-10 w-full cursor-pointer items-center gap-2 rounded-md px-3 text-sm transition-colors duration-200 focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none",
        active
          ? "text-primary bg-primary/15 font-medium"
          : "text-foreground hover:bg-black/5",
      )}
      onClick={onSelect}
      type="button"
    >
      <Icon aria-hidden className="size-4 shrink-0" />
      {label}
    </button>
  );
}

export function Shell() {
  const { dispatch, state } = useStore();
  const currentView = state.ui.view === "demands" ? "home" : state.ui.view;

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
        <PersistentTab active={currentView === "channels"}>
          <ChannelsPage />
        </PersistentTab>
        <PersistentTab active={currentView === "skills"}>
          <SkillsPage />
        </PersistentTab>
        <PersistentTab active={currentView === "repos"}>
          <RepoListPage />
        </PersistentTab>
        <PersistentTab active={currentView === "workspace"}>
          <WorkspacePage />
        </PersistentTab>
        <PersistentTab active={currentView === "settings"}>
          <SettingsPage />
        </PersistentTab>
      </main>
    </div>
  );
}
