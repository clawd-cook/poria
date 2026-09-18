import { ArrowLeftOutlined } from "@ant-design/icons";
import {
  Alert,
  App,
  Button,
  Card,
  Checkbox,
  Empty,
  Flex,
  Input,
  Pagination,
  Spin,
  Tag,
  Typography,
  theme,
} from "antd";
import { useEffect, useState, type ReactNode } from "react";

import { invokeErrorMessage } from "../lib/errors";
import { demandTaskKey } from "../lib/taskKey";
import { listDemands, resolveDemandLink, startLogin } from "../lib/tauri";
import {
  STAGE_LABELS,
  type DemandListItem,
  type DemandPage,
  type PipelineStatus,
  type PipelineSummary,
  type StageEnum,
} from "../lib/types";
import { useStore } from "../state/store";
import { DemandProjectDrawer } from "./DemandProjectDrawer";
import { PipelineDetail } from "./PipelineDetail";
import { StartPipelineWizard } from "./StartPipelineWizard";
import { StatusBadge } from "./StatusBadge";

const { Text } = Typography;

const DEFAULT_PAGE_SIZE = 20;

const LOCAL_COLUMNS: { key: string; label: string; status: PipelineStatus }[] = [
  { key: "running", label: "运行中", status: "running" },
  { key: "blocked", label: "阻塞", status: "blocked" },
  { key: "waiting_merge", label: "待合并", status: "waiting_merge" },
  { key: "completed", label: "完成", status: "completed" },
  { key: "failed", label: "失败", status: "failed" },
  { key: "cancelled", label: "已取消", status: "cancelled" },
];

type BoardCard = {
  currentStage: string | null;
  demand: DemandListItem;
  key: string;
  pipelineId: string | null;
  status: PipelineStatus | "unstarted";
  updatedAt: string | null;
};

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

function timeAgo(dateStr: string): string {
  const diff = Date.now() - new Date(dateStr).getTime();
  const minutes = Math.floor(diff / 60000);
  if (minutes < 1) return "刚刚";
  if (minutes < 60) return `${minutes}分钟前`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}小时前`;
  return `${Math.floor(hours / 24)}天前`;
}

function stageLabel(stage: string | null): string | null {
  if (!stage) return null;
  if (stage in STAGE_LABELS) {
    return STAGE_LABELS[stage as StageEnum];
  }
  return stage;
}

function matchesKeyword(name: string, code: string, demandId: number, keyword: string): boolean {
  if (!keyword) {
    return true;
  }
  const query = keyword.toLowerCase();
  return (
    name.toLowerCase().includes(query) ||
    code.toLowerCase().includes(query) ||
    String(demandId).includes(query) ||
    `id:${demandId}`.includes(query)
  );
}

function isNewerPipeline(candidate: PipelineSummary, current: PipelineSummary): boolean {
  return (
    candidate.updated_at > current.updated_at ||
    (candidate.updated_at === current.updated_at && candidate.id > current.id)
  );
}

function pipelineToDemand(pipeline: PipelineSummary): DemandListItem {
  return {
    demand_code: pipeline.demand_code,
    id: pipeline.demand_id,
    name: pipeline.demand_name,
    receiver_erp: null,
    receiver_name: null,
    status: null,
    status_label: "",
  };
}

function pipelineToCard(pipeline: PipelineSummary): BoardCard {
  return {
    currentStage: pipeline.current_stage,
    demand: pipelineToDemand(pipeline),
    key: demandTaskKey(pipeline.demand_code, pipeline.demand_id),
    pipelineId: pipeline.id,
    status: pipeline.status,
    updatedAt: pipeline.updated_at,
  };
}

export function HomeBoard() {
  const { state, dispatch } = useStore();
  const { token } = theme.useToken();
  const { message } = App.useApp();
  const loggedIn = state.auth.logged_in;
  const boardVisible = state.ui.view === "home" || state.ui.view === "demands";

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
  const [linkInput, setLinkInput] = useState("");
  const [linkLoading, setLinkLoading] = useState(false);

  useEffect(() => {
    if (!loggedIn) {
      setPage(null);
      setError(null);
      setLoading(false);
      return;
    }
    if (!boardVisible) {
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
  }, [acceptedByMe, boardVisible, current, keyword, loggedIn, pageSize, reloadToken]);

  const pipelinesByKey = new Map<string, PipelineSummary>();
  for (const pipeline of state.pipelines) {
    const key = demandTaskKey(pipeline.demand_code, pipeline.demand_id);
    const existing = pipelinesByKey.get(key);
    if (!existing || isNewerPipeline(pipeline, existing)) {
      pipelinesByKey.set(key, pipeline);
    }
  }

  const filteredPipelines = [...pipelinesByKey.values()].filter((pipeline) =>
    matchesKeyword(pipeline.demand_name, pipeline.demand_code, pipeline.demand_id, keyword),
  );

  const createdCards = filteredPipelines
    .filter((pipeline) => pipeline.status === "created")
    .map(pipelineToCard);

  const xingyunUnstarted: BoardCard[] = (page?.records ?? [])
    .filter((item) => !pipelinesByKey.has(demandTaskKey(item.demand_code, item.id)))
    .map((item) => ({
      currentStage: null,
      demand: item,
      key: demandTaskKey(item.demand_code, item.id),
      pipelineId: null,
      status: "unstarted" as const,
      updatedAt: null,
    }));

  // Prefer local created cards so a key collision still opens detail, not the wizard.
  const unstartedCards = [...createdCards, ...xingyunUnstarted].filter(
    (card, index, cards) => cards.findIndex((item) => item.key === card.key) === index,
  );

  function handleSearch(value: string) {
    setKeyword(value.trim());
    setCurrent(1);
  }

  function openCard(card: BoardCard) {
    if (card.pipelineId) {
      dispatch({ id: card.pipelineId, type: "pipelineSelected" });
      return;
    }
    setStarting(card.demand);
  }

  async function handleResolveLink(value: string) {
    const url = value.trim();
    if (!url) {
      return;
    }
    setLinkLoading(true);
    try {
      const demand = await resolveDemandLink(url);
      const existing = pipelinesByKey.get(demandTaskKey(demand.demand_code, demand.id));
      if (existing) {
        dispatch({ id: existing.id, type: "pipelineSelected" });
        setLinkInput("");
        return;
      }
      setStarting(demand);
      setLinkInput("");
    } catch (err) {
      message.error(invokeErrorMessage(err, "无法从链接打开需求"));
    } finally {
      setLinkLoading(false);
    }
  }

  if (state.selectedPipelineId) {
    return (
      <Flex vertical style={{ height: "100%" }}>
        <div style={{ padding: "8px 16px 0" }}>
          <Button
            icon={<ArrowLeftOutlined />}
            onClick={() => dispatch({ id: null, type: "pipelineSelected" })}
            type="text"
          >
            返回看板
          </Button>
        </div>
        <div style={{ flex: 1, minHeight: 0, overflow: "auto" }}>
          <PipelineDetail />
        </div>
        <StartPipelineWizard demand={starting} onClose={() => setStarting(null)} />
        <DemandProjectDrawer demand={viewing} onClose={() => setViewing(null)} />
      </Flex>
    );
  }

  return (
    <Flex vertical gap={12} style={{ height: "100%", padding: 16 }}>
      <Flex align="center" gap={12} wrap="wrap">
        <Input.Search
          allowClear
          onChange={(event) => setKeywordInput(event.target.value)}
          onSearch={handleSearch}
          placeholder="搜索任务名称或编号"
          style={{ maxWidth: 320 }}
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
        <Input.Search
          allowClear
          enterButton="从链接开始"
          loading={linkLoading}
          onChange={(event) => setLinkInput(event.target.value)}
          onSearch={(value) => void handleResolveLink(value)}
          placeholder="粘贴行云需求链接"
          style={{ flex: 1, minWidth: 280 }}
          value={linkInput}
        />
      </Flex>

      <Flex gap={12} style={{ flex: 1, minHeight: 0, overflowX: "auto" }}>
        <BoardColumn
          count={loggedIn ? unstartedCards.length : createdCards.length}
          label="未开始"
          token={token}
        >
          {!loggedIn ? (
            <>
              <Empty
                description="登录后查看行云中尚未开工的任务"
                image={Empty.PRESENTED_IMAGE_SIMPLE}
              >
                <Button onClick={() => void startLogin()} type="primary">
                  登录
                </Button>
              </Empty>
              {createdCards.map((card) => (
                <KanbanCard
                  card={card}
                  key={card.key}
                  onOpenDocs={() => setViewing(card.demand)}
                  onSelect={() => openCard(card)}
                />
              ))}
            </>
          ) : (
            <>
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
                  type="error"
                />
              ) : null}
              {loading ? (
                <Flex align="center" justify="center" style={{ padding: 16 }}>
                  <Spin size="small" />
                </Flex>
              ) : null}
              {unstartedCards.map((card) => (
                <KanbanCard
                  card={card}
                  key={card.key}
                  onOpenDocs={() => setViewing(card.demand)}
                  onSelect={() => openCard(card)}
                />
              ))}
              {!loading && !error && unstartedCards.length === 0 ? (
                <Empty
                  description={
                    acceptedByMe ? "暂无由你受理的未开始任务" : "暂无与你相关的未开始任务"
                  }
                  image={Empty.PRESENTED_IMAGE_SIMPLE}
                />
              ) : null}
              {loggedIn && (page?.total ?? 0) > pageSize ? (
                <Pagination
                  current={page?.current ?? current}
                  onChange={(nextPage, nextSize) => {
                    setCurrent(nextPage);
                    setPageSize(nextSize);
                  }}
                  pageSize={page?.page_size ?? pageSize}
                  size="small"
                  total={page?.total ?? 0}
                />
              ) : null}
            </>
          )}
        </BoardColumn>

        {LOCAL_COLUMNS.map((column) => {
          const items = filteredPipelines
            .filter((pipeline) => pipeline.status === column.status)
            .map(pipelineToCard);
          return (
            <BoardColumn count={items.length} key={column.key} label={column.label} token={token}>
              {items.map((card) => (
                <KanbanCard
                  card={card}
                  key={card.key}
                  onOpenDocs={() => setViewing(card.demand)}
                  onSelect={() => openCard(card)}
                />
              ))}
            </BoardColumn>
          );
        })}
      </Flex>

      <StartPipelineWizard demand={starting} onClose={() => setStarting(null)} />
      <DemandProjectDrawer demand={viewing} onClose={() => setViewing(null)} />
    </Flex>
  );
}

function BoardColumn({
  children,
  count,
  label,
  token,
}: {
  children: ReactNode;
  count: number;
  label: string;
  token: ReturnType<typeof theme.useToken>["token"];
}) {
  return (
    <Flex
      gap={8}
      style={{
        background: token.colorFillAlter,
        borderRadius: token.borderRadiusLG,
        flex: "0 0 260px",
        minWidth: 260,
        padding: 12,
      }}
      vertical
    >
      <Text strong>
        {label} ({count})
      </Text>
      <Flex gap={8} style={{ flex: 1, minHeight: 0, overflowY: "auto" }} vertical>
        {children}
      </Flex>
    </Flex>
  );
}

function KanbanCard({
  card,
  onOpenDocs,
  onSelect,
}: {
  card: BoardCard;
  onOpenDocs: () => void;
  onSelect: () => void;
}) {
  const stage = stageLabel(card.currentStage);

  return (
    <Card hoverable onClick={onSelect} size="small" styles={{ body: { padding: 12 } }}>
      <Flex align="flex-start" gap={8} justify="space-between">
        <Text ellipsis strong style={{ flex: 1 }}>
          {card.demand.name || card.demand.demand_code || `需求 ${card.demand.id}`}
        </Text>
        {card.status === "unstarted" || card.status === "created" ? (
          <Tag style={{ margin: 0 }}>未开始</Tag>
        ) : (
          <StatusBadge status={card.status} />
        )}
      </Flex>
      <Flex align="center" gap={8} style={{ marginTop: 8 }} wrap="wrap">
        {stage ? <Tag style={{ fontSize: 11, margin: 0 }}>{stage}</Tag> : null}
        <Text style={{ fontSize: 11 }} type="secondary">
          {card.demand.demand_code || `id:${card.demand.id}`}
          {card.updatedAt ? ` · ${timeAgo(card.updatedAt)}` : ""}
        </Text>
        <Button
          onClick={(event) => {
            event.stopPropagation();
            onOpenDocs();
          }}
          size="small"
          type="link"
        >
          文档
        </Button>
      </Flex>
    </Card>
  );
}
