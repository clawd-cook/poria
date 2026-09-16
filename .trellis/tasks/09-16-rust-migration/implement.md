# Implementation Plan — Rust Migration

## Phase 0: Workspace Setup

- [ ] 0.1 创建根 `Cargo.toml` workspace 配置（members: `crates/*`, `apps/desktop/src-tauri`）
- [ ] 0.2 `cargo init --lib crates/poria-core`
- [ ] 0.3 `cargo init --lib crates/poria-infrastructure`
- [ ] 0.4 `cargo init --lib crates/poria-commands`
- [ ] 0.5 `cargo init --lib crates/poria-resources`
- [ ] 0.6 `cargo init --lib crates/poria-skills`
- [ ] 0.7 `cargo init --lib crates/poria-channels`
- [ ] 0.8 更新 `apps/desktop/src-tauri/Cargo.toml` 使用 workspace dependencies
- [ ] 0.9 验证: `cargo build --workspace` 通过（空 crate）

## Phase 1: poria-core (child: rust-core)

- [ ] 1.1 迁移 `types/` — Pipeline, Stage, StageEnum, PipelineStatus, StageStatus 等所有 struct/enum
- [ ] 1.2 迁移 `types/demand.ts` — DemandMetadata, UserVO, CardAttachment
- [ ] 1.3 迁移 `types/gate.ts` — GateRule, GateResult, GateEvaluation, GatePhase, GateOnFail
- [ ] 1.4 迁移 `types/issue.ts` — IssueClass enum + ISSUE_POLICIES static map
- [ ] 1.5 迁移 `types/repo.ts`, `rollback.ts`, `agent.ts`, `timeout.ts`
- [ ] 1.6 迁移 `contracts/` — Channel, Resource, Command, Skill trait 定义
- [ ] 1.7 迁移 `pipeline/state-machine.ts` — transition tables + can/transition functions
- [ ] 1.8 迁移 `pipeline/events.ts` — PipelineEvent tagged enum + 30 个构造函数
- [ ] 1.9 迁移 `pipeline/gates.ts` — CR grade 比较 + evaluate 函数 + DEFAULT_GATES
- [ ] 1.10 迁移 `pipeline/id.ts` — createPipelineId
- [ ] 1.11 迁移 `pipeline/risk-classifier.ts` — classifyRisk
- [ ] 1.12 迁移 `pipeline/multi-repo.ts` — topologicalSort
- [ ] 1.13 编写测试: 状态机 transition, gate 评估, 拓扑排序, risk 分级
- [ ] 1.14 验证: `cargo test -p poria-core` 全部通过

## Phase 2: poria-infrastructure (child: rust-infra)

- [ ] 2.1 升级 `db.rs` — Connection 改为 RW 模式
- [ ] 2.2 实现 PipelineStore trait — create, update_status, get, list, delete
- [ ] 2.3 实现 EventStore — append, query by pipeline_id
- [ ] 2.4 实现 AuditStore — append, query
- [ ] 2.5 实现 PipelineQueue — enqueue, dequeue, peek
- [ ] 2.6 实现 DatabaseBackup — backup to file
- [ ] 2.7 实现 Auth 模块 — get_credentials, save_credentials, parse_username_from_cookie
- [ ] 2.8 实现 CredentialGuard — 定时检查 cookie 有效性
- [ ] 2.9 Config 已有, 确认 workspace dependency 正确
- [ ] 2.10 实现 Logger — tracing 初始化
- [ ] 2.11 实现 Metrics — InMemoryMetricsCollector
- [ ] 2.12 实现 PluginLoader — 动态加载 trait object
- [ ] 2.13 编写测试: SQLite 内存 DB CRUD, auth 解析
- [ ] 2.14 验证: `cargo test -p poria-infrastructure` 全部通过

## Phase 3: poria-commands (child: rust-commands)

- [ ] 3.1 实现 PipelineCommands — submit, cancel, get_status, list
- [ ] 3.2 编写测试: mock store 的命令执行
- [ ] 3.3 验证: `cargo test -p poria-commands`

## Phase 4: poria-resources (child: rust-resources)

- [ ] 4.1 实现 TerminalResource — tokio::process::Command wrapper
- [ ] 4.2 实现 WorktreeResource — git worktree create/remove
- [ ] 4.3 实现 ClaudeAgentPool — 进程池管理
- [ ] 4.4 实现 OutputGuard — 输出验证
- [ ] 4.5 实现 SessionTracker
- [ ] 4.6 编写测试
- [ ] 4.7 验证: `cargo test -p poria-resources`

## Phase 5: poria-skills (child: rust-skills)

- [ ] 5.1 实现 Skill trait 注册机制 + STAGE_SKILL_MAP
- [ ] 5.2 实现 InitSkill
- [ ] 5.3 实现 ReviewPrdSkill
- [ ] 5.4 实现 GenTrdSkill
- [ ] 5.5 实现 WorkspaceSkill
- [ ] 5.6 实现 GenCodeSkill
- [ ] 5.7 实现 CodeReviewSkill
- [ ] 5.8 实现 DeploySkill
- [ ] 5.9 实现 HumanLoopCoordinator（tokio mpsc channel）
- [ ] 5.10 编写测试
- [ ] 5.11 验证: `cargo test -p poria-skills`

## Phase 6: poria-channels (child: rust-channels)

- [ ] 6.1 实现 xingyun 模块 — demand URL 解析, PRD 获取, demand guard, git 操作
- [ ] 6.2 实现 coding 模块 — MR 操作, EasyCI, git URL 解析
- [ ] 6.3 实现 jme 模块 — JoyClaw bridge, reply 解析
- [ ] 6.4 实现 joyspace 模块 — 文档导出
- [ ] 6.5 实现 defect 模块 — 缺陷上报
- [ ] 6.6 编写测试: URL 解析 + HTTP mock
- [ ] 6.7 验证: `cargo test -p poria-channels`

## Phase 7: Tauri Backend Rewrite (child: rust-tauri)

- [ ] 7.1 更新 AppState — 添加 store(RW), events, commands, skills, channels
- [ ] 7.2 重写 submit_pipeline — 直接调用 PipelineCommands::submit, tokio::spawn 异步执行
- [ ] 7.3 重写 cancel_pipeline — 直接调用 PipelineCommands::cancel
- [ ] 7.4 重写 human_loop_respond — 通过 mpsc channel 发送
- [ ] 7.5 新增 list_skills command
- [ ] 7.6 新增 get_skill command
- [ ] 7.7 新增 list_channels command
- [ ] 7.8 新增 get_channel_status command
- [ ] 7.9 移除所有 sidecar event emit
- [ ] 7.10 Pipeline 执行中通过 app_handle.emit() 推送进度事件
- [ ] 7.11 验证: `cargo build -p poria-desktop`

## Phase 8: Frontend (child: rust-frontend)

- [ ] 8.1 新增 Tauri invoke wrappers: listSkills, getSkill, listChannels, getChannelStatus
- [ ] 8.2 新增 types: SkillInfo, ChannelInfo, ChannelStatus
- [ ] 8.3 新增 SkillsPage 组件 — 技能卡片列表 + 配置查看
- [ ] 8.4 新增 ChannelsPage 组件 — 渠道状态卡片 + 操作入口
- [ ] 8.5 Shell 组件新增侧边栏导航（Pipeline / Skills / Channels）
- [ ] 8.6 移除 sidecar 状态指示器和 sidecar:status event listener
- [ ] 8.7 更新 StoreProvider — 移除 sidecar 相关 state 和 action
- [ ] 8.8 验证: `pnpm tauri dev` 启动, 手动测试所有页面

## Phase 9: Integration & Cleanup

- [ ] 9.1 `cargo build --workspace` 全量编译
- [ ] 9.2 `cargo test --workspace` 全量测试
- [ ] 9.3 `pnpm tauri dev` 端到端手动测试
- [ ] 9.4 验证 SQLite 数据兼容性（用已有 DB 文件测试）

## Validation Commands

```bash
# 每个 phase 结束后运行
cargo build --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings

# Phase 8+
pnpm tauri dev
```

## Rollback

每个 phase 是独立 crate，可以单独回退。最坏情况：删除 crates/ 和根 Cargo.toml，恢复 src-tauri/Cargo.toml 为独立配置。
