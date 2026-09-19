import type {
  AppConfig,
  AuthStatus,
  ChannelInfo,
  PipelineDetail,
  PipelineEvent,
  PipelineSummary,
  RegisteredRepo,
  SkillInfo,
  StreamChunk,
  ViewType,
} from "../lib/types";

export type Action =
  | { type: "hydrate"; pipelines: PipelineSummary[] }
  | { type: "pipelineAdded"; pipeline: PipelineSummary }
  | {
      type: "pipelineUpdated";
      id: string;
      status: string;
      currentStage: string;
      issueClass?: string | null;
      issueDetail?: string | null;
    }
  | { type: "pipelineSelected"; id: string | null }
  | { type: "detailLoaded"; detail: PipelineDetail }
  | { type: "eventReceived"; event: PipelineEvent }
  | {
      type: "humanRequest";
      pipelineId: string;
      stage: string;
      issueClass: string;
      detail: string;
    }
  | { type: "humanRequestDismissed"; pipelineId: string }
  | { type: "authChanged"; auth: AuthStatus }
  | { type: "sidecarStatus"; running: boolean; error?: string }
  | { type: "configLoaded"; config: AppConfig }
  | { type: "filterChanged"; filter: string | null }
  | { type: "skillsLoaded"; skills: SkillInfo[] }
  | { type: "channelsLoaded"; channels: ChannelInfo[] }
  | { type: "viewChanged"; view: ViewType }
  | { type: "reposHydrated"; repos: RegisteredRepo[] }
  | { type: "streamChunkReceived"; pipelineId: string; chunk: StreamChunk }
  | { type: "streamCleared"; pipelineId: string }
  | { type: "stageExecuteRequested"; pipelineId: string; stage: string };
