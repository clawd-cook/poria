# Migrate packages/ to Rust crates and integrate into Tauri backend

## Goal

将 `packages/` 下所有 11 个 TypeScript 包迁移为 Rust crate（放在 `crates/`），完全替换 Node.js sidecar，让 Tauri Rust 后端成为唯一的业务引擎。前端扩展新页面以展示 Skills、Channels 等新暴露的能力。

## Background

当前架构：

- `packages/` 包含 6 个核心包 + 5 个 channel 适配包（TypeScript/Node.js）
- `apps/desktop` 是 Tauri v2 应用，Rust 后端目前只做只读 DB 查询，所有写操作和业务逻辑通过 Node sidecar 完成
- 前端（React 19 + Tailwind）仅展示 Pipeline 列表/详情/事件流

## Scope

### In Scope

1. **Rust Crates 迁移** — 将 packages/ 下的包一对一迁移到 crates/：
   - `poria-core` ← `@poria/core`（类型、合约 trait、状态机、事件、门控、风险分级、拓扑排序、ID 生成）
   - `poria-infrastructure` ← `@poria/infrastructure`（SQLite 读写、Auth、Config、Logger、Metrics、Plugin Loader）
   - `poria-commands` ← `@poria/commands`（Pipeline 命令层）
   - `poria-resources` ← `@poria/resources`（Terminal 执行、Worktree 管理、Claude Agent Pool、Output Guard、Session Tracker）
   - `poria-skills` ← `@poria/skills`（7 个 Skill 实现 + HumanLoop 协调）
   - `poria-channels` ← `packages/channels/*`（xingyun、coding、jme、joyspace、defect 渠道适配）

2. **Tauri 后端升级** — `apps/desktop/src-tauri` 从只读变为全功能：
   - DB 从只读改为读写
   - 移除所有 sidecar event 转发（`sidecar:submit`、`sidecar:cancel`、`sidecar:human-respond`）
   - 直接调用 Rust crate 执行 pipeline submit、cancel、human-loop 等操作
   - 新增 Tauri commands 暴露 Skills 和 Channels 能力

3. **前端新增页面**：
   - **Skill 管理页**：展示所有可用 Skills 及其配置、状态
   - **Channel 管理页**：展示各渠道（xingyun/jme/coding/joyspace/defect）的连接状态和操作入口
   - 保留并适配现有 Pipeline 展示（PipelineSidebar、PipelineDetail、EventStream 等）

4. **去除 Node sidecar 依赖**：
   - 移除 `sidecar:status` 事件和前端 sidecar 状态指示器
   - 前端直接通过 Tauri invoke 调用 Rust 后端

### Out of Scope

- Node.js CLI 入口（如果存在的话）保持不变，本任务只关注 desktop app 内的 Rust 化
- packages/ 目录本身暂不删除（等迁移验证完毕后由后续任务清理）
- 渠道 API 的在线集成测试（渠道 crate 只迁移本地逻辑和类型，实际 API 调用由 mock 覆盖）

## Constraints

- **Runtime**: Rust edition 2021, Tauri v2
- **DB**: rusqlite (bundled)，保持现有 SQLite schema 兼容
- **Cargo workspace**: 根目录新增 `Cargo.toml` workspace，成员包含 `crates/*` 和 `apps/desktop/src-tauri`
- **前端**: React 19 + Tailwind CSS 4，保持现有组件风格
- **Node.js 依赖替换映射**：
  - `nanoid` → `nanoid` crate 或内联实现
  - `minimatch` → `glob` crate
  - `better-sqlite3` → `rusqlite`（已有）
  - `child_process` → `tokio::process`
  - `fs/path` → `std::fs` / `std::path`

## Requirements

### R1: poria-core crate

- 所有 TypeScript 类型/接口转为 Rust struct + enum，derive `Serialize`/`Deserialize`/`Clone`/`Debug`
- Pipeline/Stage 状态机（transition tables）保持一致
- 30 种 PipelineEvent 类型通过 `#[serde(tag = "kind")]` enum 实现
- Gate 评估逻辑（CR score 比较、阈值判断）保持等价
- 拓扑排序、风险分级纯函数直接迁移
- Pipeline ID 生成保持 `pl-YYYYMMDD-<8chars>` 格式

### R2: poria-infrastructure crate

- SQLite 改为读写模式，schema 与现有 DDL 保持一致
- PipelineStore trait + rusqlite 实现（CRUD 全部）
- EventStore、AuditStore、PipelineQueue、DatabaseBackup
- Auth：从 `~/.poria/auth.json` 读取凭据、cookie 解析、CredentialGuard
- Config：从 `~/.poria/config.json` 读写
- Logger：结构化日志 trait（可用 `tracing` crate）
- Metrics：InMemory collector + 导出

### R3: poria-commands crate

- Pipeline 命令层（submit、cancel、status 查询等）
- 依赖 poria-core 和 poria-infrastructure

### R4: poria-resources crate

- Terminal 执行（`tokio::process::Command` wrapper）
- Worktree 管理（git worktree create/remove）
- Claude Agent Pool（进程池管理、output guard）
- Session Tracker

### R5: poria-skills crate

- 7 个 Skill trait 实现：Init, ReviewPrd, GenTrd, Workspace, GenCode, CodeReview, Deploy
- HumanLoop 协调器
- Stage → Skill 映射表

### R6: poria-channels crate

- 5 个 channel 子模块：xingyun, coding, jme, joyspace, defect
- 各渠道的 URL 解析、请求构造、响应解析
- Channel trait 统一接口

### R7: Tauri 后端重构

- DB 切换到读写模式
- 新增 Tauri commands：skill 列表/详情、channel 列表/状态
- Pipeline commands 直接调用 Rust 实现而非 sidecar 转发
- 事件推送从 sidecar 驱动改为 Rust 内部 channel + Tauri emit

### R8: 前端新增

- Skill 管理页：列表、配置查看
- Channel 管理页：连接状态、操作入口
- 移除 sidecar 状态指示器
- 适配新增的 Tauri invoke commands

## Acceptance Criteria

- [ ] `cargo build --workspace` 成功编译所有 crate
- [ ] `cargo test --workspace` 所有单元测试通过
- [ ] 核心类型 poria-core 与 TypeScript 定义语义等价（状态机 transition 一致、事件类型一致）
- [ ] Desktop app 可以通过 Rust 后端直接 submit pipeline（不依赖 Node sidecar）
- [ ] Desktop app 前端展示 Skill 管理页和 Channel 管理页
- [ ] 现有 Pipeline 列表/详情/事件流功能保持正常
- [ ] `pnpm tauri dev` 能正常启动并运行完整功能
- [ ] SQLite DB schema 保持向后兼容（现有数据可读取）

## Child Task Structure

此为 parent task，按 crate 拆分 child tasks：

| Child                         | Slug             | 依赖                                                                             |
| ----------------------------- | ---------------- | -------------------------------------------------------------------------------- |
| 1. poria-core crate           | `rust-core`      | 无                                                                               |
| 2. poria-infrastructure crate | `rust-infra`     | rust-core                                                                        |
| 3. poria-commands crate       | `rust-commands`  | rust-core, rust-infra                                                            |
| 4. poria-resources crate      | `rust-resources` | rust-core                                                                        |
| 5. poria-skills crate         | `rust-skills`    | rust-core, rust-resources                                                        |
| 6. poria-channels crate       | `rust-channels`  | rust-core                                                                        |
| 7. Tauri backend rewrite      | `rust-tauri`     | rust-core, rust-infra, rust-commands, rust-resources, rust-skills, rust-channels |
| 8. Frontend pages             | `rust-frontend`  | rust-tauri                                                                       |

依赖关系写在各 child 的 prd.md 中，不是阻塞关系——而是编码顺序建议。
