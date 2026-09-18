import { ReloadOutlined, SyncOutlined } from "@ant-design/icons";
import { App, Button, Empty, Flex, Input, Tag, Typography } from "antd";
import { useEffect, useState } from "react";

import { invokeErrorMessage } from "../lib/errors";
import { registerRepo, retryClone, syncRepo, updateRepoDefaultBranch } from "../lib/tauri";
import type { CloneStatus, RegisteredRepo, RepoSyncStatus } from "../lib/types";
import { useStore } from "../state/store";

const { Text, Title } = Typography;

const STATUS_CONFIG: Record<CloneStatus, { color: string; label: string }> = {
  cloning: { color: "processing", label: "进行中" },
  failed: { color: "error", label: "失败" },
  ready: { color: "success", label: "成功" },
};

const SYNC_CONFIG: Record<RepoSyncStatus, { color: string; label: string }> = {
  failed: { color: "error", label: "同步失败" },
  idle: { color: "default", label: "未同步" },
  synced: { color: "success", label: "已同步" },
  syncing: { color: "processing", label: "同步中" },
};

function groupReposByScope(repos: RegisteredRepo[]): { scope: string; repos: RegisteredRepo[] }[] {
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

export function RepoListPage() {
  const { state } = useStore();
  const { message } = App.useApp();
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
      message.success("已开始克隆");
    } catch (error) {
      message.error(invokeErrorMessage(error, "登记失败"));
    } finally {
      setRegistering(false);
    }
  }

  async function handleRetry(id: string) {
    setRetryingId(id);
    try {
      await retryClone(id);
      message.success("已重新克隆");
    } catch (error) {
      message.error(invokeErrorMessage(error, "重试失败"));
    } finally {
      setRetryingId(null);
    }
  }

  async function handleSync(id: string) {
    setSyncingId(id);
    try {
      const updated = await syncRepo(id);
      if (updated.sync_status === "failed") {
        message.error(updated.sync_error || "同步失败");
      } else {
        message.success("已同步主分支");
      }
    } catch (error) {
      message.error(invokeErrorMessage(error, "同步失败"));
    } finally {
      setSyncingId(null);
    }
  }

  async function handleSaveBranch(id: string, defaultBranch: string) {
    const trimmed = defaultBranch.trim();
    if (!trimmed) {
      message.error("主分支不能为空");
      return;
    }
    setSavingBranchId(id);
    try {
      const updated = await updateRepoDefaultBranch(id, trimmed);
      if (updated.sync_status === "failed") {
        message.warning(updated.sync_error || "主分支已保存，但同步失败");
      } else {
        message.success("已保存主分支并同步");
      }
    } catch (error) {
      message.error(invokeErrorMessage(error, "保存主分支失败"));
    } finally {
      setSavingBranchId(null);
    }
  }

  return (
    <div style={{ height: "100%", overflow: "auto", padding: 24 }}>
      <Title level={4}>仓库列表</Title>
      <Flex gap={8} style={{ marginBottom: 24, maxWidth: 720 }}>
        <Input
          disabled={registering}
          onChange={(event) => setGitUrl(event.target.value)}
          onPressEnter={() => {
            void handleRegister();
          }}
          placeholder="git@coding.jd.com:ls/ls-entrance.git"
          value={gitUrl}
        />
        <Button
          disabled={!gitUrl.trim()}
          loading={registering}
          onClick={() => {
            void handleRegister();
          }}
          type="primary"
        >
          登记
        </Button>
      </Flex>

      {groups.length === 0 ? (
        <Empty description="登记 git URL 后会克隆到 ~/.poria/repos/scope/name" />
      ) : (
        <Flex gap={16} vertical>
          {groups.map((group) => (
            <div key={group.scope}>
              <Text strong style={{ display: "block", marginBottom: 8 }}>
                {group.scope}
              </Text>
              <Flex gap={8} vertical>
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
              </Flex>
            </div>
          ))}
        </Flex>
      )}
    </div>
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
    <Flex
      align="flex-start"
      gap={12}
      justify="space-between"
      style={{
        border: "1px solid var(--ant-color-border, #303030)",
        borderRadius: 8,
        padding: 12,
      }}
    >
      <Flex gap={4} style={{ minWidth: 0 }} vertical>
        <Flex align="center" gap={8} wrap>
          <Text ellipsis strong>
            {repo.name}
          </Text>
          <Tag color={status.color} style={{ margin: 0 }}>
            {status.label}
          </Tag>
          <Tag color={sync.color} style={{ margin: 0 }}>
            {sync.label}
          </Tag>
        </Flex>
        <Text copyable ellipsis type="secondary">
          {repo.git_url}
        </Text>
        <Text ellipsis type="secondary">
          {repo.local_path}
        </Text>
        <Text type="secondary">最近同步：{formatSyncTime(repo.last_synced_at)}</Text>
        <Flex align="center" gap={8}>
          <Text style={{ flex: "0 0 auto" }}>主分支</Text>
          <Input
            aria-label="主分支"
            disabled={!ready || savingBranch || syncing}
            onChange={(event) => setBranch(event.target.value)}
            onPressEnter={() => {
              if (ready && branchDirty) {
                onSaveBranch(branch);
              }
            }}
            size="small"
            style={{ maxWidth: 180 }}
            value={branch}
          />
          <Button
            disabled={!ready || !branchDirty || !branch.trim()}
            loading={savingBranch}
            onClick={() => {
              onSaveBranch(branch);
            }}
            size="small"
          >
            保存
          </Button>
        </Flex>
        {repo.clone_status === "failed" && repo.error ? (
          <Text type="danger">{repo.error}</Text>
        ) : null}
        {repo.sync_status === "failed" && repo.sync_error ? (
          <Text type="danger">{repo.sync_error}</Text>
        ) : null}
      </Flex>
      <Flex gap={8}>
        {ready ? (
          <Button icon={<SyncOutlined />} loading={syncing} onClick={onSync} size="small">
            同步
          </Button>
        ) : null}
        {repo.clone_status === "failed" ? (
          <Button icon={<ReloadOutlined />} loading={retrying} onClick={onRetry} size="small">
            重试
          </Button>
        ) : null}
      </Flex>
    </Flex>
  );
}
