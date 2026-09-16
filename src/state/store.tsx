import { listen } from "@tauri-apps/api/event";
import {
  createContext,
  useContext,
  useReducer,
  useEffect,
  type ReactNode,
  type Dispatch,
} from "react";

import { listPipelines, getAuthStatus, getConfig } from "../lib/tauri";
import type {
  PipelineSummary,
  PipelineDetail,
  PipelineEvent,
  AuthStatus,
  AppConfig,
  SkillInfo,
  ChannelInfo,
  ViewType,
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
  ui: { filter: string | null; settingsOpen: boolean; view: ViewType };
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
  ui: { filter: null, settingsOpen: false, view: "pipeline" },
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
                status: action.status as PipelineSummary["status"],
                current_stage: action.currentStage,
              }
            : p,
        ),
      };

    case "pipelineSelected":
      return {
        ...state,
        selectedPipelineId: action.id,
        pipelineDetail: action.id === null ? null : state.pipelineDetail,
        events: action.id === null ? [] : state.events,
        humanRequest: null,
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

    case "settingsToggled":
      return { ...state, ui: { ...state.ui, settingsOpen: action.open } };

    case "skillsLoaded":
      return { ...state, skills: action.skills };

    case "channelsLoaded":
      return { ...state, channels: action.channels };

    case "viewChanged":
      return { ...state, ui: { ...state.ui, view: action.view } };

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
  }, []);

  useEffect(() => {
    const unlisten = Promise.all([
      listen("pipeline:list-changed", () => {
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
