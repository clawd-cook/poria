import { ReloadOutlined } from "@ant-design/icons";
import { App, Button, Empty, Flex, Input, Tag, Typography } from "antd";
import { useState } from "react";

import { invokeErrorMessage } from "../lib/errors";
import { registerRepo, retryClone } from "../lib/tauri";
import type { CloneStatus, RegisteredRepo } from "../lib/types";
import { useStore } from "../state/store";

const { Text, Title } = Typography;

const STATUS_CONFIG: Record<CloneStatus, { color: string; label: string }> = {
  cloning: { color: "processing", label: "进行中" },
  failed: { color: "error", label: "失败" },
  ready: { color: "success", label: "成功" },
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

export function RepoListPage() {
  const { state } = useStore();
  const { message } = App.useApp();
  const [gitUrl, setGitUrl] = useState("");
  const [registering, setRegistering] = useState(false);
  const [retryingId, setRetryingId] = useState<string | null>(null);

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
                    repo={repo}
                    retrying={retryingId === repo.id}
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
  repo,
  retrying,
}: {
  onRetry: () => void;
  repo: RegisteredRepo;
  retrying: boolean;
}) {
  const status = STATUS_CONFIG[repo.clone_status];

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
        <Flex align="center" gap={8}>
          <Text ellipsis strong>
            {repo.name}
          </Text>
          <Tag color={status.color} style={{ margin: 0 }}>
            {status.label}
          </Tag>
        </Flex>
        <Text copyable ellipsis type="secondary">
          {repo.git_url}
        </Text>
        <Text ellipsis type="secondary">
          {repo.local_path}
        </Text>
        {repo.clone_status === "failed" && repo.error ? (
          <Text type="danger">{repo.error}</Text>
        ) : null}
      </Flex>
      {repo.clone_status === "failed" ? (
        <Button icon={<ReloadOutlined />} loading={retrying} onClick={onRetry} size="small">
          重试
        </Button>
      ) : null}
    </Flex>
  );
}
