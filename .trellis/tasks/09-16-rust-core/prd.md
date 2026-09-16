# poria-core: Rust 类型/合约/状态机/事件/门控

## Goal

创建 `crates/poria-core` crate，将 `packages/core/src/` 下所有 TypeScript 类型、合约接口、Pipeline 状态机、事件系统、门控评估、风险分级和多仓拓扑排序迁移为等价的 Rust 实现。此 crate 是所有其他 poria-* crate 的基础依赖。

## Parent Task

09-16-rust-migration

## Dependencies

无外部 child 依赖。需要同步创建根 `Cargo.toml` workspace 和 crate 骨架（Phase 0）。

## Requirements

### R1: Types

- 所有 TypeScript interface/type 转为 Rust struct/enum
- derive: `Debug`, `Clone`, `Serialize`, `Deserialize`
- 字符串 union → `#[serde(rename_all = "snake_case")]` enum
- `IssueClass` enum + `ISSUE_POLICIES` static HashMap
- `TIMEOUT` 常量
- `STAGE_ORDER` const array

### R2: Contracts

- `Channel`, `Resource`, `Command`, `Skill` 四个 `#[async_trait]` trait
- `CapabilityMetadata` struct
- Context structs: `ChannelContext`, `ResourceContext`, `CommandContext`, `SkillContext`

### R3: Pipeline State Machine

- `PIPELINE_TRANSITIONS` / `STAGE_TRANSITIONS` 映射
- `can_pipeline_transition()`, `can_stage_transition()` 函数
- `transition_pipeline()`, `transition_stage()` — 返回 `Result` 而非 panic
- `InvalidTransitionError`

### R4: Pipeline Events

- `PipelineEvent` tagged union enum，30 种事件
- 每种事件有对应构造函数
- `#[serde(tag = "kind", rename_all = "snake_case")]`

### R5: Gates

- `DEFAULT_GATES` static array
- `cr_score_meets_threshold()` 纯函数
- `evaluate()` 纯函数
- `StageResult` struct

### R6: Utilities

- `create_pipeline_id()` — 格式 `pl-YYYYMMDD-<8chars>`
- `classify_risk()` — 输入 changed files + diff lines，输出 RiskLevel
- `topological_sort()` — 输入 `Vec<RepoConfig>`，输出有序 `Vec<RepoConfig>` 或 `CircularDependencyError`

## Acceptance Criteria

- [ ] `cargo build -p poria-core` 成功
- [ ] `cargo test -p poria-core` 所有测试通过
- [ ] `cargo clippy -p poria-core -- -D warnings` 无警告
- [ ] Pipeline 状态机 transition 与 TypeScript 版本一致（对照测试）
- [ ] Gate 评估结果与 TypeScript 版本一致
- [ ] 拓扑排序正确处理正常 / 空 / 环形依赖
- [ ] Risk classifier 对相同输入产生相同输出
- [ ] PipelineEvent serde JSON 输出格式与前端类型兼容（snake_case, tag = kind）
