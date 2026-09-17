import { listen } from "@tauri-apps/api/event";
import {
  createContext,
  useContext,
  useReducer,
  useEffect,
  type ReactNode,
  type Dispatch,
} from "react";

import { getAuthStatus, getConfig, listPipelines, listRepos } from "../lib/tauri";
import type {
  PipelineSummary,
  PipelineDetail,
  PipelineEvent,
  AuthStatus,
  AppConfig,
  SkillInfo,
  ChannelInfo,
  StreamChunk,
  ViewType,
  RegisteredRepo,
} from "../lib/types";
import type { Action } from "./actions";

export interface AppState {
  pipelines: PipelineSummary[];
  selectedPipelineId: string | null;
  pipelineDetail: PipelineDetail | null;
  events: PipelineEvent[];
  humanRequest: {
    pipelineId: string;
    stage: string;
    issueClass: string;
    detail: string;
  } | null;
  auth: AuthStatus;
  config: AppConfig | null;
  sidecar: { running: boolean; error?: string };
  skills: SkillInfo[];
  channels: ChannelInfo[];
  streamOutput: Record<string, StreamChunk[]>;
  repos: RegisteredRepo[];
  ui: { filter: string | null; view: ViewType };
}

const initialState: AppState = {
  pipelines: [],
  selectedPipelineId: null,
  pipelineDetail: null,
  events: [],
  humanRequest: null,
  auth: { logged_in: false, username: null, cookie_valid: false },
  config: null,
  sidecar: { running: false },
  skills: [],
  channels: [],
  streamOutput: {},
  repos: [],
  ui: { filter: null, view: "home" },
};

function reducer(state: AppState, action: Action): AppState {
  switch (action.type) {
    case "hydrate":
      return { ...state, pipelines: action.pipelines };

    case "pipelineAdded":
      return {
        ...state,
        pipelines: [action.pipeline, ...state.pipelines],
      };

    case "pipelineUpdated":
      return {
        ...state,
        pipelines: state.pipelines.map((p) =>
          p.id === action.id
            ? {
                ...p,
                current_stage: action.currentStage,
                status: action.status as PipelineSummary["status"],
                updated_at: new Date().toISOString(),
              }
            : p,
        ),
      };

    case "pipelineSelected":
      if (action.id === state.selectedPipelineId) {
        return { ...state, humanRequest: null };
      }
      return {
        ...state,
        events: [],
        humanRequest: null,
        pipelineDetail: null,
        selectedPipelineId: action.id,
      };

    case "detailLoaded":
      return { ...state, pipelineDetail: action.detail };

    case "eventReceived":
      return { ...state, events: [...state.events, action.event] };

    case "humanRequest":
      return {
        ...state,
        humanRequest: {
          pipelineId: action.pipelineId,
          stage: action.stage,
          issueClass: action.issueClass,
          detail: action.detail,
        },
      };

    case "humanRequestDismissed":
      return {
        ...state,
        humanRequest:
          state.humanRequest?.pipelineId === action.pipelineId ? null : state.humanRequest,
      };

    case "authChanged":
      return { ...state, auth: action.auth };

    case "sidecarStatus":
      return {
        ...state,
        sidecar: { running: action.running, error: action.error },
      };

    case "configLoaded":
      return { ...state, config: action.config };

    case "filterChanged":
      return { ...state, ui: { ...state.ui, filter: action.filter } };

    case "skillsLoaded":
      return { ...state, skills: action.skills };

    case "channelsLoaded":
      return { ...state, channels: action.channels };

    case "viewChanged":
      return { ...state, ui: { ...state.ui, view: action.view } };

    case "reposHydrated":
      return { ...state, repos: action.repos };

    case "streamChunkReceived": {
      const prev = state.streamOutput[action.pipelineId] ?? [];
      return {
        ...state,
        streamOutput: {
          ...state.streamOutput,
          [action.pipelineId]: [...prev, action.chunk],
        },
      };
    }

    case "streamCleared": {
      const { [action.pipelineId]: _, ...rest } = state.streamOutput;
      return { ...state, streamOutput: rest };
    }

    case "stageExecuteRequested":
      return state;

    default:
      return state;
  }
}

const StoreContext = createContext<{
  state: AppState;
  dispatch: Dispatch<Action>;
}>({ state: initialState, dispatch: () => {} });

export function StoreProvider({ children }: { children: ReactNode }) {
  const [state, dispatch] = useReducer(reducer, initialState);

  useEffect(() => {
    listPipelines()
      .then((pipelines) => dispatch({ type: "hydrate", pipelines }))
      .catch(() => {});
    getAuthStatus()
      .then((auth) => dispatch({ type: "authChanged", auth }))
      .catch(() => {});
    getConfig()
      .then((config) => dispatch({ type: "configLoaded", config }))
      .catch(() => {});
    listRepos()
      .then((repos) => dispatch({ type: "reposHydrated", repos }))
      .catch(() => {});
  }, []);

  useEffect(() => {
    const unlisten = Promise.all([
      listen("pipeline:list-changed", () => {
        listPipelines()
          .then((pipelines) => dispatch({ type: "hydrate", pipelines }))
          .catch(() => {});
      }),
      listen<string>("pipeline:created", () => {
        listPipelines()
          .then((pipelines) => dispatch({ type: "hydrate", pipelines }))
          .catch(() => {});
      }),
      listen<{ id: string; status: string; currentStage: string }>("pipeline:updated", (e) => {
        dispatch({ type: "pipelineUpdated", ...e.payload });
      }),
      listen<PipelineEvent>("stage:progress", (e) => {
        dispatch({ type: "eventReceived", event: e.payload });
      }),
      listen<{
        pipelineId: string;
        stage: string;
        issueClass: string;
        detail: string;
      }>("human:request", (e) => {
        dispatch({ type: "humanRequest", ...e.payload });
      }),
      listen<{ running: boolean; error?: string }>("sidecar:status", (e) => {
        dispatch({ type: "sidecarStatus", ...e.payload });
      }),
      listen<AuthStatus>("auth:status-changed", (e) => {
        dispatch({ type: "authChanged", auth: e.payload });
      }),
      listen("repo:updated", () => {
        listRepos()
          .then((repos) => dispatch({ type: "reposHydrated", repos }))
          .catch(() => {});
      }),
      listen<{ pipeline_id: string; chunk_type: string; content: string; tool_name?: string }>(
        "agent:stream",
        (e) => {
          dispatch({
            type: "streamChunkReceived",
            pipelineId: e.payload.pipeline_id,
            chunk: {
              type: e.payload.chunk_type as StreamChunk["type"],
              content: e.payload.content,
              timestamp: new Date().toISOString(),
              tool_name: e.payload.tool_name,
            },
          });
        },
      ),
    ]);

    return () => {
      unlisten.then((fns) => fns.forEach((fn) => fn()));
    };
  }, []);

  return <StoreContext.Provider value={{ state, dispatch }}>{children}</StoreContext.Provider>;
}

export function useStore() {
  return useContext(StoreContext);
}
