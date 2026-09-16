# Core Foundation — 实施计划

## 前置条件

- 无外部依赖（Wave 1 首个任务）
- `packages/core/` 目录已存在但为空
- 项目根已配 `pnpm-workspace.yaml`（packages: `packages/*`）、`"type": "module"`
- 无根 tsconfig.json，需创建

## 实施步骤

### Step 0: 包脚手架

- [ ] 创建根 `tsconfig.json`（Node 24, ESM, strict, 共享基线）
- [ ] 创建 `packages/core/package.json`（name: `@poria/core`, type: module, exports, scripts: typecheck + test）
- [ ] 创建 `packages/core/tsconfig.json`（extends 根配置, include src/）
- [ ] 安装唯一外部依赖 `nanoid`，dev 依赖 `typescript`, `vitest`
- [ ] 创建 `packages/core/src/index.ts`（空桶文件，后续逐步加 re-export）

验证: `pnpm --filter @poria/core run typecheck` 通过。

### Step 1: 共享类型 (`core/types/`)

- [ ] `src/types/pipeline.ts` — Pipeline, Stage, StageEnum, PipelineStatus, StageStatus, StageOutput, SkillInput, SkillOutput
- [ ] `src/types/demand.ts` — DemandMetadata, UserVO, CardAttachment
- [ ] `src/types/repo.ts` — RepoConfig
- [ ] `src/types/issue.ts` — IssueClass enum, IssuePolicy, ISSUE_POLICIES 默认策略表
- [ ] `src/types/gate.ts` — GateRule, GateResult, GateEvaluation, GatePhase
- [ ] `src/types/rollback.ts` — RollbackInstruction, RollbackCommand
- [ ] `src/types/agent.ts` — AgentTaskInput, AgentTaskResult
- [ ] `src/types/timeout.ts` — TIMEOUT 常量
- [ ] `src/types/index.ts` — 统一 re-export

验证: typecheck 通过。

### Step 2: 能力契约 (`core/contracts/`)

- [ ] `src/contracts/channel.ts` — IChannel
- [ ] `src/contracts/resource.ts` — IResource
- [ ] `src/contracts/command.ts` — ICommand
- [ ] `src/contracts/skill.ts` — ISkill
- [ ] `src/contracts/index.ts` — 统一 re-export

验证: typecheck 通过。

### Step 3: Pipeline 状态机 (`core/pipeline/state-machine.ts`)

- [ ] PipelineStatus enum + PIPELINE_TRANSITIONS 映射（11 条转换规则）
- [ ] StageStatus enum + STAGE_TRANSITIONS 映射
- [ ] `canTransition(from, to)` + `transition(entity, to)` 函数
- [ ] 非法转换抛 `InvalidTransitionError`
- [ ] `src/pipeline/__tests__/state-machine.test.ts` — 覆盖全部合法 + 非法转换

验证: test 通过，覆盖 11 条 Pipeline 转换 + Stage 转换。

### Step 4: 领域事件 (`core/pipeline/events.ts`)

- [ ] `PipelineEventBase` 共享字段（seq, pipelineId, timestamp, kind）
- [ ] 27 种事件的 discriminated union 类型
- [ ] 每种事件的工厂函数（如 `PipelineCreatedEvent(...)`, `StageStartedEvent(...)` 等）
- [ ] `src/pipeline/__tests__/events.test.ts` — 工厂函数创建各事件实例

验证: typecheck 通过，工厂函数测试通过。

### Step 5: 门禁引擎 (`core/pipeline/gates.ts`)

- [ ] `CR_GRADE_ORDER` 映射（A+=10 到 F=0）
- [ ] `crScoreMeetsThreshold(actual, threshold)` 数值比较
- [ ] `GateEngine.evaluate(result, rules, phase)` — phase 过滤 + 逐条评估
- [ ] `DEFAULT_GATES` 预置 6 条规则
- [ ] `src/pipeline/__tests__/gates.test.ts`:
  - CR 评分比较: A > B+ > B（数值非字典序）
  - phase 过滤: stage_exit 只执行 cr_score
  - regress 策略返回 regressTo
  - block/warn 分类
  - 全通过 / 部分失败 / 全失败

验证: 全部门禁测试通过。

### Step 6: Pipeline ID + 拓扑排序 + 风险分级

- [ ] `src/pipeline/id.ts` — `createPipelineId()` 格式 `pl-{YYYYMMDD}-{nanoid(8)}`
- [ ] `src/pipeline/multi-repo.ts` — `topologicalSort(repos)` Kahn's algorithm + `CircularDependencyError`
- [ ] `src/pipeline/risk-classifier.ts` — `classifyRisk(changedFiles, diffLines)` → low/medium/high/critical
- [ ] `src/pipeline/__tests__/id.test.ts` — 格式校验 + 唯一性
- [ ] `src/pipeline/__tests__/multi-repo.test.ts` — 无依赖/线性/菱形/循环
- [ ] `src/pipeline/__tests__/risk-classifier.test.ts` — 各等级边界

验证: 全部测试通过。

### Step 7: 入口文件 + 最终验证

- [ ] `src/index.ts` — 从 types/, contracts/, pipeline/ 统一 re-export
- [ ] `pnpm --filter @poria/core run typecheck` 通过
- [ ] `pnpm --filter @poria/core run test` 全部通过
- [ ] 确认无 `any` 类型、无 I/O 依赖

## 验证命令

```bash
pnpm --filter @poria/core run typecheck
pnpm --filter @poria/core run test
```
