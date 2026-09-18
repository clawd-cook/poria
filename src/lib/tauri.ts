import { invoke } from "@tauri-apps/api/core";

import type {
  AppConfig,
  AuthStatus,
  ChannelInfo,
  ClaudeProbeResult,
  DemandListItem,
  DemandPage,
  DemandPrdPreview,
  DemandProject,
  DemandProjectFileContent,
  PipelineDetail,
  PipelineSummary,
  RegisteredRepo,
  SkillDetail,
  SkillInfo,
  SubmitPipelineInput,
} from "./types";

export async function listPipelines(): Promise<PipelineSummary[]> {
  return invoke<PipelineSummary[]>("list_pipelines");
}

export async function getPipeline(id: string): Promise<PipelineDetail> {
  return invoke<PipelineDetail>("get_pipeline", { id });
}

export async function submitPipeline(input: SubmitPipelineInput): Promise<string> {
  return invoke<string>("submit_pipeline", {
    backendBranch: input.backendBranch,
    backendRepoId: input.backendRepoId,
    backendTrdUrl: input.backendTrdUrl,
    demandCode: input.demandCode,
    demandId: input.demandId,
    demandName: input.demandName,
    frontendRepoId: input.frontendRepoId,
    prdUrl: input.prdUrl,
  });
}

export async function cancelPipeline(id: string): Promise<void> {
  return invoke<void>("cancel_pipeline", { id });
}

export async function humanLoopRespond(pipelineId: string, action: string): Promise<void> {
  return invoke<void>("human_loop_respond", { pipelineId, action });
}

export async function getAuthStatus(): Promise<AuthStatus> {
  return invoke<AuthStatus>("get_auth_status");
}

export async function startLogin(): Promise<void> {
  return invoke<void>("start_login");
}

export async function logout(): Promise<void> {
  return invoke<void>("logout");
}

export async function getConfig(): Promise<AppConfig> {
  return invoke<AppConfig>("get_config");
}

export async function updateConfig(config: AppConfig): Promise<void> {
  return invoke<void>("update_config", { config });
}

export async function probeClaude(pathOverride?: string | null): Promise<ClaudeProbeResult> {
  return invoke<ClaudeProbeResult>("probe_claude", { pathOverride: pathOverride ?? null });
}

export async function listSkills(): Promise<SkillInfo[]> {
  return invoke<SkillInfo[]>("list_skills");
}

export async function getSkill(skillId: string): Promise<SkillDetail> {
  return invoke<SkillDetail>("get_skill", { skillId });
}

export async function listChannels(): Promise<ChannelInfo[]> {
  return invoke<ChannelInfo[]>("list_channels");
}

export async function executeStage(pipelineId: string): Promise<void> {
  return invoke<void>("execute_stage", { pipelineId });
}

export async function skipStage(pipelineId: string, stageName: string): Promise<void> {
  return invoke<void>("skip_stage", { pipelineId, stageName });
}

export async function openWorkspace(pipelineId: string): Promise<void> {
  return invoke<void>("open_workspace", { pipelineId });
}

export async function listRepos(): Promise<RegisteredRepo[]> {
  return invoke<RegisteredRepo[]>("list_repos");
}

export async function registerRepo(gitUrl: string): Promise<RegisteredRepo> {
  return invoke<RegisteredRepo>("register_repo", { gitUrl });
}

export async function retryClone(id: string): Promise<RegisteredRepo> {
  return invoke<RegisteredRepo>("retry_clone", { id });
}

export async function listDemands(input: {
  acceptedByMe?: boolean;
  current?: number;
  keyword?: string;
  pageSize?: number;
}): Promise<DemandPage> {
  return invoke<DemandPage>("list_demands", {
    acceptedByMe: input.acceptedByMe,
    current: input.current,
    keyword: input.keyword,
    pageSize: input.pageSize,
  });
}

export async function previewDemandPrd(demandId: number): Promise<DemandPrdPreview> {
  return invoke<DemandPrdPreview>("preview_demand_prd", { demandId });
}

export async function resolveDemandLink(url: string): Promise<DemandListItem> {
  return invoke<DemandListItem>("resolve_demand_link", { url });
}

export async function listDemandProject(input: {
  demandCode: string;
  demandId?: number;
}): Promise<DemandProject> {
  return invoke<DemandProject>("list_demand_project", {
    demandCode: input.demandCode,
    demandId: input.demandId,
  });
}

export async function readDemandProjectFile(input: {
  demandCode: string;
  demandId?: number;
  fileName: string;
}): Promise<DemandProjectFileContent> {
  return invoke<DemandProjectFileContent>("read_demand_project_file", {
    demandCode: input.demandCode,
    demandId: input.demandId,
    fileName: input.fileName,
  });
}

export async function listRepoBranches(id: string): Promise<string[]> {
  return invoke<string[]>("list_repo_branches", { id });
}

export async function syncRepo(id: string): Promise<RegisteredRepo> {
  return invoke<RegisteredRepo>("sync_repo", { id });
}

export async function updateRepoDefaultBranch(
  id: string,
  defaultBranch: string,
): Promise<RegisteredRepo> {
  return invoke<RegisteredRepo>("update_repo_default_branch", { defaultBranch, id });
}
