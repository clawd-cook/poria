import { RefreshCcw, RotateCcw } from "lucide-react";
import { useEffect, useState } from "react";
import { toast } from "sonner";

import { invokeErrorMessage } from "@/lib/errors";
import { registerRepo, retryClone, syncRepo, updateRepoDefaultBranch } from "@/lib/tauri";
import type { CloneStatus, RegisteredRepo, RepoSyncStatus } from "@/lib/types";
import { useStore } from "@/state/store";

import { EmptyState } from "./EmptyState";
import { PageFrame } from "./PageFrame";
import { Badge } from "./ui/badge";
import { Button } from "./ui/button";
import { Input } from "./ui/input";

const STATUS_CONFIG: Record<
  CloneStatus,
  { label: string; variant: "default" | "destructive" | "success" }
> = {
  cloning: { label: "进行中", variant: "default" },
  failed: { label: "失败", variant: "destructive" },
  ready: { label: "成功", variant: "success" },
};

const SYNC_CONFIG: Record<
  RepoSyncStatus,
  { label: string; variant: "destructive" | "secondary" | "success" | "default" }
> = {
  failed: { label: "同步失败", variant: "destructive" },
  idle: { label: "未同步", variant: "secondary" },
  synced: { label: "已同步", variant: "success" },
  syncing: { label: "同步中", variant: "default" },
};

function groupReposByScope(repos: RegisteredRepo[]): { repos: RegisteredRepo[]; scope: string }[] {
  const grouped = new Map<string, RegisteredRepo[]>();
  for (const repo of repos) {
    const current = grouped.get(repo.scope);
    if (current) {
      current.push(repo);
    } else {
      grouped.set(repo.scope, [repo]);
    }
  }
  return [...grouped.entries()]
    .sort(([left], [right]) => left.localeCompare(right))
    .map(([scope, items]) => ({ repos: items, scope }));
}

function formatSyncTime(value: string | null): string {
  if (!value) {
    return "尚未同步";
  }
  const parsed = Date.parse(value);
  if (Number.isNaN(parsed)) {
    return value;
  }
  return new Date(parsed).toLocaleString();
}

async function copyText(value: string) {
  try {
    await navigator.clipboard.writeText(value);
    toast.success("已复制");
  } catch {
    toast.error("复制失败");
  }
}

export function RepoListPage() {
  const { state } = useStore();
  const [gitUrl, setGitUrl] = useState("");
  const [registering, setRegistering] = useState(false);
  const [retryingId, setRetryingId] = useState<string | null>(null);
  const [syncingId, setSyncingId] = useState<string | null>(null);
  const [savingBranchId, setSavingBranchId] = useState<string | null>(null);

  const groups = groupReposByScope(state.repos);

  async function handleRegister() {
    const trimmed = gitUrl.trim();
    if (!trimmed) {
      return;
    }

    setRegistering(true);
    try {
      await registerRepo(trimmed);
      setGitUrl("");
      toast.success("已开始克隆");
    } catch (error) {
      toast.error(invokeErrorMessage(error, "登记失败"));
    } finally {
      setRegistering(false);
    }
  }

  async function handleRetry(id: string) {
    setRetryingId(id);
    try {
      await retryClone(id);
      toast.success("已重新克隆");
    } catch (error) {
      toast.error(invokeErrorMessage(error, "重试失败"));
    } finally {
      setRetryingId(null);
    }
  }

  async function handleSync(id: string) {
    setSyncingId(id);
    try {
      const updated = await syncRepo(id);
      if (updated.sync_status === "failed") {
        toast.error(updated.sync_error || "同步失败");
      } else {
        toast.success("已同步主分支");
      }
    } catch (error) {
      toast.error(invokeErrorMessage(error, "同步失败"));
    } finally {
      setSyncingId(null);
    }
  }

  async function handleSaveBranch(id: string, defaultBranch: string) {
    const trimmed = defaultBranch.trim();
    if (!trimmed) {
      toast.error("主分支不能为空");
      return;
    }
    setSavingBranchId(id);
    try {
      const updated = await updateRepoDefaultBranch(id, trimmed);
      if (updated.sync_status === "failed") {
        toast.warning(updated.sync_error || "主分支已保存，但同步失败");
      } else {
        toast.success("已保存主分支并同步");
      }
    } catch (error) {
      toast.error(invokeErrorMessage(error, "保存主分支失败"));
    } finally {
      setSavingBranchId(null);
    }
  }

  return (
    <PageFrame description="登记 git URL 后会克隆到 ~/.poria/repos/scope/name" title="仓库">
      <form
        className="mb-6 flex max-w-3xl gap-2"
        onSubmit={(event) => {
          event.preventDefault();
          void handleRegister();
        }}
      >
        <Input
          aria-label="Git URL"
          disabled={registering}
          onChange={(event) => setGitUrl(event.target.value)}
          placeholder="git@coding.jd.com:ls/ls-entrance.git"
          value={gitUrl}
        />
        <Button disabled={!gitUrl.trim()} loading={registering} type="submit">
          登记
        </Button>
      </form>

      {groups.length === 0 ? (
        <EmptyState description="暂无已登记仓库" />
      ) : (
        <div className="grid gap-4">
          {groups.map((group) => (
            <div key={group.scope}>
              <p className="mb-2 font-semibold">{group.scope}</p>
              <div className="grid gap-2">
                {group.repos.map((repo) => (
                  <RepoRow
                    key={repo.id}
                    onRetry={() => {
                      void handleRetry(repo.id);
                    }}
                    onSaveBranch={(branch) => {
                      void handleSaveBranch(repo.id, branch);
                    }}
                    onSync={() => {
                      void handleSync(repo.id);
                    }}
                    repo={repo}
                    retrying={retryingId === repo.id}
                    savingBranch={savingBranchId === repo.id}
                    syncing={syncingId === repo.id || repo.sync_status === "syncing"}
                  />
                ))}
              </div>
            </div>
          ))}
        </div>
      )}
    </PageFrame>
  );
}

function RepoRow({
  onRetry,
  onSaveBranch,
  onSync,
  repo,
  retrying,
  savingBranch,
  syncing,
}: {
  onRetry: () => void;
  onSaveBranch: (branch: string) => void;
  onSync: () => void;
  repo: RegisteredRepo;
  retrying: boolean;
  savingBranch: boolean;
  syncing: boolean;
}) {
  const status = STATUS_CONFIG[repo.clone_status];
  const sync = SYNC_CONFIG[repo.sync_status];
  const [branch, setBranch] = useState(repo.default_branch || "master");
  useEffect(() => {
    setBranch(repo.default_branch || "master");
  }, [repo.default_branch]);
  const branchDirty = branch.trim() !== (repo.default_branch || "master");
  const ready = repo.clone_status === "ready";

  return (
    <div className="border-border bg-card flex items-start justify-between gap-3 rounded-lg border p-4">
      <div className="grid min-w-0 gap-1">
        <div className="flex flex-wrap items-center gap-2">
          <p className="truncate font-semibold">{repo.name}</p>
          <Badge variant={status.variant}>{status.label}</Badge>
          <Badge variant={sync.variant}>{sync.label}</Badge>
        </div>
        <button
          className="text-muted-foreground hover:text-foreground cursor-pointer truncate text-left text-sm"
          onClick={() => void copyText(repo.git_url)}
          type="button"
        >
          {repo.git_url}
        </button>
        <p className="text-muted-foreground truncate text-sm">{repo.local_path}</p>
        <p className="text-muted-foreground text-sm">
          最近同步：{formatSyncTime(repo.last_synced_at)}
        </p>
        <div className="flex items-center gap-2">
          <span className="text-sm">主分支</span>
          <Input
            aria-label="主分支"
            className="max-w-44"
            disabled={!ready || savingBranch || syncing}
            onChange={(event) => setBranch(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter" && ready && branchDirty) {
                onSaveBranch(branch);
              }
            }}
            value={branch}
          />
          <Button
            disabled={!ready || !branchDirty || !branch.trim()}
            loading={savingBranch}
            onClick={() => onSaveBranch(branch)}
            size="sm"
            variant="outline"
          >
            保存
          </Button>
        </div>
        {repo.clone_status === "failed" && repo.error ? (
          <p className="text-destructive text-sm">{repo.error}</p>
        ) : null}
        {repo.sync_status === "failed" && repo.sync_error ? (
          <p className="text-destructive text-sm">{repo.sync_error}</p>
        ) : null}
      </div>
      <div className="flex gap-2">
        {ready ? (
          <Button loading={syncing} onClick={onSync} size="sm" variant="outline">
            <RefreshCcw aria-hidden />
            同步
          </Button>
        ) : null}
        {repo.clone_status === "failed" ? (
          <Button loading={retrying} onClick={onRetry} size="sm" variant="outline">
            <RotateCcw aria-hidden />
            重试
          </Button>
        ) : null}
      </div>
    </div>
  );
}
