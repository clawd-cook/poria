# Desktop App Inventory

## Build Stack

- **Frontend**: React 19.1 + TypeScript 5.8 + Vite 6.3 + Tailwind CSS 4.1
- **Backend**: Tauri v2 (Rust, edition 2021)
- **Icons**: lucide-react
- **Dev port**: 1420
- **App ID**: `com.poria.desktop`
- **Window**: 1200×800 (min 900×600), `withGlobalTauri: true`

## 1. React Components

| Component         | File                                 | Purpose                                                                                                                                                          |
| ----------------- | ------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `App`             | `src/App.tsx`                        | Root; wraps `Shell` in `StoreProvider`                                                                                                                           |
| `Shell`           | `src/components/Shell.tsx`           | Main layout: top `SubmitBar`, left sidebar (`PipelineSidebar` + `AuthStatus` + settings gear + sidecar indicator), right `PipelineDetail`, modal `SettingsPanel` |
| `SubmitBar`       | `src/components/SubmitBar.tsx`       | Text input for pasting xingyun demand URL + submit button. Calls `submitPipeline(link)`                                                                          |
| `PipelineSidebar` | `src/components/PipelineSidebar.tsx` | Filter tabs (全部/运行中/已阻塞/已完成/已失败) + grouped pipeline list by status. Dispatches `pipelineSelected` and `filterChanged`                              |
| `PipelineDetail`  | `src/components/PipelineDetail.tsx`  | Full detail view: demand name/code/operator/link, `StageProgress` bar, `HumanLoopCard` (conditional), `EventStream`, `GateResults`, per-stage expandable list    |
| `StageProgress`   | `src/components/StageProgress.tsx`   | Horizontal progress bar showing 7 stages (init → review_prd → design → workspace → dev → cr → deploy) with status icons                                          |
| `StatusBadge`     | `src/components/StatusBadge.tsx`     | Colored pill showing pipeline status label (已创建/运行中/待合并/已阻塞/已完成/已失败/已取消)                                                                    |
| `EventStream`     | `src/components/EventStream.tsx`     | Scrollable log of `PipelineEvent`s with kind badges (stage_started, stage_completed, stage_failed, gate_passed, gate_failed, human_loop)                         |
| `GateResults`     | `src/components/GateResults.tsx`     | Table of gate results parsed from `StageDetail.gate_results` JSON (gate name, pass/fail, actual value, threshold)                                                |
| `HumanLoopCard`   | `src/components/HumanLoopCard.tsx`   | Alert card when pipeline needs human input. Three actions: resume/skip/cancel. Calls `humanLoopRespond(pipelineId, action)`                                      |
| `AuthStatus`      | `src/components/AuthStatus.tsx`      | Green/red dot + username or "未登录"                                                                                                                             |
| `SettingsPanel`   | `src/components/SettingsPanel.tsx`   | Modal dialog to edit `AppConfig` fields. Calls `updateConfig(config)`                                                                                            |

## 2. Tauri Invoke Calls (frontend → Rust backend)

All wrappers in `src/lib/tauri.ts`:

| Function                               | Tauri Command        | Args                     | Returns                         |
| -------------------------------------- | -------------------- | ------------------------ | ------------------------------- |
| `listPipelines()`                      | `list_pipelines`     | none                     | `PipelineSummary[]`             |
| `getPipeline(id)`                      | `get_pipeline`       | `{ id: string }`         | `PipelineDetail`                |
| `submitPipeline(link)`                 | `submit_pipeline`    | `{ link: string }`       | `string` (confirmation message) |
| `cancelPipeline(id)`                   | `cancel_pipeline`    | `{ id: string }`         | `void`                          |
| `humanLoopRespond(pipelineId, action)` | `human_loop_respond` | `{ pipelineId, action }` | `void`                          |
| `getAuthStatus()`                      | `get_auth_status`    | none                     | `AuthStatus`                    |
| `getConfig()`                          | `get_config`         | none                     | `AppConfig`                     |
| `updateConfig(config)`                 | `update_config`      | `{ config: AppConfig }`  | `void`                          |

## 3. Tauri Event Listeners (Rust backend → frontend)

Registered in **two places** (both `StoreProvider` init effect and `useTauriEvents` hook — the hook is defined but the store already subscribes directly):

| Event Name              | Payload Shape                               | Dispatch Action                    |
| ----------------------- | ------------------------------------------- | ---------------------------------- |
| `pipeline:list-changed` | none (triggers refetch)                     | `hydrate` (re-lists all pipelines) |
| `pipeline:updated`      | `{ id, status, currentStage }`              | `pipelineUpdated`                  |
| `stage:progress`        | `PipelineEvent`                             | `eventReceived`                    |
| `human:request`         | `{ pipelineId, stage, issueClass, detail }` | `humanRequest`                     |
| `sidecar:status`        | `{ running, error? }`                       | `sidecarStatus`                    |
| `auth:status-changed`   | `AuthStatus`                                | `authChanged`                      |

**Outbound events from Rust (emitted by commands):**

| Event Name              | Emitter                      | Purpose                                     |
| ----------------------- | ---------------------------- | ------------------------------------------- |
| `sidecar:submit`        | `submit_pipeline` command    | Tells Node sidecar to process a demand link |
| `sidecar:cancel`        | `cancel_pipeline` command    | Tells Node sidecar to cancel a pipeline     |
| `sidecar:human-respond` | `human_loop_respond` command | Sends human-loop response to sidecar        |

## 4. State Management

- **Pattern**: React `useReducer` + Context (`StoreProvider` / `useStore`)
- **State shape** (`AppState`):
  - `pipelines: PipelineSummary[]`
  - `selectedPipelineId: string | null`
  - `pipelineDetail: PipelineDetail | null`
  - `events: PipelineEvent[]`
  - `humanRequest: { pipelineId, stage, issueClass, detail } | null`
  - `auth: AuthStatus`
  - `config: AppConfig | null`
  - `sidecar: { running: boolean; error?: string }`
  - `ui: { filter: string | null; settingsOpen: boolean }`
- **Actions** (12 types): hydrate, pipelineAdded, pipelineUpdated, pipelineSelected, detailLoaded, eventReceived, humanRequest, humanRequestDismissed, authChanged, sidecarStatus, configLoaded, filterChanged, settingsToggled
- **Init effects**: on mount, fetches `listPipelines`, `getAuthStatus`, `getConfig` and subscribes to 6 Tauri events

## 5. Types (`src/lib/types.ts`)

### Enums/Unions

- `PipelineStatus`: `"created" | "running" | "waiting_merge" | "blocked" | "completed" | "failed" | "cancelled"`
- `StageStatus`: `"pending" | "running" | "completed" | "failed" | "blocked" | "skipped"`
- `StageEnum`: `"init" | "review_prd" | "design" | "workspace" | "dev" | "cr" | "deploy"`
- `STAGE_ORDER`: ordered array of `StageEnum`
- `STAGE_LABELS`: Chinese labels for each stage

### Interfaces

- **`PipelineSummary`**: `id, demand_name, demand_code, status, current_stage, created_at, updated_at`
- **`StageDetail`**: `name, status, retry_count, output_summary, gate_results, issue, started_at, completed_at`
- **`PipelineDetail`**: `id, demand_id, demand_code, demand_name, status, raw_link, operator, has_regressed, stages[], created_at, updated_at`
- **`PipelineEvent`**: `seq, kind, payload, created_at`
- **`AuthStatus`**: `logged_in, username, cookie_valid`
- **`AppConfig`**: `cr_score_threshold, test_coverage_threshold, max_diff_lines, agent_timeout_ms, max_retries, db_path`

## 6. Hooks

| Hook             | File                          | Purpose                                                                                                                              |
| ---------------- | ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| `usePipeline`    | `src/hooks/usePipeline.ts`    | Fetches `PipelineDetail` when `selectedPipelineId` changes; returns `{ detail, events, humanRequest }`                               |
| `useTauriEvents` | `src/hooks/useTauriEvents.ts` | Subscribes to the same 6 Tauri events. **Note**: appears unused — `StoreProvider` already subscribes directly in its own `useEffect` |

## 7. Tauri Config Summary

- **Plugins**: shell, notification, opener
- **Capabilities**: `default.json` (not read, presumably standard)
- **Rust crate**: `poria-desktop`, edition 2021
- **Rust deps**: tauri 2, tauri-plugin-{shell,notification,opener}, serde, serde_json, rusqlite (bundled), dirs, chrono, tokio (sync)
- **Rust modules**: `lib.rs` (app setup + state), `db.rs` (read-only SQLite queries + schema DDL), `commands/{mod,pipeline,auth,config}.rs`

## 8. Key Architectural Notes

- The Rust backend is currently **read-only** for the database (WAL mode, read-only flags). Writes happen via a **Node.js sidecar** that uses `better-sqlite3`.
- `submit_pipeline`, `cancel_pipeline`, and `human_loop_respond` all just **emit Tauri events** for the sidecar to process — they don't write to the DB themselves.
- `get_auth_status` reads `~/.poria/auth.json` directly from the filesystem.
- `get_config` / `update_config` reads/writes `~/.poria/config.json`.
- Frontend is pure dark theme (slate-900 background), Chinese UI text.
