import { useEffect, useState } from "react";
import { toast } from "sonner";

import { invokeErrorMessage } from "@/lib/errors";
import { listPipelines, listRepoBranches, previewDemandPrd, submitPipeline } from "@/lib/tauri";
import type { DemandListItem, RegisteredRepo } from "@/lib/types";
import { useStore } from "@/state/store";

import { Alert, AlertDescription } from "./ui/alert";
import { Button } from "./ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "./ui/dialog";
import { Input } from "./ui/input";
import { Label } from "./ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "./ui/select";

const STEP_ITEMS = ["前端仓库", "后端仓库", "文档"];

function isJoySpaceUrl(raw: string): boolean {
  try {
    const host = new URL(raw.trim()).hostname.toLowerCase();
    return host === "joyspace.jd.com" || host.endsWith(".joyspace.jd.com");
  } catch {
    return false;
  }
}

function repoLabel(repo: RegisteredRepo): string {
  return `${repo.scope}/${repo.name}`;
}

export function StartPipelineWizard({
  demand,
  onClose,
}: {
  demand: DemandListItem | null;
  onClose: () => void;
}) {
  const { dispatch, state } = useStore();
  const open = demand !== null;
  const demandId = demand?.id ?? 0;

  const readyRepos = state.repos.filter((repo) => repo.clone_status === "ready");

  const [step, setStep] = useState(0);
  const [frontendRepoId, setFrontendRepoId] = useState<string>();
  const [backendRepoId, setBackendRepoId] = useState<string>();
  const [backendBranch, setBackendBranch] = useState<string>();
  const [prdUrl, setPrdUrl] = useState("");
  const [backendTrdUrl, setBackendTrdUrl] = useState("");
  const [prdHint, setPrdHint] = useState<string | null>(null);
  const [branches, setBranches] = useState<string[]>([]);
  const [branchesLoading, setBranchesLoading] = useState(false);
  const [branchError, setBranchError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  const backendOptions = readyRepos.filter((repo) => repo.id !== frontendRepoId);

  useEffect(() => {
    if (!open || !demandId) {
      return;
    }
    setStep(0);
    setFrontendRepoId(undefined);
    setBackendRepoId(undefined);
    setBackendBranch(undefined);
    setPrdUrl("");
    setBackendTrdUrl("");
    setPrdHint(null);
    setBranches([]);
    setBranchError(null);

    let cancelled = false;
    void (async () => {
      try {
        const preview = await previewDemandPrd(demandId);
        if (cancelled) {
          return;
        }
        const resolved = preview.url?.trim();
        if (resolved) {
          setPrdUrl((current) => (current.trim() ? current : resolved));
          setPrdHint(null);
        } else {
          setPrdHint((current) => current ?? "未能自动解析 PRD，请粘贴 JoySpace 链接");
        }
      } catch (error) {
        if (!cancelled) {
          setPrdHint(invokeErrorMessage(error, "未能自动解析 PRD，请粘贴 JoySpace 链接"));
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [demandId, open]);

  useEffect(() => {
    if (!open || step !== 1 || !backendRepoId) {
      return;
    }

    let cancelled = false;
    setBranches([]);
    setBranchesLoading(true);
    setBranchError(null);

    void (async () => {
      try {
        const listed = await listRepoBranches(backendRepoId);
        if (cancelled) {
          return;
        }
        setBranches(listed);
        setBackendBranch((current) => (current && listed.includes(current) ? current : undefined));
        if (listed.length === 0) {
          setBranchError("未找到后端分支，请确认仓库已克隆并 fetch");
        }
      } catch (error) {
        if (!cancelled) {
          setBranchError(invokeErrorMessage(error, "加载后端分支失败"));
        }
      } finally {
        if (!cancelled) {
          setBranchesLoading(false);
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [backendRepoId, open, step]);

  const canNextStep0 = Boolean(frontendRepoId) && readyRepos.length >= 2;
  const canNextStep1 = Boolean(
    backendRepoId && backendBranch && frontendRepoId && backendRepoId !== frontendRepoId,
  );
  const trimmedPrd = prdUrl.trim();
  const trimmedBackendTrd = backendTrdUrl.trim();
  const canSubmit =
    canNextStep0 &&
    canNextStep1 &&
    isJoySpaceUrl(trimmedPrd) &&
    isJoySpaceUrl(trimmedBackendTrd) &&
    trimmedPrd !== trimmedBackendTrd;

  async function handleSubmit() {
    if (!demand || !frontendRepoId || !backendRepoId || !backendBranch) {
      return;
    }
    if (!isJoySpaceUrl(trimmedPrd)) {
      toast.error("请填写 JoySpace PRD 链接");
      return;
    }
    if (!isJoySpaceUrl(trimmedBackendTrd)) {
      toast.error("请填写 JoySpace 后端 TRD 链接");
      return;
    }
    if (trimmedPrd === trimmedBackendTrd) {
      toast.error("后端 TRD 不能与 PRD 使用相同链接");
      return;
    }

    setSubmitting(true);
    try {
      const id = await submitPipeline({
        backendBranch,
        backendRepoId,
        backendTrdUrl: trimmedBackendTrd,
        demandCode: demand.demand_code || undefined,
        demandId: demand.id,
        demandName: demand.name || undefined,
        frontendRepoId,
        prdUrl: trimmedPrd,
      });
      const pipelines = await listPipelines();
      dispatch({ pipelines, type: "hydrate" });
      dispatch({ type: "viewChanged", view: "home" });
      dispatch({ id, type: "pipelineSelected" });
      toast.success("已创建流水线并开始自动执行");
      onClose();
    } catch (error) {
      toast.error(invokeErrorMessage(error, "创建流水线失败"));
    } finally {
      setSubmitting(false);
    }
  }

  function handleNext() {
    if (step === 0 && !canNextStep0) {
      return;
    }
    if (step === 1 && !canNextStep1) {
      return;
    }
    setStep((current) => Math.min(current + 1, 2));
  }

  return (
    <Dialog
      onOpenChange={(next) => {
        if (!next) {
          onClose();
        }
      }}
      open={open}
    >
      <DialogContent className="max-w-xl">
        <DialogHeader>
          <DialogTitle>
            {demand ? `开始：${demand.name || demand.demand_code || demand.id}` : "开始"}
          </DialogTitle>
          <DialogDescription>提交后将按 Init → Deploy 自动连跑。</DialogDescription>
        </DialogHeader>
        <ol className="mb-2 flex gap-2 text-xs">
          {STEP_ITEMS.map((item, index) => (
            <li
              className={
                index === step
                  ? "bg-primary text-primary-foreground rounded-full px-3 py-1"
                  : "bg-muted text-muted-foreground rounded-full px-3 py-1"
              }
              key={item}
            >
              {index + 1}. {item}
            </li>
          ))}
        </ol>
        <Alert>
          <AlertDescription>
            提交后将按 Init → Deploy 自动连跑。失败或阻塞会停住，不会跳过门禁。
          </AlertDescription>
        </Alert>
        {readyRepos.length < 2 ? (
          <Alert variant="warning">
            <AlertDescription className="flex items-center justify-between gap-3">
              <span>需要至少两个已克隆成功的仓库（前端、后端各一）</span>
              <Button
                onClick={() => {
                  onClose();
                  dispatch({ type: "viewChanged", view: "repos" });
                }}
                size="sm"
                variant="outline"
              >
                去登记仓库
              </Button>
            </AlertDescription>
          </Alert>
        ) : null}

        {step === 0 ? (
          <div className="grid gap-2">
            <Label htmlFor="frontend-repo">选择前端仓库。功能分支基于该仓登记的主分支创建。</Label>
            <Select
              onValueChange={(value) => {
                setFrontendRepoId(value);
                if (value === backendRepoId) {
                  setBackendRepoId(undefined);
                  setBackendBranch(undefined);
                  setBranches([]);
                  setBranchError(null);
                }
              }}
              value={frontendRepoId}
            >
              <SelectTrigger aria-label="前端仓库" id="frontend-repo">
                <SelectValue placeholder="选择已克隆的前端仓库" />
              </SelectTrigger>
              <SelectContent>
                {readyRepos.map((repo) => (
                  <SelectItem key={repo.id} value={repo.id}>
                    {repoLabel(repo)}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        ) : null}

        {step === 1 ? (
          <div className="grid gap-3">
            <p className="text-sm">选择后端仓库和分支。仅作为只读上下文，不会创建后端 MR。</p>
            <Select
              onValueChange={(value) => {
                setBackendRepoId(value);
                setBackendBranch(undefined);
                setBranches([]);
                setBranchError(null);
              }}
              value={backendRepoId}
            >
              <SelectTrigger aria-label="后端仓库">
                <SelectValue placeholder="选择已克隆的后端仓库" />
              </SelectTrigger>
              <SelectContent>
                {backendOptions.map((repo) => (
                  <SelectItem key={repo.id} value={repo.id}>
                    {repoLabel(repo)}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <Select
              disabled={!backendRepoId || branchesLoading}
              onValueChange={setBackendBranch}
              value={backendBranch}
            >
              <SelectTrigger aria-label="后端分支">
                <SelectValue placeholder={branchesLoading ? "加载分支..." : "选择后端分支"} />
              </SelectTrigger>
              <SelectContent>
                {branches.map((branch) => (
                  <SelectItem key={branch} value={branch}>
                    {branch}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            {branchError ? (
              <Alert variant="destructive">
                <AlertDescription>{branchError}</AlertDescription>
              </Alert>
            ) : null}
          </div>
        ) : null}

        {step === 2 ? (
          <div className="grid gap-4">
            <div className="grid gap-2">
              <Label htmlFor="prd-url">填写 JoySpace PRD 链接，可改自动预填结果。</Label>
              <Input
                aria-label="JoySpace PRD"
                id="prd-url"
                onChange={(event) => setPrdUrl(event.target.value)}
                placeholder="https://joyspace.jd.com/pages/prd"
                value={prdUrl}
              />
              {prdHint ? <p className="text-muted-foreground text-xs">{prdHint}</p> : null}
              {trimmedPrd && !isJoySpaceUrl(trimmedPrd) ? (
                <p className="text-destructive text-xs">PRD 必须是 JoySpace 链接</p>
              ) : null}
            </div>
            <div className="grid gap-2">
              <Label htmlFor="backend-trd-url">
                填写 JoySpace 后端 TRD 链接。后端 TRD 只读，辅助前端设计/编码，不会改后端仓。
              </Label>
              <Input
                aria-label="JoySpace 后端 TRD"
                id="backend-trd-url"
                onChange={(event) => setBackendTrdUrl(event.target.value)}
                placeholder="https://joyspace.jd.com/pages/backend-trd"
                value={backendTrdUrl}
              />
              {trimmedBackendTrd && !isJoySpaceUrl(trimmedBackendTrd) ? (
                <p className="text-destructive text-xs">后端 TRD 必须是 JoySpace 链接</p>
              ) : null}
              {trimmedPrd && trimmedBackendTrd && trimmedPrd === trimmedBackendTrd ? (
                <p className="text-destructive text-xs">后端 TRD 不能与 PRD 使用相同链接</p>
              ) : null}
            </div>
          </div>
        ) : null}

        <DialogFooter className="sm:justify-between">
          <Button onClick={onClose} variant="outline">
            取消
          </Button>
          <div className="flex gap-2">
            {step > 0 ? (
              <Button
                onClick={() => setStep((current) => Math.max(current - 1, 0))}
                variant="outline"
              >
                上一步
              </Button>
            ) : null}
            {step < 2 ? (
              <Button disabled={step === 0 ? !canNextStep0 : !canNextStep1} onClick={handleNext}>
                下一步
              </Button>
            ) : (
              <Button
                disabled={!canSubmit}
                loading={submitting}
                onClick={() => void handleSubmit()}
              >
                创建并自动执行
              </Button>
            )}
          </div>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
