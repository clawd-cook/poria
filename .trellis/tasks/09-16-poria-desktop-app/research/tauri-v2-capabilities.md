# Tauri v2 Capabilities — Poria Desktop App

## IPC Patterns

Three primitives, each maps to a Poria use case:

| Primitive | Direction | Use in Poria |
|-----------|-----------|-------------|
| **Commands** (`invoke`) | Frontend→Rust→Frontend | Pipeline CRUD, auth login, config read/write |
| **Events** (`emit`/`listen`) | Bidirectional, fire-and-forget | Pipeline state changes broadcast, human-loop notifications |
| **Channels** (`Channel<T>`) | Rust→Frontend streaming | Real-time 7-stage progress, event log streaming |

**Key for Poria**: Pipeline execution progress is the perfect Channel use case — `Channel<PipelineEvent>` with a tagged union (`#[serde(tag="event", content="data")]`) maps directly to the existing `PipelineEvent` types in `@poria/core`.

## Plugin Ecosystem (Actionable for MVP)

| Plugin | Poria Use | Priority |
|--------|-----------|----------|
| `tauri-plugin-shell` | Sidecar (Node.js worker) + git operations | P0 |
| `tauri-plugin-notification` | Pipeline completion / human-loop alerts | P0 |
| `tauri-plugin-store` | User preferences, gate thresholds | P1 |
| `tauri-plugin-dialog` | File picker for project paths | P1 |
| `tauri-plugin-opener` | Open MR URLs, worktree paths | P1 |
| `tauri-plugin-updater` | Auto-update desktop app | P2 |
| `tauri-plugin-deep-link` | `poria://submit?link=...` from browser | P2 |

**No tauri-plugin-sql needed** — our SQLite lives in the Node.js sidecar (better-sqlite3 via `@poria/infrastructure`). Rust side only needs to read pipeline status via IPC to the sidecar.

## Sidecar Pattern (Critical Path)

Tauri sidecars bundle an executable into the app. For Poria:

1. **Bundle a Node.js sidecar** compiled from `@poria/commands` + `@poria/infrastructure` using `pkg` or `sea` (Node 24 single-executable-applications)
2. Config: `"bundle": { "externalBin": ["binaries/poria-worker"] }`
3. Naming convention: `poria-worker-aarch64-apple-darwin` (macOS ARM)
4. Permission: `shell:allow-execute` scoped to `{ "name": "poria-worker", "sidecar": true }`
5. Rust spawns via `app.shell().sidecar("poria-worker").args([...]).spawn()`

**Alternative**: Skip sidecar, implement Rust commands that call `@poria/infrastructure` SQLite directly via `rusqlite`. The Node-dependent parts (Agent SDK, channels) run as a background process managed by Rust. This is cleaner but requires more Rust code.

**Recommended**: Hybrid — Rust owns SQLite reads (status queries, UI data) via `rusqlite`; Node sidecar runs the PipelineWorker (Agent SDK + channel I/O). Rust ↔ sidecar communicate via stdout JSON lines or local HTTP.

## SQLite Sharing

Both Rust (`rusqlite`) and Node (`better-sqlite3`) can open the same SQLite file. SQLite handles concurrent readers. Write contention is handled by WAL mode (already set in our schema). **No special integration needed** — just point both at the same `workspace/db/poria.db` path.

## Window Management

- Main window: Pipeline dashboard (list + detail)
- System tray: `TrayIconBuilder` with status indicator + quick actions (submit, open dashboard)
- Notifications: native OS notifications for pipeline completion / human-loop requests
- No multi-window needed for MVP

## Build / Packaging

- macOS primary: `cargo tauri build` → `.dmg` + `.app`
- Scaffolding: `npm create tauri-app@latest` with React + Vite template
- Frontend in `src/`, Rust in `src-tauri/`
- `beforeDevCommand: "pnpm dev"`, `devUrl: "http://localhost:5173"`
- Production: `frontendDist: "../dist"`

## React + Vite Integration

Standard Tauri v2 scaffold with `@vitejs/plugin-react` + Tailwind CSS (matching OpenMausBot pattern). State management via `useReducer` + Tauri event listener fold — same pattern OpenMausBot uses with SSE, but replacing SSE with Tauri `listen()`.
