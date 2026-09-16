# Pipeline Orchestrator — Implementation Plan

## 实施顺序

### Step 1: Skills 骨架 + ISkill 实际接口

> 先建 skill 骨架，Executor 才能有东西可调度

- [ ] 1.1 `packages/skills/` 目录结构 + 各 skill 的 `package.json` + `tsconfig.json`
- [ ] 1.2 每个 skill 实现 `ISkill` 接口：`execute(input, ctx) → SkillOutput`
- [ ] 1.3 fixture 模式：`PORIA_PIPELINE_FIXTURE=1` 时返回预设输出
- [ ] 1.4 init skill: 调用 xingyun + joyspace，解析链接 + 导出 PRD
- [ ] 1.5 review-prd skill: Claude Agent 分析 PRD，产出 PRD_REVIEW.md
- [ ] 1.6 gen-trd skill: Claude Agent 生成 TRD.md + trdScope
- [ ] 1.7 workspace skill: WorktreeResource 创建 worktree + xingyun 分支绑定
- [ ] 1.8 gen-code skill: Claude Agent 编码 + OutputGuard 检查
- [ ] 1.9 code-review skill: Claude Agent CR + security-scan 内部调用
- [ ] 1.10 deploy skill: build + push + 幂等 MR 创建 (单 repo 逻辑)
- [ ] 1.11 human-loop skill: JmeChannel 通知 + 回复轮询 + 模糊匹配路由

**验证**: `pnpm -r run typecheck` 通过

### Step 2: ExceptionClassifier + handleStageError

- [ ] 2.1 `packages/commands/pipeline/exception-classifier.ts` — error → IssueClass 分类
- [ ] 2.2 `packages/commands/pipeline/handle-error.ts` — IssuePolicy 路由逻辑
- [ ] 2.3 单元测试: 各异常类型映射正确, retry/block/fail 路径覆盖

**验证**: `vitest run` 测试通过

### Step 3: PipelineExecutor

- [ ] 3.1 `packages/commands/pipeline/executor.ts` — 主循环
- [ ] 3.2 STAGE_SKILL_MAP 映射
- [ ] 3.3 credential 校验入口
- [ ] 3.4 单 repo executeSingleRepo + OutputGuard 集成
- [ ] 3.5 多 repo executeMultiRepo 委托
- [ ] 3.6 cr 出口门禁 + regress 回退 + crFeedback 注入
- [ ] 3.7 deploy 出口门禁 + warn 追加
- [ ] 3.8 deploy → WAITING_MERGE 释放
- [ ] 3.9 buildRollbackInstructions 生成
- [ ] 3.10 单元测试: mock skill 完整 7-stage 流转, regress 路径, error 路径

**验证**: `vitest run` executor 测试全通过

### Step 4: PipelineWorker

- [ ] 4.1 `packages/commands/pipeline/worker.ts` — 文件锁 + 主循环
- [ ] 4.2 consumeQueue 循环
- [ ] 4.3 pollMergeRequests 循环 (60s 间隔, merged/closed/timeout 判定)
- [ ] 4.4 启动时 recovery 调用
- [ ] 4.5 单元测试: 锁互斥, MR 轮询状态流转

**验证**: `vitest run` worker 测试全通过

### Step 5: PipelineRollback

- [ ] 5.1 `packages/commands/pipeline/rollback.ts` — 回滚引擎
- [ ] 5.2 未合并回滚: 逆序执行 rollback 指令
- [ ] 5.3 已合并回滚: per-repo revert commit + revert MR
- [ ] 5.4 幂等保护: close_mr/delete_branch/remove_worktree
- [ ] 5.5 单元测试: 各场景覆盖 (未合并/已合并/单步失败继续)

**验证**: `vitest run` rollback 测试全通过

### Step 6: 包配置 + 集成验证

- [ ] 6.1 `packages/commands/package.json` + `packages/skills/package.json` 依赖声明
- [ ] 6.2 `pnpm install` 无报错
- [ ] 6.3 `pnpm -r run typecheck` 全通过
- [ ] 6.4 `pnpm -r run test` 全通过

## 验收标准 (来自 PRD)

- [ ] Executor 驱动 mock skill 跑完完整 7-stage (init → deploy), 最终 waiting_merge
- [ ] CR 回退: 评分不达标 → dev 重跑 (含 crFeedback) → 仍不达标 → blocked
- [ ] WAITING_MERGE: deploy MR → pipeline waiting_merge → Worker 释放
- [ ] Recovery: running stage 中断 → 重启后 stage retried
- [ ] 多仓编排: repo A 失败 → 依赖 A 的 repo B skipped → 独立 C 正常
- [ ] 回滚 (未合并): 逆序清理, 单步失败不阻断
- [ ] 回滚 (已合并): per-repo revert MR
- [ ] 人工回路: 通知 → 回复路由 → resume/skip/cancel
- [ ] deploy MR 幂等: 同分支已有 open MR 复用
- [ ] 门禁 warn 不阻断, 追加到 MR description
