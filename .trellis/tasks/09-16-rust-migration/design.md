# Technical Design — Rust Migration

## Architecture Overview

```
crates/
├── poria-core/          # 类型、合约 trait、状态机、事件、门控
├── poria-infrastructure/# SQLite store、auth、config、logger、metrics
├── poria-commands/      # Pipeline 命令层
├── poria-resources/     # Terminal、Worktree、Agent Pool
├── poria-skills/        # 7 个 Skill + HumanLoop
└── poria-channels/      # xingyun, coding, jme, joyspace, defect

apps/desktop/src-tauri/  # Tauri v2 后端，依赖所有 crates
apps/desktop/src/        # React 前端
```

### Cargo Workspace

根目录新增 `Cargo.toml`:

```toml
[workspace]
resolver = "2"
members = [
  "crates/poria-core",
  "crates/poria-infrastructure",
  "crates/poria-commands",
  "crates/poria-resources",
  "crates/poria-skills",
  "crates/poria-channels",
  "apps/desktop/src-tauri",
]

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rusqlite = { version = "0.32", features = ["bundled"] }
tokio = { version = "1", features = ["full"] }
chrono = { version = "0.4", features = ["serde"] }
thiserror = "2"
tracing = "0.1"
async-trait = "0.1"
nanoid = "0.4"
glob = "0.3"
dirs = "6"
reqwest = { version = "0.12", features = ["json", "cookies"] }
```

## Crate Design

### 1. poria-core

**纯类型 + 纯函数，零外部 IO 依赖。**

#### Types (types.rs)

TypeScript `interface` → Rust `struct` with `#[derive(Debug, Clone, Serialize, Deserialize)]`

TypeScript `type A = "x" | "y"` → Rust `enum` with `#[serde(rename_all = "snake_case")]`

关键映射：

| TypeScript                            | Rust                                                              |
| ------------------------------------- | ----------------------------------------------------------------- |
| `Pipeline`                            | `struct Pipeline`                                                 |
| `Stage`                               | `struct Stage`                                                    |
| `PipelineStatus` (union)              | `enum PipelineStatus`                                             |
| `StageStatus` (union)                 | `enum StageStatus`                                                |
| `StageEnum` (union)                   | `enum StageEnum`                                                  |
| `DemandMetadata`                      | `struct DemandMetadata`                                           |
| `GateRule/GateResult/GateEvaluation`  | `struct GateRule` / `struct GateResult` / `struct GateEvaluation` |
| `IssueClass` (enum)                   | `enum IssueClass`                                                 |
| `IssuePolicy`                         | `struct IssuePolicy` + `lazy_static` map                          |
| `PipelineConfig`                      | `struct PipelineConfig`                                           |
| `RepoConfig`                          | `struct RepoConfig`                                               |
| `RollbackInstruction/RollbackCommand` | `struct RollbackInstruction` / `struct RollbackCommand`           |
| `AgentTaskInput/AgentTaskResult`      | `struct AgentTaskInput` / `struct AgentTaskResult`                |
| `TIMEOUT`                             | `const` 常量                                                      |

#### Contracts (contracts.rs)

TypeScript `interface IChannel<T,R>` → Rust `#[async_trait] trait`:

```rust
#[async_trait]
pub trait Channel: Send + Sync {
    fn metadata(&self) -> &CapabilityMetadata;
    async fn execute(&self, input: serde_json::Value, ctx: ChannelContext) -> Result<serde_json::Value>;
}
```

同理 `IResource`、`ICommand`、`ISkill` 各有对应 trait。

#### Pipeline (pipeline.rs)

- **State Machine**: `PIPELINE_TRANSITIONS` / `STAGE_TRANSITIONS` → `const` 数组或 `phf` map
- **Events**: 30 种 event 用 `#[serde(tag = "kind")]` tagged enum:
  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize)]
  #[serde(tag = "kind", rename_all = "snake_case")]
  pub enum PipelineEvent {
      PipelineCreated { pipeline_id: String, timestamp: DateTime<Utc>, demand_ref: DemandMetadata },
      StageStarted { pipeline_id: String, timestamp: DateTime<Utc>, stage: StageEnum },
      // ... 28 more
  }
  ```
- **Gates**: `evaluate()` 纯函数，CR grade → `u8` 比较
- **ID**: `format!("pl-{}-{}", date_str, nanoid::nanoid!(8))`
- **Multi-repo**: `topological_sort()` 返回 `Result<Vec<RepoConfig>, CircularDependencyError>`
- **Risk classifier**: `classify_risk()` 纯函数

### 2. poria-infrastructure

**SQLite 读写 + 文件系统 IO + 认证。**

#### Store Layer

将现有 `db.rs`（只读）扩展为完整 CRUD：

```rust
#[async_trait]
pub trait PipelineStore: Send + Sync {
    fn create_pipeline(&self, pipeline: &Pipeline) -> Result<()>;
    fn update_pipeline_status(&self, id: &str, status: PipelineStatus) -> Result<()>;
    fn get_pipeline(&self, id: &str) -> Result<Option<PipelineDetail>>;
    fn list_pipelines(&self) -> Result<Vec<PipelineSummary>>;
    // ...
}

pub struct SqlitePipelineStore { conn: Mutex<Connection> }
```

`EventStore`、`AuditStore`、`PipelineQueue` 同理。

Connection 改为 `SQLITE_OPEN_READ_WRITE | SQLITE_OPEN_CREATE`。

#### Auth

- `get_credentials()`: 读 `~/.poria/auth.json` → `JacpCredentials` struct
- `parse_username_from_cookie()`: cookie 字符串解析
- `CredentialGuard`: 运行期间定期检查 cookie 有效性

#### Config

复用现有 `config.rs`，已经是读写。

#### Logger

用 `tracing` crate 替代自定义 logger：

```rust
pub fn init_logger() {
    tracing_subscriber::fmt().json().init();
}
```

#### Metrics

```rust
pub struct InMemoryMetricsCollector {
    entries: Mutex<Vec<MetricEntry>>,
}
```

### 3. poria-commands

Pipeline 命令层，orchestrate core + infrastructure：

```rust
pub struct PipelineCommands<S: PipelineStore, E: EventStore> {
    store: Arc<S>,
    events: Arc<E>,
}

impl<S: PipelineStore, E: EventStore> PipelineCommands<S, E> {
    pub async fn submit(&self, link: &str, operator: &str) -> Result<String>;
    pub async fn cancel(&self, id: &str, operator: &str) -> Result<()>;
    pub async fn get_status(&self, id: &str) -> Result<PipelineDetail>;
}
```

### 4. poria-resources

- **TerminalResource**: `tokio::process::Command` wrapper，支持 timeout、输出捕获
- **WorktreeResource**: `git worktree add/remove` 封装
- **ClaudeAgentPool**: 管理 Claude CLI 子进程池（`tokio::process`），限制并发
- **OutputGuard**: 验证 agent 输出合规性（正则 + 长度检查）
- **SessionTracker**: 跟踪活跃 agent session

### 5. poria-skills

每个 Skill 实现 `ISkill` trait：

```rust
pub struct InitSkill;

#[async_trait]
impl Skill for InitSkill {
    fn metadata(&self) -> &CapabilityMetadata { ... }
    async fn execute(&self, input: SkillInput, ctx: SkillContext) -> Result<SkillOutput> { ... }
}
```

`STAGE_SKILL_MAP`: `StageEnum → Box<dyn Skill>` 映射。

`HumanLoopCoordinator`: 使用 `tokio::sync::mpsc` channel 等待人类响应。

### 6. poria-channels

每个渠道实现 `Channel` trait：

```rust
pub mod xingyun;  // demand URL 解析、PRD 获取、git 操作
pub mod coding;   // MR 操作、EasyCI 对接
pub mod jme;      // JoyClaw bridge、reply 解析
pub mod joyspace; // 文档导出
pub mod defect;   // 缺陷上报
```

HTTP 调用使用 `reqwest`。

### 7. Tauri Backend Rewrite

`AppState` 升级：

```rust
pub struct AppState {
    pub store: Arc<SqlitePipelineStore>,
    pub events: Arc<SqliteEventStore>,
    pub commands: Arc<PipelineCommands<...>>,
    pub skills: Arc<SkillRegistry>,
    pub channels: Arc<ChannelRegistry>,
}
```

新增 Tauri commands：

| Command              | 用途                             |
| -------------------- | -------------------------------- |
| `list_skills`        | 返回所有已注册 Skill 元数据      |
| `get_skill`          | 返回单个 Skill 详情              |
| `list_channels`      | 返回所有 Channel 及连接状态      |
| `get_channel_status` | 检查单个 Channel 可用性          |
| `submit_pipeline`    | **直接执行**（不再转发 sidecar） |
| `cancel_pipeline`    | 直接取消                         |
| `human_loop_respond` | 通过 mpsc channel 发送响应       |

Pipeline 执行模型：`submit_pipeline` 在 `tokio::spawn` 中启动异步任务，通过 Tauri emit 推送进度事件。

### 8. Frontend Changes

新增路由/页面：

- `/skills` — Skill 管理页（列表 + 配置面板）
- `/channels` — Channel 管理页（状态卡片 + 操作按钮）

导航：Shell 组件新增侧边栏导航项。

新增 Tauri invoke 调用：`listSkills()`, `getSkill(id)`, `listChannels()`, `getChannelStatus(id)`。

移除：sidecar 状态指示器、`sidecar:status` event listener。

## Error Handling

统一使用 `thiserror` 定义每个 crate 的错误类型：

```rust
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("Invalid transition: {entity} {from} → {to}")]
    InvalidTransition { entity: String, from: String, to: String },
    #[error("Circular dependency: {0:?}")]
    CircularDependency(Vec<String>),
}
```

Tauri command 统一返回 `Result<T, String>`，错误转为用户可读消息。

## Data Compatibility

- SQLite schema DDL 保持不变（已在 `db.rs` 中定义）
- JSON 序列化格式保持 snake_case（与前端 TypeScript 类型匹配）
- `~/.poria/auth.json` 和 `~/.poria/config.json` 格式不变

## Testing Strategy

- 每个 crate 有独立的 `#[cfg(test)]` 模块
- poria-core: 状态机 transition、gate 评估、拓扑排序、ID 格式测试
- poria-infrastructure: SQLite 内存数据库集成测试
- poria-channels: HTTP mock 测试（`mockito` crate）
- 前端: 手动验证 Tauri dev 模式下所有页面功能
