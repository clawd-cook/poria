import { useEffect, useRef, useState, type ReactNode } from "react";
import { toast } from "sonner";

import { isAuthExpiredMessage } from "@/lib/auth";
import { invokeErrorMessage } from "@/lib/errors";
import { pipelineHitlLane, type HitlLane } from "@/lib/hitlLane";
import { issueClassLabel } from "@/lib/issueClass";
import { countAttentionPipelines } from "@/lib/pipelineViewModel";
import { demandTaskKey } from "@/lib/taskKey";
import { listDemands, resolveDemandLink, startLogin } from "@/lib/tauri";
import {
  STAGE_LABELS,
  type DemandListItem,
  type DemandPage,
  type PipelineStatus,
  type PipelineSummary,
  type StageEnum,
} from "@/lib/types";
import { useStore } from "@/state/store";

import { DemandProjectDrawer } from "./DemandProjectDrawer";
import { DemandWorkbench } from "./DemandWorkbench";
import { EmptyState } from "./EmptyState";
import { PipelineStatsPanel } from "./PipelineStatsPanel";
import { Spinner } from "./Spinner";
import { StartPipelineWizard } from "./StartPipelineWizard";
import { StatusBadge } from "./StatusBadge";
import { Alert, AlertDescription } from "./ui/alert";
import { Badge } from "./ui/badge";
import { Button } from "./ui/button";
import { Card, CardContent } from "./ui/card";
import { Checkbox } from "./ui/checkbox";
import { Input } from "./ui/input";
import { Label } from "./ui/label";

const DEFAULT_PAGE_SIZE = 20;

const LOCAL_COLUMNS: {
  key: string;
  label: string;
  match: (pipeline: PipelineSummary) => boolean;
}[] = [
  { key: "running", label: "运行中", match: (pipeline) => pipeline.status === "running" },
  {
    key: "confirm",
    label: "待确认",
    match: (pipeline) => pipelineHitlLane(pipeline) === "confirm",
  },
  { key: "review", label: "待验收", match: (pipeline) => pipelineHitlLane(pipeline) === "review" },
  { key: "blocked", label: "阻塞", match: (pipeline) => pipelineHitlLane(pipeline) === "blocked" },
  { key: "completed", label: "完成", match: (pipeline) => pipeline.status === "completed" },
  { key: "failed", label: "失败", match: (pipeline) => pipeline.status === "failed" },
  { key: "cancelled", label: "已取消", match: (pipeline) => pipeline.status === "cancelled" },
];

type BoardCard = {
  currentStage: string | null;
  demand: DemandListItem;
  hitlLane: HitlLane | null;
  issueClass: string | null;
  key: string;
  pipelineId: string | null;
  status: PipelineStatus | "unstarted";
  updatedAt: string | null;
};

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
  if (!stage) {
    return null;
  }
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

function isBoardLaneInteractive(target: EventTarget | null): boolean {
  return (
    target instanceof HTMLElement &&
    Boolean(target.closest("a, button, input, textarea, [data-board-interactive]"))
  );
}

function canScrollVertically(element: HTMLElement, deltaY: number): boolean {
  if (deltaY < 0) {
    return element.scrollTop > 0;
  }
  if (deltaY > 0) {
    return element.scrollTop + element.clientHeight < element.scrollHeight - 1;
  }
  return false;
}

function pipelineToCard(pipeline: PipelineSummary): BoardCard {
  return {
    currentStage: pipeline.current_stage,
    demand: pipelineToDemand(pipeline),
    hitlLane: pipelineHitlLane(pipeline),
    issueClass: pipeline.issue_class,
    key: demandTaskKey(pipeline.demand_code, pipeline.demand_id),
    pipelineId: pipeline.id,
    status: pipeline.status,
    updatedAt: pipeline.updated_at,
  };
}

export function HomeBoard() {
  const { dispatch, state } = useStore();
  const loggedIn = state.auth.logged_in && state.auth.cookie_valid;
  const boardVisible = state.ui.view === "home";

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
  const laneRef = useRef<HTMLDivElement>(null);
  const showLane = state.ui.surface === "list";
  const [attentionOnly, setAttentionOnly] = useState(false);
  const attentionCount = countAttentionPipelines(
    state.pipelines,
    state.humanRequest?.pipelineId ?? null,
  );

  useEffect(() => {
    if (!showLane) {
      return;
    }
    const lane = laneRef.current;
    if (!lane) {
      return;
    }
    const scroller: HTMLDivElement = lane;

    let dragging = false;
    let startX = 0;
    let startScroll = 0;

    function onWheel(event: WheelEvent) {
      if (Math.abs(event.deltaY) < Math.abs(event.deltaX)) {
        return;
      }
      const columnScroll =
        event.target instanceof Element ? event.target.closest("[data-board-column-scroll]") : null;
      if (columnScroll instanceof HTMLElement && canScrollVertically(columnScroll, event.deltaY)) {
        return;
      }
      if (scroller.scrollWidth <= scroller.clientWidth) {
        return;
      }
      event.preventDefault();
      scroller.scrollLeft += event.deltaY;
    }

    function onPointerDown(event: PointerEvent) {
      if (event.button !== 0 || isBoardLaneInteractive(event.target)) {
        return;
      }
      dragging = true;
      startX = event.clientX;
      startScroll = scroller.scrollLeft;
      scroller.setPointerCapture(event.pointerId);
    }

    function onPointerMove(event: PointerEvent) {
      if (!dragging) {
        return;
      }
      scroller.scrollLeft = startScroll - (event.clientX - startX);
    }

    function onPointerUp() {
      dragging = false;
    }

    scroller.addEventListener("wheel", onWheel, { passive: false });
    scroller.addEventListener("pointerdown", onPointerDown);
    scroller.addEventListener("pointermove", onPointerMove);
    scroller.addEventListener("pointerup", onPointerUp);
    scroller.addEventListener("pointercancel", onPointerUp);
    return () => {
      scroller.removeEventListener("wheel", onWheel);
      scroller.removeEventListener("pointerdown", onPointerDown);
      scroller.removeEventListener("pointermove", onPointerMove);
      scroller.removeEventListener("pointerup", onPointerUp);
      scroller.removeEventListener("pointercancel", onPointerUp);
    };
  }, [showLane]);

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
      hitlLane: null,
      issueClass: null,
      key: demandTaskKey(item.demand_code, item.id),
      pipelineId: null,
      status: "unstarted" as const,
      updatedAt: null,
    }));

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
      toast.error(invokeErrorMessage(err, "无法从链接打开需求"));
    } finally {
      setLinkLoading(false);
    }
  }

  if (state.ui.surface === "workbench" && state.selectedPipelineId) {
    return (
      <div className="flex h-full flex-col">
        <div className="min-h-0 flex-1 overflow-hidden">
          <DemandWorkbench />
        </div>
        <StartPipelineWizard demand={starting} onClose={() => setStarting(null)} />
        <DemandProjectDrawer demand={viewing} onClose={() => setViewing(null)} />
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col gap-4 p-6">
      <PipelineStatsPanel compact />
      <div className="flex flex-wrap items-center gap-3">
        <form
          className="flex max-w-80 flex-1 gap-2"
          onSubmit={(event) => {
            event.preventDefault();
            handleSearch(keywordInput);
          }}
        >
          <Input
            aria-label="搜索任务名称或编号"
            onChange={(event) => setKeywordInput(event.target.value)}
            placeholder="搜索任务名称或编号"
            value={keywordInput}
          />
          <Button type="submit" variant="outline">
            搜索
          </Button>
        </form>
        <div className="flex items-center gap-2">
          <Checkbox
            checked={acceptedByMe}
            id="accepted-by-me"
            onCheckedChange={(value) => {
              setAcceptedByMe(value === true);
              setCurrent(1);
            }}
          />
          <Label htmlFor="accepted-by-me">由我受理</Label>
        </div>
        <div className="flex items-center gap-2">
          <Checkbox
            checked={attentionOnly}
            id="attention-only"
            onCheckedChange={(value) => setAttentionOnly(value === true)}
          />
          <Label htmlFor="attention-only">
            待我处理
            {attentionCount > 0 ? (
              <Badge className="ml-1" variant="destructive">
                {attentionCount}
              </Badge>
            ) : null}
          </Label>
        </div>
        <form
          className="flex min-w-[280px] flex-1 gap-2"
          onSubmit={(event) => {
            event.preventDefault();
            void handleResolveLink(linkInput);
          }}
        >
          <Input
            aria-label="行云需求链接"
            onChange={(event) => setLinkInput(event.target.value)}
            placeholder="粘贴行云需求链接"
            value={linkInput}
          />
          <Button loading={linkLoading} type="submit">
            从链接开始
          </Button>
        </form>
      </div>

      <div className="flex min-h-0 flex-1 gap-4 overflow-x-auto" ref={laneRef}>
        {!attentionOnly ? (
          <BoardColumn
            count={loggedIn ? unstartedCards.length : createdCards.length}
            label="未开始"
          >
            {!loggedIn ? (
              <>
                <EmptyState
                  action={<Button onClick={() => void startLogin()}>登录</Button>}
                  description="登录后查看行云中尚未开工的任务"
                />
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
                  <Alert variant="destructive">
                    <AlertDescription className="flex flex-col gap-2">
                      <span>{error}</span>
                      <div className="flex gap-2">
                        {isAuthExpiredMessage(error) ? (
                          <Button onClick={() => void startLogin()} size="sm" variant="outline">
                            登录
                          </Button>
                        ) : null}
                        <Button onClick={() => setReloadToken((count) => count + 1)} size="sm">
                          重试
                        </Button>
                      </div>
                    </AlertDescription>
                  </Alert>
                ) : null}
                {loading ? <Spinner label="加载需求..." /> : null}
                {unstartedCards.map((card) => (
                  <KanbanCard
                    card={card}
                    key={card.key}
                    onOpenDocs={() => setViewing(card.demand)}
                    onSelect={() => openCard(card)}
                  />
                ))}
                {!loading && !error && unstartedCards.length === 0 ? (
                  <EmptyState
                    description={
                      acceptedByMe ? "暂无由你受理的未开始任务" : "暂无与你相关的未开始任务"
                    }
                  />
                ) : null}
                {loggedIn && (page?.total ?? 0) > pageSize ? (
                  <div
                    className="flex items-center justify-between gap-2"
                    data-board-interactive=""
                  >
                    <Button
                      disabled={(page?.current ?? current) <= 1}
                      onClick={() => setCurrent((page?.current ?? current) - 1)}
                      size="sm"
                      variant="outline"
                    >
                      上一页
                    </Button>
                    <span className="text-muted-foreground text-xs">
                      {page?.current ?? current} / {Math.ceil((page?.total ?? 0) / pageSize)}
                    </span>
                    <Button
                      disabled={
                        (page?.current ?? current) >= Math.ceil((page?.total ?? 0) / pageSize)
                      }
                      onClick={() => {
                        setCurrent((page?.current ?? current) + 1);
                        setPageSize(page?.page_size ?? pageSize);
                      }}
                      size="sm"
                      variant="outline"
                    >
                      下一页
                    </Button>
                  </div>
                ) : null}
              </>
            )}
          </BoardColumn>
        ) : null}

        {LOCAL_COLUMNS.filter((column) =>
          attentionOnly
            ? column.key === "confirm" || column.key === "review" || column.key === "blocked"
            : true,
        ).map((column) => {
          const items = filteredPipelines.filter(column.match).map(pipelineToCard);
          return (
            <BoardColumn count={items.length} key={column.key} label={column.label}>
              {items.map((card) => (
                <KanbanCard
                  card={card}
                  key={card.key}
                  onOpenDocs={() => setViewing(card.demand)}
                  onSelect={() => openCard(card)}
                />
              ))}
              {attentionOnly && items.length === 0 ? (
                <EmptyState description="该列暂无待处理项" />
              ) : null}
            </BoardColumn>
          );
        })}
      </div>

      <StartPipelineWizard demand={starting} onClose={() => setStarting(null)} />
      <DemandProjectDrawer demand={viewing} onClose={() => setViewing(null)} />
    </div>
  );
}

function BoardColumn({
  children,
  count,
  label,
}: {
  children: ReactNode;
  count: number;
  label: string;
}) {
  return (
    <section className="border-border bg-card flex w-[260px] shrink-0 flex-col gap-2 rounded-lg border p-4">
      <h2 className="font-serif text-sm font-bold">
        {label} ({count})
      </h2>
      <div
        className="flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto"
        data-board-column-scroll=""
      >
        {children}
      </div>
    </section>
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
    <Card
      className="hover:border-primary cursor-pointer transition-colors duration-200"
      data-board-interactive=""
      onClick={onSelect}
    >
      <CardContent className="p-4">
        <div className="flex items-start justify-between gap-2">
          <p className="flex-1 truncate text-sm font-semibold">
            {card.demand.name || card.demand.demand_code || `需求 ${card.demand.id}`}
          </p>
          {card.status === "unstarted" || card.status === "created" ? (
            <Badge variant="secondary">未开始</Badge>
          ) : (
            <StatusBadge status={card.status} />
          )}
        </div>
        <div className="mt-2 flex flex-wrap items-center gap-2">
          {card.hitlLane === "confirm" || card.hitlLane === "review" ? (
            <Badge variant="outline">{issueClassLabel(card.issueClass)}</Badge>
          ) : null}
          {stage ? <Badge variant="outline">{stage}</Badge> : null}
          <span className="text-muted-foreground text-xs">
            {card.demand.demand_code || `id:${card.demand.id}`}
            {card.updatedAt ? ` · ${timeAgo(card.updatedAt)}` : ""}
          </span>
          <Button
            onClick={(event) => {
              event.stopPropagation();
              onOpenDocs();
            }}
            size="sm"
            variant="link"
          >
            文档
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}
