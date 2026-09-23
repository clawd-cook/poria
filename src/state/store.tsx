import { listen } from "@tauri-apps/api/event";
import {
  createContext,
  useContext,
  useReducer,
  useEffect,
  type ReactNode,
  type Dispatch,
} from "react";

import { loadHitlAutoNavigate } from "../lib/hitlPrefs";
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
  UiSurface,
  WorkbenchTab,
} from "../lib/types";
import type { Action } from "./actions";

export interface AppUiState {
  filter: string | null;
  hitlAutoNavigate: boolean;
  surface: UiSurface;
  view: ViewType;
  workbenchTab: WorkbenchTab;
}

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
  ui: AppUiState;
}

function coerceView(view: string): ViewType {
  if (view === "repos" || view === "settings") {
    return view;
  }
  return "home";
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
  ui: {
    filter: null,
    hitlAutoNavigate: loadHitlAutoNavigate(),
    surface: "list",
    view: "home",
    workbenchTab: "trajectory",
  },
};

function retainHumanRequest(
  state: AppState,
  nextPipelineId: string | null,
): AppState["humanRequest"] {
  if (!state.humanRequest) {
    return null;
  }
  if (nextPipelineId === null) {
    return state.humanRequest;
  }
  if (state.humanRequest.pipelineId === nextPipelineId) {
    return state.humanRequest;
  }
  return null;
}

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
        humanRequest:
          state.humanRequest?.pipelineId === action.id &&
          (action.status === "running" || action.status === "cancelled")
            ? null
            : state.humanRequest,
        pipelines: state.pipelines.map((p) =>
          p.id === action.id
            ? {
                ...p,
                current_stage: action.currentStage,
                issue_class:
                  action.issueClass === undefined ? p.issue_class : (action.issueClass ?? null),
                issue_detail:
                  action.issueDetail === undefined ? p.issue_detail : (action.issueDetail ?? null),
                status: action.status as PipelineSummary["status"],
                updated_at: new Date().toISOString(),
              }
            : p,
        ),
      };

    case "pipelineSelected": {
      if (action.id === state.selectedPipelineId) {
        return {
          ...state,
          ui: {
            ...state.ui,
            surface: action.id ? "workbench" : "list",
          },
        };
      }
      return {
        ...state,
        events: [],
        humanRequest: retainHumanRequest(state, action.id),
        pipelineDetail: null,
        selectedPipelineId: action.id,
        ui: {
          ...state.ui,
          surface: action.id ? "workbench" : "list",
          workbenchTab: action.id ? state.ui.workbenchTab : state.ui.workbenchTab,
        },
      };
    }

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
        pipelines: state.pipelines.map((pipeline) =>
          pipeline.id === action.pipelineId
            ? {
                ...pipeline,
                issue_class: action.issueClass,
                issue_detail: action.detail,
              }
            : pipeline,
        ),
      };

    case "openHitlWorkbench":
      return {
        ...state,
        events: action.pipelineId === state.selectedPipelineId ? state.events : [],
        pipelineDetail:
          action.pipelineId === state.selectedPipelineId ? state.pipelineDetail : null,
        selectedPipelineId: action.pipelineId,
        ui: {
          ...state.ui,
          surface: "workbench",
          view: "home",
          workbenchTab: "confirm",
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
      return {
        ...state,
        ui: { ...state.ui, view: coerceView(action.view) },
      };

    case "surfaceChanged":
      return {
        ...state,
        ui: { ...state.ui, surface: action.surface },
      };

    case "workbenchTabChanged":
      return {
        ...state,
        ui: { ...state.ui, workbenchTab: action.tab },
      };

    case "hitlAutoNavigateChanged":
      return {
        ...state,
        ui: { ...state.ui, hitlAutoNavigate: action.enabled },
      };

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
      listen<{
        id: string;
        status: string;
        currentStage: string;
        issueClass?: string | null;
        issueDetail?: string | null;
      }>("pipeline:updated", (e) => {
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
        if (loadHitlAutoNavigate()) {
          dispatch({ pipelineId: e.payload.pipelineId, type: "openHitlWorkbench" });
        }
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
