# Pipeline Orchestrator — PRD

> 依赖: core-foundation, infra-persistence, channel-migration, resource-layer

## Goal

实现 Pipeline 编排引擎：驱动 7-stage 顺序执行、门禁检查、回退重跑、多仓库编排、断点续跑、人工回路协调，以及 MR 等待与回滚。

---

## 核心需求

### R1: PipelineExecutor 主循环

- 从 `SqlitePipelineStore` 加载 Pipeline，驱动 stages 顺序执行
- 每个 Stage 通过 `STAGE_SKILL_MAP` 映射到对应 skill（`skill:init` → `skill:deploy`，共 7 个）
- 每个 Stage 开始前调用 `CredentialGuard.ensureValid()` 校验 SSO cookie
- Stage 执行前检查：若 stage 已 failed 且 `retryCount >= maxRetries`，Pipeline 直接 failed
- 多仓库场景（`pipeline.repos.length > 1` 且 stage 为 dev/cr/deploy）→ 委托 `MultiRepoOrchestrator`；否则单仓执行
- 所有状态写入统一通过 `store.saveStageTx(stage, pipeline, events)` 事务提交

### R2: 阶段出口门禁 + CR 回退

- cr stage 完成后，调用 `GateEngine.evaluate(result, rules, "stage_exit")`
- cr_score 门禁 `onFail: "regress"` → 回退到 dev stage 重跑：
  1. 首次回退：`pipeline.hasRegressed = true`，dev stage 重置为 pending，注入 `crFeedback`（上次 CR 评分 + findings 列表 + 修改指令），cr stage 同步重置
  2. 已回退过仍不达标 → stage/pipeline 进入 blocked → 通知人工
- deploy stage 完成后，调用 `GateEngine.evaluate(result, rules, "deploy")`
  - blocking failure → pipeline blocked + 通知人工
  - warn failure → 追加到 MR description，不阻断

### R3: WAITING_MERGE 状态 + Worker 释放

- deploy stage 创建 MR 后，Pipeline 进入 `waiting_merge` 状态，**立即返回释放 Worker**
- 不在 Executor 内阻塞等待 MR 合并
- MR 轮询由独立的 `pollMergeRequests` 循环负责（见 R4）

### R4: PipelineWorker 队列消费 + MR 轮询

- **单实例互斥**：文件锁（`workspace/db/worker.lock`），PID 检查死进程可抢占
- **启动时恢复**：调用 `PipelineRecovery.recoverAll()`
- **两个并行循环**：
  1. `consumeQueue()` — 从队列 dequeue → `executor.run()`，空队列时 sleep 5s
  2. `pollMergeRequests()` — 每 60s 查所有 `waiting_merge` Pipeline 的 MR 状态
     - 全部 merged → Pipeline completed
     - 任一 closed → Pipeline failed + 通知
     - mrUrls 为空（deploy output 丢失）→ Pipeline failed
     - deploy 完成超 24h 未合并 → 升级通知

### R5: MultiRepoOrchestrator

- 按 `dependsOn` 拓扑排序（Kahn's algorithm），循环依赖 → 抛错终止
- MVP 串行执行：逐 repo 调用对应 stage 的 skill
- **依赖熔断**：repo A 失败 → 依赖 A 的 repo B 自动 skipped（传递熔断）→ 不依赖 A 的 repo C 继续
- 聚合产出：`mergeRepoOutputs` 合并各 repo 的 output（如 deploy 阶段聚合 `mrUrls: string[]`）
- deploy skill 始终单 repo 逻辑，MultiRepo 负责分发（避免 N*N MR 问题）

### R6: 回滚机制

- **每个 Stage 完成时生成 `RollbackInstruction`**：
  - workspace stage → `remove_worktree` + `delete_branch`
  - deploy stage → `close_mr`（统一用 `mrUrls: string[]`）
- **`poria pipeline rollback <id>` 执行逻辑**：
  - MR 未合并：按 Stage 逆序执行 rollback 指令
  - MR 已合并：per-repo 创建 revert commit + revert MR → 京ME 通知 reviewer
- **幂等保护**（每个 rollback command）：
  - `close_mr` → 先查 MR 状态，已 closed/merged 跳过
  - `delete_branch` → `git push origin --delete ... || true`
  - `remove_worktree` → `existsSync` 先检查
  - 单步失败不阻断整体回滚，记录 warn 继续

### R7: 7-Skill 骨架

每个 skill 实现 `ISkill` 接口，支持 fixture 模式：

| Skill       | 输入                   | 输出                                                 | 关键逻辑                                 |
| ----------- | ---------------------- | ---------------------------------------------------- | ---------------------------------------- |
| init        | 行云链接               | projectDir + prdPath + demandMetadata                | 链接解析 + JoySpace 导出 PRD             |
| review-prd  | PRD.md                 | PRD_REVIEW.md (P0/P1/P2)                             | Agent 分析 PRD 文本                      |
| gen-trd     | PRD + PRD_REVIEW       | TRD.md + trdScope                                    | Agent 生成技术设计，提取允许修改文件列表 |
| workspace   | 项目配置               | worktreePath + branch + changeId + gitlabProjectPath | Git worktree 创建 + 星云分支绑定         |
| gen-code    | TRD.md (+ crFeedback?) | 代码变更                                             | Agent 编码，OutputGuard 检查产出         |
| code-review | 代码 diff              | CR_REPORT.md + crScore + findings                    | Agent CR + 内部调用 security-scan        |
| deploy      | 门禁通过的代码         | mrUrls + perRepo 状态                                | Build + Push + 幂等 MR 创建              |

- P1 实现真实逻辑骨架 + fixture fallback，确保 Executor 可驱动
- 每个 skill 的 `execute(input, ctx)` 返回 `SkillOutput`

### R8: 人工回路协调

- **异常分类**：`ExceptionClassifier.classify(error)` → `IssueClass`
- **handleStageError 处理流程**：
  1. 查 `ISSUE_POLICIES[issueClass]`
  2. 可重试且未耗尽 → `stage.status = "failed"`, `retryCount++`，不改 pipeline.status（下次循环重试）
  3. 重试耗尽或不可重试 + 有通知角色 → `stage/pipeline = blocked` → 京ME 通知
  4. 无通知角色 → `stage/pipeline = failed`
- **京ME 通知**：
  - 消息格式：Pipeline ID + 需求名 + 阶段 + 问题分类 + 详情（截断 500 字）+ 操作关键字
  - 通知目标：按 `notifyRoles` 路由到 demand.processor/proposer/ops
  - 支持多角色通知（`notifyRoles: string[]`）
- **回复轮询 + 路由**：
  - 模糊匹配：包含"修复/fix" → resume；包含"跳过/skip" → skip；包含"取消/cancel" → cancel
  - 无法解析 → 记录事件日志 + 继续等待
  - 超时 → 升级通知
- **恢复后重启**：BLOCKED Pipeline 在 recovery 时重新发送京ME 提醒

### R9: 异常处理完整覆盖

- Executor catch 块统一走 `handleStageError`
- `OutputGuardError` → 回滚 worktree dirty state + 标记异常
- `AuthExpiredDuringPipelineError` → blocked + 通知用户重新登录
- `MultiRepoPartialFailure` → 记录 per-repo 状态 + 按策略处理

---

## 约束

- 依赖 `core-foundation` 提供的类型、状态机、GateEngine、事件定义
- 依赖 `infra-persistence` 提供的 SqlitePipelineStore、EventStore、PipelineQueue、Recovery
- 依赖 `channel-migration` 提供的 CodingChannel（getMrStatus、findMr、createMrIdempotent）、XingyunChannel、JoySpaceChannel、JmeChannel
- 依赖 `resource-layer` 提供的 ClaudeAgentPool、OutputGuard、TerminalResource、WorktreeResource
- MVP 单 Pipeline 串行，队列排队
- 所有写入统一通过 `saveStageTx` 事务提交，不使用分散的 save 调用

---

## Acceptance Criteria

- [ ] Executor 驱动 mock skill 跑完完整 7-stage Pipeline（init → deploy），最终状态为 `waiting_merge`
- [ ] CR 回退：CR 评分不达标 → dev stage 重跑（input 含 crFeedback）→ 第二次仍不达标 → Pipeline blocked
- [ ] WAITING_MERGE：deploy 创建 MR → pipeline.status = waiting_merge → Worker 释放可消费下一条
- [ ] Recovery：running stage 被中断 → 重启后 worktree cleaned + stage retried
- [ ] 多仓编排：repo A 失败 → 依赖 A 的 repo B skipped → 独立 repo C 正常执行
- [ ] 回滚（未合并）：`rollback` 按逆序清理 worktree/branch/MR，单步失败不阻断
- [ ] 回滚（已合并）：per-repo 创建 revert MR → 京ME 通知 reviewer
- [ ] 人工回路：通知 → 回复"修复" → Pipeline resume；回复"取消" → Pipeline cancel；超时 → 升级通知
- [ ] deploy MR 幂等：同分支已有 open MR → 复用而非重复创建
- [ ] 门禁 warn 项不阻断 Pipeline，追加到 MR description
