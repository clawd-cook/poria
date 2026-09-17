import { Alert, Button, Checkbox, Empty, Flex, Input, Table, Typography } from "antd";
import type { ColumnsType } from "antd/es/table";
import { useEffect, useState } from "react";

import { invokeErrorMessage } from "../lib/errors";
import { listDemands, startLogin } from "../lib/tauri";
import type { DemandListItem, DemandPage } from "../lib/types";
import { useStore } from "../state/store";
import { DemandProjectDrawer } from "./DemandProjectDrawer";
import { StartPipelineWizard } from "./StartPipelineWizard";

const { Title } = Typography;

const DEFAULT_PAGE_SIZE = 20;

function isAuthError(message: string): boolean {
  const lower = message.toLowerCase();
  return (
    message.includes("请先登录") ||
    message.includes("登录已过期") ||
    lower.includes("auth") ||
    lower.includes("login") ||
    lower.includes("cookie")
  );
}

function receiverLabel(item: DemandListItem): string {
  return item.receiver_name?.trim() || item.receiver_erp?.trim() || "—";
}

function demandColumns(
  onStart: (item: DemandListItem) => void,
  onOpenDocs: (item: DemandListItem) => void,
): ColumnsType<DemandListItem> {
  return [
    {
      dataIndex: "name",
      ellipsis: true,
      key: "name",
      title: "名称",
    },
    {
      dataIndex: "demand_code",
      key: "demand_code",
      title: "编号",
      width: 140,
    },
    {
      dataIndex: "status_label",
      key: "status_label",
      render: (label: string) => label.trim() || "—",
      title: "状态",
      width: 160,
    },
    {
      key: "receiver",
      render: (_, item) => receiverLabel(item),
      title: "接收人",
      width: 140,
    },
    {
      align: "right",
      key: "action",
      render: (_, item) => (
        <Flex justify="flex-end">
          <Button
            disabled={!item.demand_code && !item.id}
            onClick={() => onOpenDocs(item)}
            size="small"
            type="link"
          >
            文档
          </Button>
          <Button disabled={!item.id} onClick={() => onStart(item)} size="small" type="link">
            开始
          </Button>
        </Flex>
      ),
      title: "操作",
      width: 128,
    },
  ];
}

export function DemandListPage() {
  const { state } = useStore();
  const loggedIn = state.auth.logged_in;
  const active = state.ui.view === "demands";

  const [keywordInput, setKeywordInput] = useState("");
  const [keyword, setKeyword] = useState("");
  const [acceptedByMe, setAcceptedByMe] = useState(false);
  const [current, setCurrent] = useState(1);
  const [pageSize, setPageSize] = useState(DEFAULT_PAGE_SIZE);
  const [reloadToken, setReloadToken] = useState(0);
  const [page, setPage] = useState<DemandPage | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [starting, setStarting] = useState<DemandListItem | null>(null);
  const [viewing, setViewing] = useState<DemandListItem | null>(null);

  useEffect(() => {
    if (!loggedIn) {
      setPage(null);
      setError(null);
      setLoading(false);
      return;
    }
    if (!active) {
      return;
    }

    let cancelled = false;
    setLoading(true);
    setError(null);

    void (async () => {
      try {
        const result = await listDemands({
          acceptedByMe,
          current,
          keyword: keyword || undefined,
          pageSize,
        });
        if (!cancelled) {
          setPage(result);
        }
      } catch (err) {
        if (!cancelled) {
          setPage(null);
          setError(invokeErrorMessage(err, "需求列表加载失败"));
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [acceptedByMe, active, current, keyword, loggedIn, pageSize, reloadToken]);

  function handleSearch(value: string) {
    setKeyword(value.trim());
    setCurrent(1);
  }

  if (!loggedIn) {
    return (
      <div style={{ height: "100%", overflow: "auto", padding: 24 }}>
        <Title level={4}>需求列表</Title>
        <Empty description="请先登录后再查看与你相关的需求" style={{ marginTop: 64 }}>
          <Button onClick={() => void startLogin()} type="primary">
            登录
          </Button>
        </Empty>
      </div>
    );
  }

  return (
    <div style={{ height: "100%", overflow: "auto", padding: 24 }}>
      <Title level={4}>需求列表</Title>
      <Flex align="center" gap={12} style={{ marginBottom: 16, maxWidth: 640 }}>
        <Input.Search
          allowClear
          onChange={(event) => setKeywordInput(event.target.value)}
          onSearch={handleSearch}
          placeholder="搜索需求名称或编号"
          value={keywordInput}
        />
        <Checkbox
          checked={acceptedByMe}
          onChange={(event) => {
            setAcceptedByMe(event.target.checked);
            setCurrent(1);
          }}
        >
          由我受理
        </Checkbox>
      </Flex>

      {error ? (
        <Alert
          action={
            <Flex gap={8}>
              {isAuthError(error) ? (
                <Button onClick={() => void startLogin()} size="small">
                  登录
                </Button>
              ) : null}
              <Button
                onClick={() => setReloadToken((token) => token + 1)}
                size="small"
                type="primary"
              >
                重试
              </Button>
            </Flex>
          }
          message={error}
          showIcon
          style={{ marginBottom: 16 }}
          type="error"
        />
      ) : null}

      <Table<DemandListItem>
        columns={demandColumns(
          (item) => setStarting(item),
          (item) => setViewing(item),
        )}
        dataSource={page?.records ?? []}
        loading={loading}
        locale={{
          emptyText: error
            ? "加载失败，请重试"
            : acceptedByMe
              ? "暂无由你受理的需求"
              : "暂无与你相关的需求",
        }}
        pagination={{
          current: page?.current ?? current,
          onChange: (nextPage, nextSize) => {
            setCurrent(nextPage);
            setPageSize(nextSize);
          },
          pageSize: page?.page_size ?? pageSize,
          showSizeChanger: true,
          showTotal: (total) => `共 ${total} 条`,
          total: page?.total ?? 0,
        }}
        rowKey="id"
        size="middle"
      />
      <StartPipelineWizard demand={starting} onClose={() => setStarting(null)} />
      <DemandProjectDrawer demand={viewing} onClose={() => setViewing(null)} />
    </div>
  );
}
