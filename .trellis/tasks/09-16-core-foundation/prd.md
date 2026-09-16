# Core Foundation — PRD v1.0

> 父任务: poria-platform-arch。本任务为 Wave 1，无外部依赖。

## Goal

提供 Poria 平台化架构的**核心类型系统与纯逻辑引擎**，包括共享类型定义、四种能力契约接口、Pipeline 状态机、门禁引擎、领域事件模型。所有上层包（infrastructure、channels、resources、commands、skills）均依赖本包。

---

## 核心需求

### R1: 共享类型定义 (`core/types/`)

导出以下类型，字段定义参照父任务 Design：

- **Pipeline 相关**: `Pipeline`, `Stage`, `StageEnum`（init | review_prd | design | workspace | dev | cr | deploy）, `StageOutput`, `SkillInput`, `SkillOutput`
- **需求元数据**: `DemandMetadata`（demandId, demandCode, name, status, demandProjectId, processor, proposer, receiver, prdUrl, attachments, rawLink）, `UserVO`（erp, name, orgId, orgName）, `CardAttachment`（tagName, name, url）
- **仓库配置**: `RepoConfig`（name, gitUrl, branch, baseBranch, gitlabProjectPath, dependsOn, buildCmd）
- **异常分类**: `IssueClass` 枚举（14 种）, `IssuePolicy` 接口（autoRetry, notifyRoles, escalateAt, retryDelay, note）, `ISSUE_POLICIES` 默认策略表
- **门禁相关**: `GateRule`, `GateResult`, `GateEvaluation`
- **回滚相关**: `RollbackInstruction`, `RollbackCommand`
- **Agent 相关**: `AgentTaskInput`, `AgentTaskResult`
- **超时档位**: `TIMEOUT` 常量（GIT_SHORT=30s, GIT_MEDIUM=120s, GIT_LONG=300s, BUILD=600s, AGENT=1800s）

### R2: 能力契约接口 (`core/contracts/`)

四种能力契约，每个独立文件：

- `IChannel` — 外部平台集成（xingyun, joyspace, coding, jme, defect）
- `IResource` — 本机资源（claude, terminal, worktree）
- `ICommand` — CLI 命令（execute + metadata）
- `ISkill` — AI 能力（execute + metadata）

每个接口定义 `execute(input, context)` 签名 + `metadata`（id, name, description, version）。

### R3: Pipeline 状态机 (`core/pipeline/state-machine.ts`)

**Pipeline 状态枚举**:

```
created | running | waiting_merge | blocked | completed | failed | cancelled
```

**Stage 状态枚举**:

```
pending | running | completed | failed | blocked | skipped
```

**Pipeline 转换规则**（完整覆盖 Design §14 F1 状态图）:

| 源状态 | 目标状态 | 触发条件 |
|--------|---------|---------|
| created | running | submit |
| created | cancelled | cancel before start |
| running | waiting_merge | deploy MR created |
| running | blocked | stage needs human |
| running | failed | retries exhausted |
| running | cancelled | manual cancel |
| blocked | running | human reply (resume) |
| blocked | cancelled | manual cancel |
| waiting_merge | completed | all MRs merged |
| waiting_merge | failed | MR closed / 24h timeout |
| waiting_merge | cancelled | manual cancel |

**实现要求**:
- 提供 `canTransition(from, to): boolean` 判定
- 提供 `transition(pipeline, to): Pipeline` 执行转换（非法转换抛异常）
- 提供 `Stage.canTransition(from, to)` 同理

### R4: 领域事件定义 (`core/pipeline/events.ts`)

完整事件类型联合，覆盖 Design §14 F9 清单的全部 27 种事件：

- Pipeline 生命周期（6 种）: created, started, completed, failed, cancelled, waiting_merge
- Stage 生命周期（6 种）: started, completed, failed, blocked, resumed, regressed
- Agent 执行（4 种）: dispatched, progress, completed, failed
- 门禁（2 种）: evaluated, regress_triggered
- 人工回路（3 种）: assist_requested, assist_received, assist_escalated
- 凭证（2 种）: refreshed, expired
- Git 操作/审计（7 种）: commit, push, mr_created, mr_merged, worktree_created, worktree_cleaned, rollback_executed

每种事件共享 base 字段: `{ seq, pipelineId, timestamp, kind }`。

提供工厂函数创建各事件实例。

### R5: 门禁引擎 (`core/pipeline/gates.ts`)

- `GateEngine.evaluate(result, rules, phase)` — 按 `gatePhase` 过滤规则后逐条评估
- `phase` 参数: `"stage_exit"` | `"deploy"`
- `onFail` 三种策略: `"block"` | `"warn"` | `"regress"`
- `regress` 策略支持 `regressTo` 字段指定回退目标阶段
- **CR 评分数值比较**: 定义 `CR_GRADE_ORDER` 映射（A+=10 到 F=0），使用数值比较而非字符串比较
- 返回 `GateEvaluation`: `{ allPass, details, blockingFailures, warnFailures }`
- `DEFAULT_GATES` 预置 6 条规则（ci_build, test_coverage, security_scan, diff_size, merge_conflict, cr_score）

### R6: Pipeline ID 生成 (`core/pipeline/id.ts`)

- 格式: `pl-{YYYYMMDD}-{nanoid(8)}`
- 示例: `pl-20260916-a1b2c3d4`
- 可排序、含日期前缀、无冲突
- 依赖: `nanoid`

### R7: 拓扑排序 (`core/pipeline/multi-repo.ts`)

- `topologicalSort(repos: RepoConfig[]): RepoConfig[]` — Kahn's algorithm
- 循环依赖检测: 发现环 → 抛 `CircularDependencyError`（含环路信息）
- 纯逻辑，不含任何 I/O 操作

### R8: 变更风险分级 (`core/pipeline/risk-classifier.ts`)

- 根据 diff 文件类型、变更量、是否涉及配置文件等分级
- 风险等级: `low` | `medium` | `high` | `critical`
- 用于 OutputGuard 和门禁决策

---

## 约束

### C1: 纯逻辑包

- 零 I/O 依赖（无 fs、无 network、无 SQLite）
- 唯一外部依赖: `nanoid`（ID 生成）
- 所有导出均为类型定义、纯函数、常量

### C2: 包结构

- 包路径: `packages/core/`
- 包名: `@poria/core`（monorepo 内部包，不发 npm）
- 入口: `src/index.ts` 统一导出

### C3: TypeScript 严格模式

- `strict: true`
- 无 `any` 类型（允许 `unknown`）
- 所有公开 API 有完整类型签名

---

## 非目标

- 不含 I/O 操作（数据库、网络、文件系统）
- 不含 CLI 逻辑
- 不含 Agent 调度实现（仅定义类型）
- 不含 Channel/Resource 实现（仅定义契约接口）

---

## Acceptance Criteria

- [ ] 所有类型从 `@poria/core` 入口统一导出
- [ ] Pipeline 状态机转换规则完整覆盖 Design §14 F1 状态图（11 条转换），非法转换抛异常
- [ ] Stage 状态机转换规则完整
- [ ] GateEngine 门禁引擎 unit test 通过:
  - CR 评分数值比较正确（A > B+ > B > C）
  - phase 过滤正确（stage_exit 只执行 cr_score，deploy 只执行其余）
  - regress 策略返回 regressTo 字段
  - block/warn 分类正确
- [ ] Pipeline ID 格式为 `pl-{YYYYMMDD}-{nanoid(8)}`
- [ ] 拓扑排序正确处理: 无依赖、线性依赖、菱形依赖、循环依赖（抛 CircularDependencyError）
- [ ] 事件类型联合覆盖全部 27 种事件，工厂函数可正确创建各事件
- [ ] `pnpm run typecheck` 通过（strict 模式，无 any）
- [ ] `pnpm run test` 通过（状态机、门禁、拓扑排序、ID 生成均有 unit test）
