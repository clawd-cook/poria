import { Check, FileText, LogIn, Pencil, RotateCcw, SkipForward, XCircle } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";

import { isAuthExpiredIssue, isAuthExpiredMessage } from "@/lib/auth";
import { invokeErrorMessage } from "@/lib/errors";
import { issueClassKey, issueClassLabel, issueClassTitle } from "@/lib/issueClass";
import { isOutputGuardBlock } from "@/lib/outputGuard";
import { isP0UnansweredMessage, isRequirementAmbiguousIssue } from "@/lib/prdReview";
import { isQualityGateBlock, qualityGateTitle } from "@/lib/qualityGates";
import {
  confirmMergeReady,
  confirmTrd,
  humanLoopRespond,
  pipelineAdvance,
  readDemandProjectFile,
  startLogin,
} from "@/lib/tauri";
import { isTrdUnconfirmedIssue, isTrdUnconfirmedMessage } from "@/lib/trd";
import type { PrdReviewStatus } from "@/lib/types";
import { useStore } from "@/state/store";

import { MarkdownView } from "./MarkdownView";
import { PrdReviewEditor } from "./PrdReviewEditor";
import { Spinner } from "./Spinner";
import { Alert, AlertDescription, AlertIcon, AlertTitle } from "./ui/alert";
import { Button } from "./ui/button";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "./ui/dialog";

interface HumanLoopCardProps {
  demandCode: string;
  demandId: number;
  detail: string;
  issueClass: string;
  pipelineId: string;
  stage: string;
}

export function HumanLoopCard({
  demandCode,
  demandId,
  detail,
  issueClass,
  pipelineId,
  stage,
}: HumanLoopCardProps) {
  const { dispatch } = useStore();
  const [loading, setLoading] = useState<string | null>(null);
  const [editorOpen, setEditorOpen] = useState(false);
  const [trdOpen, setTrdOpen] = useState(false);
  const [trdContent, setTrdContent] = useState("");
  const [trdLoading, setTrdLoading] = useState(false);
  const [annotateNote, setAnnotateNote] = useState("");
  const [skipForce, setSkipForce] = useState(false);
  const authExpired = isAuthExpiredIssue(issueClass) || isAuthExpiredMessage(detail);
  const p0Blocked = isRequirementAmbiguousIssue(issueClass) || isP0UnansweredMessage(detail);
  const trdBlocked = isTrdUnconfirmedIssue(issueClass) || isTrdUnconfirmedMessage(detail);
  const outputGuardBlocked = isOutputGuardBlock(issueClass, detail);
  const qualityGateBlocked =
    !authExpired &&
    !p0Blocked &&
    !trdBlocked &&
    !outputGuardBlocked &&
    isQualityGateBlock(issueClass, detail);
  const waitingMerge =
    issueClassKey(issueClass) === "waiting_merge" || detail.includes("不会自动点合并");
  const awaitingAdvance = issueClassKey(issueClass) === "awaiting_advance";
  const routedTitle = issueClassTitle(issueClass, detail);

  async function handleAction(action: string) {
    setLoading(action);
    try {
      await humanLoopRespond(pipelineId, action);
      dispatch({ pipelineId, type: "humanRequestDismissed" });
    } catch (error) {
      toast.error(invokeErrorMessage(error, "处理协助请求失败"));
    } finally {
      setLoading(null);
    }
  }

  async function handleAdvance(action: string, force = false) {
    setLoading(action);
    try {
      await pipelineAdvance(pipelineId, action, action === "annotate" ? annotateNote : null, force);
      if (action !== "annotate") {
        dispatch({ pipelineId, type: "humanRequestDismissed" });
      } else {
        toast.success("已记录补充上下文，确认后点「继续」进入下一阶段");
        setAnnotateNote("");
      }
    } catch (error) {
      toast.error(invokeErrorMessage(error, "阶段推进失败"));
    } finally {
      setLoading(null);
    }
  }

  async function handleRelogin() {
    setLoading("login");
    try {
      await startLogin();
      toast.info("请在浏览器完成登录，成功后将从当前阶段继续");
    } catch (error) {
      toast.error(invokeErrorMessage(error, "打开登录页失败"));
    } finally {
      setLoading(null);
    }
  }

  async function handleConfirmMerge() {
    setLoading("confirm-merge");
    try {
      await confirmMergeReady(pipelineId);
      dispatch({ pipelineId, type: "humanRequestDismissed" });
      toast.success("已在 MR 留下可合并确认，请在 Coding 上合入");
    } catch (error) {
      toast.error(invokeErrorMessage(error, "确认可合并失败"));
    } finally {
      setLoading(null);
    }
  }

  async function handleConfirmTrd(skipped: boolean) {
    setLoading(skipped ? "skip-confirm" : "confirm");
    try {
      await confirmTrd(pipelineId, skipped);
      dispatch({ pipelineId, type: "humanRequestDismissed" });
    } catch (error) {
      toast.error(invokeErrorMessage(error, "确认 TRD 失败"));
    } finally {
      setLoading(null);
    }
  }

  async function handleOpenTrd() {
    setTrdOpen(true);
    setTrdLoading(true);
    try {
      const file = await readDemandProjectFile({
        demandCode,
        demandId,
        fileName: "TRD.md",
      });
      setTrdContent(file.content);
    } catch (error) {
      toast.error(invokeErrorMessage(error, "读取 TRD.md 失败"));
      setTrdContent("");
    } finally {
      setTrdLoading(false);
    }
  }

  async function handleSaved(status: PrdReviewStatus | null) {
    if (!status?.p0_done) {
      return;
    }
    setEditorOpen(false);
    await handleAction("resume");
  }

  const title = authExpired
    ? "SSO 已过期，流水线已挂起"
    : awaitingAdvance
      ? "阶段已完成，请确认后继续"
      : p0Blocked
        ? "P0 未答，无法进入设计"
        : trdBlocked
          ? "请确认前端 TRD 后再进入开发"
          : outputGuardBlocked
            ? "代码超出 TRD 允许范围，无法进入 CR"
            : qualityGateBlocked
              ? qualityGateTitle(issueClass, detail)
              : waitingMerge
                ? "MR 待审查人确认后合入"
                : (routedTitle ?? "Pipeline 需要协助");

  return (
    <>
      <Alert variant="warning">
        <div className="flex gap-3">
          <AlertIcon variant="warning" />
          <div className="min-w-0 flex-1">
            <AlertTitle>{title}</AlertTitle>
            <AlertDescription className="text-foreground grid gap-3">
              <dl className="grid gap-1 text-sm">
                <div>
                  <dt className="text-muted-foreground inline">问题：</dt>
                  <dd className="inline">
                    {issueClassLabel(issueClass)}
                    <span className="text-muted-foreground ml-1">({issueClass})</span>
                  </dd>
                </div>
                <div>
                  <dt className="text-muted-foreground inline">阶段：</dt>
                  <dd className="inline">{stage}</dd>
                </div>
              </dl>
              <p className="text-muted-foreground text-xs">
                {waitingMerge
                  ? detail
                  : awaitingAdvance
                    ? detail
                    : `${detail} 已同时通过京ME 通知；桌面或京ME 回复只处理一次。`}
              </p>
              {awaitingAdvance ? (
                <div className="grid gap-2">
                  <textarea
                    className="border-input bg-background min-h-16 w-full rounded-md border px-3 py-2 text-sm"
                    onChange={(e) => setAnnotateNote(e.target.value)}
                    placeholder="可选：补充上下文（写入下一阶段 prompt）"
                    value={annotateNote}
                  />
                  <label className="text-muted-foreground flex items-center gap-2 text-xs">
                    <input
                      checked={skipForce}
                      onChange={(e) => setSkipForce(e.target.checked)}
                      type="checkbox"
                    />
                    跳过锁闸阶段（高风险，需勾选）
                  </label>
                </div>
              ) : null}
              <div className="flex flex-wrap gap-2">
                {awaitingAdvance ? (
                  <>
                    <Button
                      disabled={loading !== null}
                      loading={loading === "continue"}
                      onClick={() => void handleAdvance("continue")}
                    >
                      <Check aria-hidden />
                      继续
                    </Button>
                    <Button
                      disabled={loading !== null}
                      loading={loading === "redo"}
                      onClick={() => void handleAdvance("redo")}
                      variant="outline"
                    >
                      <RotateCcw aria-hidden />
                      重做
                    </Button>
                    <Button
                      disabled={loading !== null || !annotateNote.trim()}
                      loading={loading === "annotate"}
                      onClick={() => void handleAdvance("annotate")}
                      variant="outline"
                    >
                      补充上下文
                    </Button>
                    <Button
                      disabled={loading !== null}
                      loading={loading === "skip"}
                      onClick={() => void handleAdvance("skip", skipForce)}
                      variant="outline"
                    >
                      <SkipForward aria-hidden />
                      跳过
                    </Button>
                  </>
                ) : null}
                {authExpired ? (
                  <Button
                    disabled={loading !== null}
                    loading={loading === "login"}
                    onClick={() => void handleRelogin()}
                  >
                    <LogIn aria-hidden />
                    重新登录
                  </Button>
                ) : null}
                {p0Blocked ? (
                  <Button disabled={loading !== null} onClick={() => setEditorOpen(true)}>
                    <Pencil aria-hidden />
                    填写 P0 答案
                  </Button>
                ) : null}
                {trdBlocked ? (
                  <>
                    <Button
                      disabled={loading !== null}
                      onClick={() => void handleOpenTrd()}
                      variant="outline"
                    >
                      <FileText aria-hidden />
                      查看 TRD.md
                    </Button>
                    <Button
                      disabled={loading !== null}
                      loading={loading === "confirm"}
                      onClick={() => void handleConfirmTrd(false)}
                    >
                      <Check aria-hidden />
                      确认 TRD
                    </Button>
                  </>
                ) : null}
                {waitingMerge ? (
                  <Button
                    disabled={loading !== null}
                    loading={loading === "confirm-merge"}
                    onClick={() => void handleConfirmMerge()}
                  >
                    <Check aria-hidden />
                    确认可合并
                  </Button>
                ) : null}
                {waitingMerge || awaitingAdvance ? null : (
                  <Button
                    disabled={loading !== null}
                    loading={loading === "resume"}
                    onClick={() => void handleAction("resume")}
                    variant={
                      authExpired ||
                      p0Blocked ||
                      trdBlocked ||
                      outputGuardBlocked ||
                      qualityGateBlocked
                        ? "outline"
                        : "default"
                    }
                  >
                    <RotateCcw aria-hidden />
                    {authExpired
                      ? "登录后继续"
                      : p0Blocked
                        ? "答完后继续"
                        : trdBlocked
                          ? "确认后继续"
                          : outputGuardBlocked
                            ? "收回越界改动后继续"
                            : qualityGateBlocked
                              ? "补齐报告后继续"
                              : "修复并重试"}
                  </Button>
                )}
                {waitingMerge || awaitingAdvance ? null : (
                  <Button
                    disabled={loading !== null}
                    loading={loading === "skip" || loading === "skip-confirm"}
                    onClick={() =>
                      void (trdBlocked ? handleConfirmTrd(true) : handleAction("skip"))
                    }
                    variant="outline"
                  >
                    <SkipForward aria-hidden />
                    {trdBlocked ? "跳过确认" : "跳过"}
                  </Button>
                )}
                <Button
                  disabled={loading !== null}
                  loading={loading === "cancel"}
                  onClick={() => void handleAction("cancel")}
                  variant="destructive"
                >
                  <XCircle aria-hidden />
                  取消 Pipeline
                </Button>
              </div>
            </AlertDescription>
          </div>
        </div>
      </Alert>
      <Dialog onOpenChange={setEditorOpen} open={editorOpen}>
        <DialogContent className="max-w-3xl">
          <DialogHeader>
            <DialogTitle>填写 PRD_REVIEW.md P0 答案</DialogTitle>
          </DialogHeader>
          <PrdReviewEditor demandCode={demandCode} demandId={demandId} onSaved={handleSaved} />
        </DialogContent>
      </Dialog>
      <Dialog onOpenChange={setTrdOpen} open={trdOpen}>
        <DialogContent className="max-w-3xl">
          <DialogHeader>
            <DialogTitle>前端 TRD.md</DialogTitle>
          </DialogHeader>
          {trdLoading ? <Spinner /> : <MarkdownView content={trdContent} />}
        </DialogContent>
      </Dialog>
    </>
  );
}
