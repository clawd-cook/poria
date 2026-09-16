import type {
  PipelineSummary,
  PipelineDetail,
  PipelineEvent,
  AuthStatus,
  AppConfig,
} from "../lib/types";

export type Action =
  | { type: "hydrate"; pipelines: PipelineSummary[] }
  | { type: "pipelineAdded"; pipeline: PipelineSummary }
  | {
      type: "pipelineUpdated";
      id: string;
      status: string;
      currentStage: string;
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
  | { type: "settingsToggled"; open: boolean };
