# OpenMausBot Architecture Patterns — Poria Adaptation Notes

## 1. Server-Client Architecture

OpenMausBot uses a **Node.js harness server** (`server/`) that React talks to via two channels:

- **HTTP dispatch**: UI dispatches typed actions → `fetch("/api/bots/{id}/messages", { POST })`. Every mutation goes through HTTP to the server, never directly to state.
- **SSE event stream**: `GET /api/events` delivers a single `EventSource` connection. Server pushes `LiveFrame` objects with `kind` field: `"message"`, `"bot"`, `"runtime"`, `"config"`, `"notify"`, etc. The `openLiveEvents()` supervisor handles reconnects, replay cursors, and liveness pings (40s stale timeout).
- **Runtime events** (`server/contracts.ts`): A `RuntimeEvent` union (~15 types) describes agent lifecycle: `session.started`, `turn.started/completed`, `item.started/completed`, `content.delta`, `request.opened/resolved`, `runtime.error`. These flow through SSE → store reducer.

**Poria adaptation**: Replace HTTP API + SSE with **Tauri IPC** (`invoke` for commands, `listen` for events). The Rust backend emits events through Tauri's event system; the React frontend folds them identically.

## 2. State Management

- **Pattern**: `useReducer` + wrapped dispatch. The `reducer()` is pure; all async (HTTP calls, SSE subscription) lives in the dispatch wrapper.
- **Action union**: ~80 action types covering hydrate, bot/group CRUD, message add/patch, card answer/dismiss, task management, model selection, UI toggles.
- **SSE fold**: `handleFrame()` in StoreProvider maps each `LiveFrame.kind` to a `rawDispatch()` call — e.g., `frame.kind === "message"` → `dispatch({ type: "messageAdded" })`. Content deltas are buffered and flushed per animation frame.
- **BotPatchQueue** (`bot-patch-queue.ts`): Optimistic updates for bot settings. Patches are queued locally, overlaid onto server state, and confirmed/rejected asynchronously. Prevents race conditions when multiple settings change rapidly.
- **Background thread events**: Messages arriving for non-visible threads are buffered in `backgroundThreadEvents` and replayed on switch.

**Poria adaptation**: Same useReducer pattern. Action union maps to Pipeline lifecycle (submit, stage progress, gate result, human-loop request, MR status). Tauri event listener replaces SSE fold.

## 3. Key UI Components

| OpenMausBot | Poria Equivalent |
|---|---|
| **ChatView** — scrollable message list, branching, streaming deltas | **PipelineTimeline** — stage progress with event log |
| **ApprovalCard** — tool permission ask with Allow/Deny/Always | **HumanLoopCard** — pipeline needs human input (fix/skip/cancel) |
| **ActivityRun** — tool call chip (name, ok/fail, summary) | **StageActivity** — skill execution chip (stage name, status) |
| **Sidebar** — bot list, sections, unread badges | **PipelineSidebar** — pipeline list, status badges |
| **SettingsModal** — API keys, model config, approval mode | **SettingsPanel** — gate thresholds, timeout config, auth |
| **BotSettingsDialog** — per-bot name/model/approval/tools | **PipelineDetail** — per-pipeline config, demand metadata |

Notable patterns: Card-based interaction (OptionCardData with `answered`/`dismissed` state), lucide-react icons, Tailwind CSS classes, i18n via `t()` function.

## 4. Agent Lifecycle

- **Bot = Agent instance**: Each bot has an `id`, `threadId`, tasks (separate conversations), `modelSelection`, `approvalMode`, `alwaysAllow` tools, `activity` state (working/waiting-on-you/idle/dead).
- **Sending a turn**: `POST /api/bots/{id}/messages` → server routes to the provider driver → driver spawns/resumes CLI process → runtime events stream back.
- **Approval flow**: Provider raises `request.opened` (permission/question) → UI shows ApprovalCard → user decides → `POST /api/threads/{id}/respond` → provider continues.
- **Auto-approve**: Bots can have `autoApprove: true` or `approvalMode: "auto"` where the harness answers tool requests without human input.
- **Task management**: Each bot can have multiple tasks (threads), switchable. Tasks have independent model selection, approval mode, usage tracking.

**Poria adaptation**: A Pipeline replaces a Bot as the primary entity. Stages replace turns. The human-loop notification (JME) maps to the ApprovalCard pattern — instead of tool permission, it's "fix/skip/cancel this blocked stage."

## 5. Desktop Integration (Electron)

- **Single instance**: `single-instance.mjs` — file lock + IPC to activate existing window.
- **Server supervisor**: `server-supervisor.mjs` — Electron spawns the Node.js server as a child process, monitors health, restarts on crash.
- **Auto-updater**: `updater.mjs` — electron-updater with status states (idle/checking/available/downloading/downloaded/error). IPC bridge to renderer for update banner.
- **Tray**: Not prominent; window management via BrowserWindow.
- **Notifications**: `showNotification()` in lib — native OS notifications routed through the server's `notify` SSE frame.
- **Preload bridge**: Electron preload exposes safe APIs (`window.ogb`), desktop capabilities detected at runtime.

**Poria adaptation**: Tauri v2 handles all of these natively — single instance via plugin, auto-updater via `tauri-plugin-updater`, system tray via `tauri-plugin-tray`, notifications via `tauri-plugin-notification`. The Rust backend replaces both Electron main process and Node.js server.
