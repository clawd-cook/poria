# Poria 平台化架构 — 实施计划 (Parent)

> 本任务为 parent task，不直接编码。所有实施由 child tasks 独立交付。

## 任务树

```
poria-platform-arch (parent, 本任务)
├── core-foundation       — 核心类型 + 契约 + 状态机 + 门禁 + 事件
├── infra-persistence     — SQLite schema + CRUD + 事件存储 + 队列
├── channel-migration     — 从 submodules 迁移 xingyun/joyspace/coding + 新增 jme
├── resource-layer        — Claude Agent 调度 + OutputGuard + terminal + worktree
├── pipeline-orchestrator — Executor + 多仓编排 + Worker + MR 轮询 + 人工回路
└── cli-commands          — pipeline/auth/workspace CLI 命令
```

## 依赖顺序

```
core-foundation
    ↓
infra-persistence ←─── channel-migration ←─── resource-layer
    ↓                       ↓                       ↓
    └───────────────────────┴───────────────────────┘
                            ↓
                   pipeline-orchestrator
                            ↓
                       cli-commands
```

- `core-foundation` 无外部依赖，最先实施
- `infra-persistence`、`channel-migration`、`resource-layer` 仅依赖 core-foundation，可并行
- `pipeline-orchestrator` 依赖上面全部三个
- `cli-commands` 依赖 pipeline-orchestrator

## Child Task 范围定义

### 1. core-foundation

**包路径**: `packages/core/`

**交付物**:
- `core/types/` — 共享类型定义（Pipeline, Stage, RepoConfig, DemandMetadata, IssueClass, IssuePolicy）
- `core/contracts/` — IChannel, IResource, ICommand, ISkill 四种能力契约接口
- `core/pipeline/state-machine.ts` — Pipeline/Stage 状态流转
- `core/pipeline/events.ts` — 完整事件类型定义（Design §14 F9 清单）
- `core/pipeline/gates.ts` — GateEngine + CR 评分数值比较（Design §5, §14 F2/F3）
- `core/pipeline/risk-classifier.ts` — 变更风险分级
- `core/pipeline/id.ts` — Pipeline ID 生成（Design §14 F8）
- `core/pipeline/multi-repo.ts` — RepoConfig 类型 + 拓扑排序（纯逻辑，不含 I/O）

**验收**: 全部类型可导出，状态机转换覆盖 Design §14 F1 状态图，门禁引擎 unit test 通过。

### 2. infra-persistence

**包路径**: `packages/infrastructure/`

**交付物**:
- `store/schema.ts` — SQLite 建表（Design §7）+ migration
- `store/pipeline-repo.ts` — Pipeline CRUD + saveStageTx 事务写入（Design §14 F10）
- `store/event-store.ts` — 事件追加/查询
- `store/audit-store.ts` — 审计日志写入
- `store/recovery.ts` — 断点续跑恢复（Design §4.2）
- `store/queue.ts` — 串行队列 + 单实例锁（Design §4.3）
- `store/archiver.ts` — 事件归档（Design §7.2）
- `auth/` — SSO cookie 提取（迁移自 poria-auth）
- `auth/credential-guard.ts` — cookie 有效性探测 + 自动刷新（Design A3）
- `config/` — 全局配置（门禁阈值、超时、重试次数）
- `logger/` — 结构化日志

**验收**: schema 可创建，CRUD round-trip 测试通过，saveStageTx 事务原子性测试通过，recovery 可从 RUNNING 状态恢复。

### 3. channel-migration

**包路径**: `packages/channels/`

**交付物**:
- `xingyun/` — 迁移自 `@dj-lib/poria-channel-xingyun`（JACP 客户端 + 链接解析 + 分支绑定）
- `joyspace/` — 迁移自 `@dj-lib/poria-channel-joyspace`（JoySpace → Markdown 导出）
- `coding/` — 迁移自 `@dj-lib/poria-channel-coding`（EasyCI 仓库/分支/MR）+ 新增 getMrStatus、findMr（Design A11）
- `jme/` — 新建，JoyClaw 桥接（Design A9）
- `defect/` — 预留目录（P1 不实现）

**验收**: 迁移后的 channel 在 fixture 模式下（PORIA_*_FIXTURE=1）可正常调用，链接解析 unit test 通过，coding 新增 API 有 fixture 覆盖。

### 4. resource-layer

**包路径**: `packages/resources/`

**交付物**:
- `claude/agent-pool.ts` — Agent SDK startup() + query() 调度（Design A7）
- `claude/output-guard.ts` — 文件范围/diff 量/依赖安全检查（Design §6.1）
- `claude/session-tracker.ts` — session_id 管理
- `terminal/` — 迁移自 `@dj-lib/poria-resource-terminal`（Shell exec）+ 超时档位（Design A4）
- `worktree/` — Git worktree 生命周期管理

**验收**: OutputGuard unit test 覆盖 trdScope 空值/超范围/diff 超限/恶意依赖场景，terminal exec 超时可配置。

### 5. pipeline-orchestrator

**包路径**: `packages/commands/pipeline/` + `packages/skills/`

**交付物**:
- `commands/pipeline/executor.ts` — PipelineExecutor 主循环（Design §4.1 最终版）
- `commands/pipeline/worker.ts` — PipelineWorker 队列消费 + MR 轮询（Design §14 F1）
- `commands/pipeline/rollback.ts` — 回滚机制（Design §6.3 + F-10 幂等保护）
- `core/pipeline/multi-repo.ts` — MultiRepoOrchestrator I/O 层（Design §10）
- `skills/` — 7 个 skill 骨架（init, review-prd, gen-trd, workspace, gen-code, code-review, deploy）+ human-loop 协调器

**验收**: Executor 可驱动 mock skill 跑完完整 7-stage，断点续跑从中间 stage 恢复，门禁回退 dev→cr 可触发，WAITING_MERGE 释放 Worker。

### 6. cli-commands

**包路径**: `packages/commands/`

**交付物**:
- `pipeline/` — submit, status, list, resume, cancel, rollback, replay 命令
- `auth/` — login, logout, status 命令
- `workspace/` — enter, exit, status 命令
- `project/` — create, list, status 命令（骨架）

**验收**: `poria pipeline submit <link>` 可创建 Pipeline 并入队，`poria pipeline status <id>` 可查询状态。

## 跨 child 验收标准

- [ ] 全部 child task 独立 typecheck 通过（`pnpm -r run typecheck`）
- [ ] 全部 child task 独立 unit test 通过（`pnpm -r run test`）
- [ ] Pipeline 端到端集成测试：fixture 模式下 `poria pipeline submit <link>` → 7-stage 全部 COMPLETED（或 WAITING_MERGE）
- [ ] 断点续跑集成测试：kill worker → restart → Pipeline 从中断点恢复
- [ ] 门禁集成测试：CR 评分不达标 → dev→cr 回退 → 第二次不达标 → BLOCKED
- [ ] 回滚集成测试：`poria pipeline rollback <id>` → worktree/branch/MR 清理
- [ ] 不依赖 `@dj-lib/poria-*` 外部包（submodules 仅作只读参考）

## 实施顺序建议

1. **Wave 1**: core-foundation（约 2-3 天）
2. **Wave 2**: infra-persistence + channel-migration + resource-layer 并行（约 3-5 天/各）
3. **Wave 3**: pipeline-orchestrator（约 3-5 天）
4. **Wave 4**: cli-commands（约 2-3 天）
5. **集成验收**: 跨 child 端到端测试
