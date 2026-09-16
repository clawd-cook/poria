# Pipeline Orchestrator — Design

> 本 child task 的设计细节均来自父任务 design.md §4, §5, §6, §8, §10, §14, §15, §16。本文件仅做交付范围聚焦和实现决策记录。

## 交付范围

### 1. PipelineExecutor (`packages/commands/pipeline/executor.ts`)

父 design §4.1 的完整实现。核心职责：
- 从 store 加载 Pipeline，驱动 stages 顺序执行
- STAGE_SKILL_MAP 映射 7 个 stage → skill
- 每个 Stage 前校验 CredentialGuard
- 多仓库 dev/cr/deploy 委托 MultiRepoOrchestrator
- cr 出口门禁：GateEngine.evaluate + regress 回退
- deploy 出口门禁：blocking/warn 分类
- deploy 完成 → WAITING_MERGE 释放 Worker
- catch 块统一走 handleStageError
- 所有写入通过 store.saveStageTx

### 2. PipelineWorker (`packages/commands/pipeline/worker.ts`)

父 design §4.3 + §14 F1。核心职责：
- 文件锁单实例互斥（workspace/db/worker.lock）
- 启动时 PipelineRecovery.recoverAll()
- 两个并行循环：consumeQueue + pollMergeRequests
- pollMergeRequests: 每 60s 查 waiting_merge Pipeline 的 MR 状态

### 3. PipelineRollback (`packages/commands/pipeline/rollback.ts`)

父 design §6.3 + §15 F-10。核心职责：
- 未合并：按 Stage 逆序执行 rollback 指令
- 已合并：per-repo 创建 revert commit + revert MR
- 幂等保护：close_mr/delete_branch/remove_worktree 各自安全检查
- 单步失败不阻断整体回滚

### 4. Skills 骨架 (`packages/skills/`)

7 个 pipeline stage skill + human-loop 协调器。每个 skill:
- 实现 ISkill 接口
- 支持 fixture 模式 (PORIA_*_FIXTURE=1)
- P1 实现真实逻辑骨架 + fixture fallback

| Skill | 包路径 | 关键依赖 |
|-------|--------|----------|
| init | skills/init/ | xingyun, joyspace |
| review-prd | skills/review-prd/ | claude agent |
| gen-trd | skills/gen-trd/ | claude agent |
| workspace | skills/workspace/ | worktree, xingyun |
| gen-code | skills/gen-code/ | claude agent, output-guard |
| code-review | skills/code-review/ | claude agent |
| deploy | skills/deploy/ | coding, terminal |
| human-loop | skills/human-loop/ | jme |

### 5. ExceptionClassifier + handleStageError

父 design §16 F-09。IssueClass → IssuePolicy 路由 + 重试/通知/阻断逻辑。

## 不做的事

- multi-repo.ts 纯逻辑（拓扑排序）已在 core 实现，只需 I/O 薄层
- SQLite store / queue / recovery 已在 infrastructure 实现
- OutputGuard / AgentPool / Terminal / Worktree 已在 resources 实现
- Channel 层已在 channels 实现

## 关键接口依赖（来自已完成的包）

```typescript
// core
import { PipelineStateMachine, GateEngine, createPipelineId } from "@poria/core";
import type { Pipeline, Stage, StageEnum, GateRule, PipelineEvent, ISkill } from "@poria/core";

// infrastructure
import { SqlitePipelineStore, PipelineQueue, PipelineRecovery, EventStore } from "@poria/infrastructure";
import { CredentialGuard } from "@poria/infrastructure";
import { loadConfig } from "@poria/infrastructure";

// channels
import { XingyunChannel } from "@poria/channels-xingyun";
import { JoySpaceChannel } from "@poria/channels-joyspace";
import { CodingChannel } from "@poria/channels-coding";
import { JmeChannel } from "@poria/channels-jme";

// resources
import { ClaudeAgentPool, OutputGuard } from "@poria/resources";
import { TerminalResource, WorktreeResource } from "@poria/resources";
```
