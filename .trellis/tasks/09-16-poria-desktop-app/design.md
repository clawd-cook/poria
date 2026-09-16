# Poria Desktop App — Design

## 架构概览

```
┌────────────────────────────────────────────────────────┐
│  Tauri v2 App                                          │
│                                                        │
│  ┌──────────────────────┐  ┌────────────────────────┐  │
│  │  React Frontend      │  │  Rust Backend           │  │
│  │  (Vite + Tailwind)   │  │  (src-tauri/)           │  │
│  │                      │  │                          │  │
│  │  useReducer Store ◄──┤──┤  Commands (invoke)       │  │
│  │       ▲              │  │  - list_pipelines        │  │
│  │       │ listen()     │  │  - get_pipeline          │  │
│  │       │              │  │  - submit_pipeline       │  │
│  │  Tauri Events ◄──────┤──┤  - human_loop_respond    │  │
│  │                      │  │  - get_auth_status       │  │
│  │  Components:         │  │  - update_config         │  │
│  │  - PipelineSidebar   │  │                          │  │
│  │  - StageProgress     │  │  Events (emit):          │  │
│  │  - EventStream       │  │  - pipeline:updated      │  │
│  │  - HumanLoopCard     │  │  - stage:progress        │  │
│  │  - SettingsPanel     │  │  - human:request         │  │
│  └──────────────────────┘  │                          │  │
│                            │  SQLite (rusqlite, RO)   │  │
│                            │  ┌──────────────────┐    │  │
│                            │  │ workspace/db/     │    │  │
│                            │  │ poria.db (WAL)   │◄───┤──┤── Node Sidecar
│                            │  └──────────────────┘    │  │   (PipelineWorker)
│                            │                          │  │   writes via
│                            │  Sidecar Management:     │  │   better-sqlite3
│                            │  - spawn / health check  │  │
│                            │  - stdout JSON lines     │  │
│                            └────────────────────────┘  │
└────────────────────────────────────────────────────────┘
```

## 1. 目录结构

```
apps/
└── desktop/
    ├── package.json              # React 前端依赖
    ├── vite.config.ts
    ├── tailwind.config.ts
    ├── tsconfig.json
    ├── index.html
    ├── src/                      # React 前端
    │   ├── main.tsx
    │   ├── App.tsx
    │   ├── styles.css            # Tailwind 入口
    │   ├── state/
    │   │   ├── store.tsx         # useReducer + Tauri event fold
    │   │   └── actions.ts        # Action union type
    │   ├── components/
    │   │   ├── Shell.tsx         # 主布局 (sidebar + main)
    │   │   ├── PipelineSidebar.tsx
    │   │   ├── PipelineList.tsx
    │   │   ├── PipelineDetail.tsx
    │   │   ├── StageProgress.tsx   # 7-stage 横向步骤条
    │   │   ├── EventStream.tsx     # 实时事件流
    │   │   ├── HumanLoopCard.tsx   # 人工回路操作卡片
    │   │   ├── SubmitBar.tsx       # 行云链接输入框
    │   │   ├── GateResults.tsx     # 门禁结果
    │   │   ├── SettingsPanel.tsx
    │   │   ├── AuthStatus.tsx
    │   │   └── StatusBadge.tsx
    │   ├── lib/
    │   │   ├── tauri.ts          # invoke/listen 封装
    │   │   └── types.ts          # 前端类型定义
    │   └── hooks/
    │       ├── usePipeline.ts
    │       └── useTauriEvents.ts
    └── src-tauri/                # Rust backend
        ├── Cargo.toml
        ├── tauri.conf.json
        ├── capabilities/
        │   └── default.json
        ├── src/
        │   ├── main.rs
        │   ├── lib.rs
        │   ├── commands/         # Tauri commands
        │   │   ├── mod.rs
        │   │   ├── pipeline.rs   # list, get, submit, respond
        │   │   ├── auth.rs
        │   │   └── config.rs
        │   ├── db.rs             # rusqlite 只读查询
        │   ├── sidecar.rs        # Node sidecar 管理
        │   ├── tray.rs           # 系统托盘
        │   └── events.rs         # 事件桥接 (sidecar → frontend)
        └── icons/
```

## 2. Rust-Frontend IPC 契约

### Commands (Frontend → Rust → Frontend)

```rust
#[tauri::command]
async fn list_pipelines(state: State<AppState>) -> Result<Vec<PipelineSummary>, String>;

#[tauri::command]
async fn get_pipeline(id: String, state: State<AppState>) -> Result<PipelineDetail, String>;

#[tauri::command]
async fn submit_pipeline(link: String, state: State<AppState>) -> Result<String, String>;

#[tauri::command]
async fn human_loop_respond(pipeline_id: String, action: String, state: State<AppState>) -> Result<(), String>;

#[tauri::command]
async fn get_auth_status(state: State<AppState>) -> Result<AuthStatus, String>;

#[tauri::command]
async fn get_config(state: State<AppState>) -> Result<AppConfig, String>;

#[tauri::command]
async fn update_config(config: AppConfig, state: State<AppState>) -> Result<(), String>;

#[tauri::command]
async fn cancel_pipeline(id: String, state: State<AppState>) -> Result<(), String>;

#[tauri::command]
async fn rollback_pipeline(id: String, state: State<AppState>) -> Result<(), String>;
```

### Events (Rust → Frontend)

```typescript
// 前端 listen 的事件类型
type TauriEvent =
  | { event: "pipeline:list-changed" }
  | { event: "pipeline:updated"; data: { id: string; status: string; currentStage: string } }
  | { event: "stage:progress"; data: { pipelineId: string; stage: string; event: PipelineEvent } }
  | { event: "human:request"; data: { pipelineId: string; stage: string; issueClass: string; detail: string } }
  | { event: "auth:status-changed"; data: AuthStatus }
  | { event: "sidecar:status"; data: { running: boolean; error?: string } };
```

## 3. 状态管理 (参考 OpenMausBot)

```typescript
// state/store.tsx — 参考 OpenMausBot 的 useReducer 模式

interface AppState {
  pipelines: PipelineSummary[];
  selectedPipelineId: string | null;
  pipelineDetail: PipelineDetail | null;
  events: PipelineEvent[];  // 当前选中 pipeline 的事件流
  auth: AuthStatus;
  config: AppConfig;
  sidecar: { running: boolean; error?: string };
  ui: { sidebarFilter: string | null; settingsOpen: boolean };
}

type Action =
  | { type: "hydrate"; pipelines: PipelineSummary[] }
  | { type: "pipelineAdded"; pipeline: PipelineSummary }
  | { type: "pipelineUpdated"; id: string; status: string; currentStage: string }
  | { type: "pipelineSelected"; id: string }
  | { type: "detailLoaded"; detail: PipelineDetail }
  | { type: "eventReceived"; event: PipelineEvent }
  | { type: "humanRequest"; pipelineId: string; stage: string; issueClass: string; detail: string }
  | { type: "authChanged"; auth: AuthStatus }
  | { type: "sidecarStatus"; running: boolean; error?: string }
  | { type: "configLoaded"; config: AppConfig }
  | { type: "filterChanged"; filter: string | null }
  | { type: "settingsToggled"; open: boolean };
```

Tauri event listener 替代 OpenMausBot 的 SSE fold：

```typescript
useEffect(() => {
  const unlisten = Promise.all([
    listen("pipeline:list-changed", () => refreshPipelines()),
    listen("pipeline:updated", (e) => dispatch({ type: "pipelineUpdated", ...e.payload })),
    listen("stage:progress", (e) => dispatch({ type: "eventReceived", event: e.payload })),
    listen("human:request", (e) => { dispatch({ type: "humanRequest", ...e.payload }); }),
    listen("sidecar:status", (e) => dispatch({ type: "sidecarStatus", ...e.payload })),
  ]);
  return () => { unlisten.then(fns => fns.forEach(fn => fn())); };
}, []);
```

## 4. Sidecar 管理

Rust 侧管理 Node sidecar 生命周期：

```rust
// sidecar.rs
struct SidecarManager {
    child: Option<CommandChild>,
}

impl SidecarManager {
    fn spawn(app: &AppHandle) -> Result<Self> {
        // poria-worker sidecar, 通过 stdout JSON lines 通信
        let (rx, child) = app.shell()
            .sidecar("poria-worker")
            .expect("sidecar not found")
            .args(["--mode", "daemon", "--db", &db_path])
            .spawn()
            .expect("failed to spawn sidecar");

        // 读 stdout 事件 → emit 到前端
        tauri::async_runtime::spawn(async move {
            while let Some(event) = rx.recv().await {
                if let CommandEvent::Stdout(line) = event {
                    // 解析 JSON line → emit Tauri event
                }
            }
        });
    }
}
```

SQLite 共享：Rust (rusqlite, WAL mode, 只读) 和 Node sidecar (better-sqlite3, 读写) 同时访问 `workspace/db/poria.db`。

## 5. 关键 UI 组件设计

### StageProgress (7-stage 横向步骤条)

```
[✅ init] → [✅ review_prd] → [✅ design] → [🔄 workspace] → [⬜ dev] → [⬜ cr] → [⬜ deploy]
```

每个节点：圆形图标 + 阶段名 + 连接线。状态映射：
- completed → 绿色勾
- running → 蓝色旋转
- pending → 灰色空心
- failed → 红色叉
- blocked → 橙色感叹号
- skipped → 灰色划线

### HumanLoopCard (参考 OpenMausBot ApprovalCard)

```
┌─────────────────────────────────────────┐
│ ⚠️ Pipeline 需要协助                    │
│                                         │
│ 问题: compilation_error                 │
│ 阶段: dev                               │
│ 详情: tsc: 3 errors found...            │
│                                         │
│ [✅ 修复并重试] [⏭ 跳过] [❌ 取消]       │
└─────────────────────────────────────────┘
```

## 6. Tauri 插件清单

| 插件 | 用途 | 配置 |
|------|------|------|
| tauri-plugin-shell | Sidecar 管理 | `shell:allow-execute` scoped to poria-worker |
| tauri-plugin-notification | 系统通知 | 默认 |
| tauri-plugin-store | 用户偏好 | 默认 |
| tauri-plugin-dialog | 文件/目录选择 | 默认 |
| tauri-plugin-opener | 打开 MR URL | 默认 |
