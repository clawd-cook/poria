import { Alert, App, Button, Flex, Input, Modal, Select, Steps, Typography } from "antd";
import { useEffect, useState } from "react";

import { invokeErrorMessage } from "../lib/errors";
import { listPipelines, listRepoBranches, previewDemandPrd, submitPipeline } from "../lib/tauri";
import type { DemandListItem, RegisteredRepo } from "../lib/types";
import { useStore } from "../state/store";

const { Text } = Typography;

const STEP_ITEMS = [{ title: "前端仓库" }, { title: "后端仓库" }, { title: "文档" }];

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
  const { message } = App.useApp();
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
      message.error("请填写 JoySpace PRD 链接");
      return;
    }
    if (!isJoySpaceUrl(trimmedBackendTrd)) {
      message.error("请填写 JoySpace 后端 TRD 链接");
      return;
    }
    if (trimmedPrd === trimmedBackendTrd) {
      message.error("后端 TRD 不能与 PRD 使用相同链接");
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
      message.success("已创建流水线");
      onClose();
    } catch (error) {
      message.error(invokeErrorMessage(error, "创建流水线失败"));
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
    <Modal
      destroyOnHidden
      footer={
        <Flex justify="space-between">
          <Button onClick={onClose}>取消</Button>
          <Flex gap={8}>
            {step > 0 ? (
              <Button
                onClick={() => {
                  setStep((current) => Math.max(current - 1, 0));
                }}
              >
                上一步
              </Button>
            ) : null}
            {step < 2 ? (
              <Button
                disabled={step === 0 ? !canNextStep0 : !canNextStep1}
                onClick={handleNext}
                type="primary"
              >
                下一步
              </Button>
            ) : (
              <Button
                disabled={!canSubmit}
                loading={submitting}
                onClick={() => void handleSubmit()}
                type="primary"
              >
                创建流水线
              </Button>
            )}
          </Flex>
        </Flex>
      }
      onCancel={onClose}
      open={open}
      title={demand ? `开始：${demand.name || demand.demand_code || demand.id}` : "开始"}
      width={640}
    >
      <Steps current={step} items={STEP_ITEMS} size="small" style={{ marginBottom: 24 }} />

      {readyRepos.length < 2 ? (
        <Alert
          action={
            <Button
              onClick={() => {
                onClose();
                dispatch({ type: "viewChanged", view: "repos" });
              }}
              size="small"
              type="primary"
            >
              去登记仓库
            </Button>
          }
          message="需要至少两个已克隆成功的仓库（前端、后端各一）"
          showIcon
          style={{ marginBottom: 16 }}
          type="warning"
        />
      ) : null}

      {step === 0 ? (
        <Flex gap={8} vertical>
          <Text>选择前端仓库。不选分支，使用托管副本当前检出作为 base_branch。</Text>
          <Select
            onChange={(value) => {
              setFrontendRepoId(value);
              if (value === backendRepoId) {
                setBackendRepoId(undefined);
                setBackendBranch(undefined);
                setBranches([]);
                setBranchError(null);
              }
            }}
            optionFilterProp="label"
            options={readyRepos.map((repo) => ({
              label: repoLabel(repo),
              value: repo.id,
            }))}
            placeholder="选择已克隆的前端仓库"
            showSearch
            value={frontendRepoId}
          />
        </Flex>
      ) : null}

      {step === 1 ? (
        <Flex gap={12} vertical>
          <Text>选择后端仓库和分支。仅作为只读上下文，不会创建后端 MR。</Text>
          <Select
            onChange={(value) => {
              setBackendRepoId(value);
              setBackendBranch(undefined);
              setBranches([]);
              setBranchError(null);
            }}
            optionFilterProp="label"
            options={backendOptions.map((repo) => ({
              label: repoLabel(repo),
              value: repo.id,
            }))}
            placeholder="选择已克隆的后端仓库"
            showSearch
            value={backendRepoId}
          />
          <Select
            disabled={!backendRepoId}
            loading={branchesLoading}
            onChange={setBackendBranch}
            optionFilterProp="label"
            options={branches.map((branch) => ({
              label: branch,
              value: branch,
            }))}
            placeholder="选择后端分支"
            showSearch
            value={backendBranch}
          />
          {branchError ? <Alert message={branchError} showIcon type="error" /> : null}
        </Flex>
      ) : null}

      {step === 2 ? (
        <Flex gap={12} vertical>
          <Flex gap={8} vertical>
            <Text>填写 JoySpace PRD 链接，可改自动预填结果。</Text>
            <Input
              aria-label="JoySpace PRD"
              onChange={(event) => setPrdUrl(event.target.value)}
              placeholder="https://joyspace.jd.com/pages/prd"
              value={prdUrl}
            />
            {prdHint ? <Text type="secondary">{prdHint}</Text> : null}
            {trimmedPrd && !isJoySpaceUrl(trimmedPrd) ? (
              <Text type="danger">PRD 必须是 JoySpace 链接</Text>
            ) : null}
          </Flex>
          <Flex gap={8} vertical>
            <Text>
              填写 JoySpace 后端 TRD 链接。后端 TRD 只读，辅助前端设计/编码，不会改后端仓。
            </Text>
            <Input
              aria-label="JoySpace 后端 TRD"
              onChange={(event) => setBackendTrdUrl(event.target.value)}
              placeholder="https://joyspace.jd.com/pages/backend-trd"
              value={backendTrdUrl}
            />
            {trimmedBackendTrd && !isJoySpaceUrl(trimmedBackendTrd) ? (
              <Text type="danger">后端 TRD 必须是 JoySpace 链接</Text>
            ) : null}
            {trimmedPrd && trimmedBackendTrd && trimmedPrd === trimmedBackendTrd ? (
              <Text type="danger">后端 TRD 不能与 PRD 使用相同链接</Text>
            ) : null}
          </Flex>
        </Flex>
      ) : null}
    </Modal>
  );
}
