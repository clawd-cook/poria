# Poria 平台化架构设计 — Design v4.3

> v4.3: 源头修正 F-01~F-10（Recovery 事务/GateEngine 签名/trdScope/security_scan 定位/handleStageError）。

## 1. 设计原则

| # | 原则 | 含义 |
|---|------|------|
| P1 | **能力即包** | 每个 channel/resource/command/skill 独立发包、独立测试、独立安装 |
| P2 | **自举安装** | Poria 通过 init 装载自身产出的能力 + 第三方能力 |
| P3 | **门禁前置** | 每个阶段有明确的准入/准出条件，不满足不推进 |
| P4 | **可恢复** | SQLite 持久化 + 断点续跑，任何中断都能恢复 |
| P5 | **可审计** | 全部 git 操作留痕，Agent 输出受限，一键回滚 |

---

## 2. 入口协议：行云卡片链接

### 2.1 链接格式（基于真实链接验证）

```
真实格式: http://xingyun.jd.com/demands/view/{demandCode}/-1?demandId={demandId}
示例:      http://xingyun.jd.com/demands/view/JL3R4IV4/-1?demandId=4840029
```

已有实现 `parseXingyunDemandUrl()` (channel-xingyun/src/demandUrl.ts) 可直接复用：
- 白名单域名：`*.xingyun.jd.com`
- 从 query param `demandId` 或 `id` 提取 `demandId: number`
- 从路径 `/demands/view/{code}` 提取 `demandCode: string`
- 两个字段都在 URL 中，无需中间查询步骤

```typescript
// 直接复用，迁移到 packages/channels/xingyun/
import { parseXingyunDemandUrl } from "./demandUrl.js";

// parseXingyunDemandUrl("http://xingyun.jd.com/demands/view/JL3R4IV4/-1?demandId=4840029")
// → { demandId: 4840029, demandCode: "JL3R4IV4", url: "http://..." }
```

### 2.2 入口校验流程

```
用户粘贴链接
  │
  ├─▶ parseXingyunDemandUrl(url)
  │     └─ 抛异常 → 返回具体错误信息（域名/格式/缺 demandId）
  │
  ├─▶ SSO 鉴权（poria-auth cookie 校验）
  │     └─ AuthRequiredError → "请先运行 poria auth login"
  │
  ├─▶ getDemandById(credentials, demandId)
  │     已有实现 (jacp/demands.ts): GET /openapi/v3/demands/{demandId}
  │     ├─ 请求失败 → "需求不存在或已删除"
  │     ├─ 认证失效 → "登录态已失效，请重新登录"
  │     └─ 返回 DemandDetail { id, demandCode, name, status, projectId,
  │                            processor, proposer, receiver, extendedFields }
  │
  ├─▶ listCardAttachments({ demandCode })
  │     已有实现: GET /openapi/v3/cards/code/{demandCode} → attachments[]
  │
  ├─▶ resolvePrdFromAttachments(attachments)
  │     已有实现 (prd.ts):
  │     1. 过滤 JoySpace 链接（isJoySpacePrdLink）
  │     2. 单个 → 使用
  │     3. 多个 → 优先 name 含 "PRD/需求" 的
  │     4. 仍有歧义 → PrdResolveError("AMBIGUOUS_PRD") → 通知用户选择
  │     5. 零个 → PrdResolveError("NO_PRD") → "需求未关联 PRD"
  │
  └─▶ 创建 Pipeline 实例 → 进入静默执行
```

### 2.3 解析产出的元数据

```typescript
// 基于真实 JACP API 响应字段
interface DemandMetadata {
  demandId: number;             // URL query param
  demandCode: string;           // URL path (如 "JL3R4IV4")
  name: string;                 // 需求名称
  status?: number;              // 需求状态码
  projectId?: number;           // 所属项目
  processor?: UserVO;           // 处理人 { erp, name, orgId, orgName }
  proposer?: UserVO;            // 提出人（MR reviewer 候选）
  receiver?: UserVO;            // 接收人
  prdUrl: string;               // 从卡片附件解析出的 JoySpace PRD 链接
  attachments: CardAttachment[];// 全部卡片附件 { tagName, name, url }
  rawLink: string;              // 原始链接，用于审计
}

// 辅助：自动生成分支名（已有实现）
// featureBranchName("JL3R4IV4", 4840029) → "feature_JL3R4IV4"
// featureSlug("JL3R4IV4", 4840029) → "feat-jl3r4iv4"
```

---

## 3. 包结构（完整版）

```
poria/
├── packages/
│   ├── core/                        # @core — 共享原语
│   │   ├── types/                   #   共享类型定义
│   │   ├── pipeline/                #   Pipeline 状态机 + 事件 + 门禁规则
│   │   │   ├── state-machine.ts     #     Pipeline/Stage 状态流转
│   │   │   ├── events.ts            #     领域事件定义
│   │   │   ├── gates.ts             #     门禁规则引擎
│   │   │   └── risk-classifier.ts   #     变更风险分级
│   │   └── contracts/               #   IChannel/IResource/ICommand/ISkill 接口
│   │
│   ├── infrastructure/              # @infrastructure — 基建层
│   │   ├── auth/                    #   SSO cookie + 身份传递
│   │   ├── config/                  #   全局配置（门禁阈值、超时、重试次数）
│   │   ├── logger/                  #   结构化日志
│   │   ├── store/                   #   SQLite 持久化
│   │   │   ├── schema.ts            #     表结构（pipelines, stages, events, audit_log）
│   │   │   ├── pipeline-repo.ts     #     Pipeline CRUD
│   │   │   ├── event-store.ts       #     事件追加/查询
│   │   │   ├── audit-store.ts       #     审计日志写入
│   │   │   └── recovery.ts          #     断点续跑恢复
│   │   ├── plugin-loader/           #   能力包发现 + 加载 + 注册
│   │   └── metrics/                 #   监控指标采集
│   │       ├── collector.ts         #     指标收集器
│   │       └── reporters/           #     上报适配器（stdout/file/...）
│   │
│   ├── channels/                    # @channels — 外部平台集成
│   │   ├── coding/                  #   EasyCI 仓库/分支/MR
│   │   ├── joyspace/                #   JoySpace → Markdown
│   │   ├── xingyun/                 #   星云需求/卡片/PRD/分支绑定/链接解析
│   │   ├── jme/                     #   京ME 消息（JoyClaw MCP 桥）
│   │   └── defect/                  #   缺陷管理
│   │
│   ├── resources/                   # @resources — 本机资源
│   │   ├── claude/                  #   Claude Code Agent SDK 调度
│   │   │   ├── agent-pool.ts        #     Agent SDK startup() 池 + query() 调度
│   │   │   ├── output-guard.ts      #     输出拦截（文件范围/diff 量/恶意依赖）
│   │   │   └── session-tracker.ts   #     session_id 管理（断点续跑用）
│   │   ├── terminal/                #   Shell 执行
│   │   └── worktree/                #   Git worktree 生命周期
│   │
│   ├── commands/                    # @commands — CLI 命令
│   │   ├── pipeline/                #   pipeline submit/status/list/replay/resume/cancel/rollback
│   │   ├── project/                 #   project create/list/status
│   │   ├── workspace/               #   workspace enter/exit/status
│   │   └── auth/                    #   auth login/logout/status
│   │
│   ├── skills/                      # @skills — AI 能力
│   │   ├── init/                    #   需求解析 + PRD 拉取
│   │   ├── review-prd/              #   PRD 澄清
│   │   ├── gen-trd/                 #   技术设计
│   │   ├── gen-code/                #   AI 编码
│   │   ├── code-review/             #   AI 代码审查 + 门禁评分 + 内部调用 security-scan
│   │   ├── security-scan/           #   安全扫描（不是独立 Stage，由 code-review 内部调用）
│   │   ├── workspace/               #   Git worktree 创建 + 分支绑定
│   │   ├── deploy/                  #   构建 + 推送 + MR 创建 + 门禁标记
│   │   └── human-loop/              #   人工回路协调
│   │
│   └── dashboard/                   # @dashboard — 可视化
│       └── src/
│
├── apps/
│   ├── web/                         # Next.js 全栈
│   └── desktop/                     # Tauri 桌面
│
└── workspace/                       # 运行时数据（gitignored）
    ├── db/                          #   SQLite 文件
    │   ├── poria.db
    │   └── backup/                  #   每日备份
    ├── projects/                    #   Pipeline 工作目录
    ├── archive/                     #   事件归档（按月/Pipeline ID）
    │   └── {YYYY-MM}/
    │       └── events-{pipelineId}.jsonl
    └── logs/                        #   结构化日志
```

---

## 4. Pipeline 编排（异步 + 持久化）

### 4.1 编排模型

```typescript
// packages/commands/pipeline/executor.ts

const STAGE_SKILL_MAP: Record<StageEnum, string> = {
  init:       "skill:init",
  review_prd: "skill:review-prd",
  design:     "skill:gen-trd",
  workspace:  "skill:workspace",
  dev:        "skill:gen-code",
  cr:         "skill:code-review",  // 内部调用 skill:security-scan
  deploy:     "skill:deploy",       // 单 repo 逻辑，MultiRepo 由编排器分发
};

class PipelineExecutor {
  constructor(
    private store: SqlitePipelineStore,   // 统一事务写入
    private loader: IPluginLoader,
    private metrics: IMetricsCollector,
    private multiRepo: MultiRepoOrchestrator,
    private credentialGuard: CredentialGuard,
    private humanLoop: IHumanLoop,
  ) {}

  async run(pipelineId: string): Promise<void> {
    const pipeline = await this.store.load(pipelineId);

    if (pipeline.status === "created") {
      pipeline.status = "running";
      await this.store.saveStageTx(null, pipeline, [PipelineStartedEvent(pipeline)]);
    }

    while (pipeline.hasNextStage()) {
      const stage = pipeline.currentStage();
      stage.skillId = STAGE_SKILL_MAP[stage.name];

      // L-05: 入口检查 — recovery 后重试耗尽直接退出
      if (stage.status === "failed" && stage.retryCount >= stage.maxRetries) {
        pipeline.status = "failed";
        await this.store.saveStageTx(stage, pipeline, [StageFailedFinalEvent(stage)]);
        return;
      }

      // 每个 Stage 开始前校验 credentials (A3)
      await this.credentialGuard.ensureValid(await this.store.getCredentials());

      stage.status = "running";
      stage.startedAt = new Date();
      await this.store.saveStageTx(stage, pipeline, [StageStartedEvent(stage)]);

      try {
        // 多仓库 dev/cr/deploy → MultiRepoOrchestrator 分发
        const result = (pipeline.repos.length > 1 && ["dev", "cr", "deploy"].includes(stage.name))
          ? await this.executeMultiRepo(stage, pipeline)
          : await this.executeSingleRepo(stage, pipeline);

        // ── 阶段出口门禁（cr stage: CR 评分检查 → 可能回退到 dev）
        const exitGates = GateEngine.evaluate(result, pipeline.config.gates, "stage_exit");
        const regress = exitGates.details.find(g => !g.pass && g.rule.onFail === "regress");

        if (regress) {
          if (!pipeline.hasRegressed) {
            pipeline.hasRegressed = true;
            const crOutput = stage.output ?? result.output;
            pipeline.regressTo(regress.rule.regressTo!, {
              crFeedback: { score: crOutput?.crScore, findings: crOutput?.findings },
            });
            await this.store.saveStageTx(stage, pipeline, [
              StageRegressionEvent(stage.name, regress.rule.regressTo!, "Gate regress"),
            ]);
            continue;
          }
          // 已回退过 → 人工
          stage.status = "blocked";
          pipeline.status = "blocked";
          await this.store.saveStageTx(stage, pipeline, [StageBlockedEvent(stage, "low_cr_score")]);
          await this.humanLoop.notify(pipeline, stage, "low_cr_score");
          return;
        }

        // ── deploy 门禁
        if (stage.name === "deploy") {
          const deployGates = GateEngine.evaluate(result, pipeline.config.gates, "deploy");
          if (!deployGates.allPass) {
            pipeline.status = "blocked";
            await this.store.saveStageTx(stage, pipeline, [GateBlockedEvent(stage, deployGates)]);
            await this.humanLoop.notify(pipeline, stage, "gate_failure");
            return;
          }
          if (deployGates.warnFailures.length > 0) {
            result.output.mrDescription += formatWarnGatesSummary(deployGates.warnFailures);
          }
        }

        // ── Stage 完成
        stage.status = "completed";
        stage.output = result.output;
        stage.completedAt = new Date();

        // deploy 完成 → WAITING_MERGE（释放 Worker，不阻塞队列）
        if (stage.name === "deploy") {
          pipeline.status = "waiting_merge";
          await this.store.saveStageTx(stage, pipeline, [PipelineWaitingMergeEvent(pipeline)]);
          return; // Worker 自由消费下一条队列
        }

        pipeline.advanceToNext();
        await this.store.saveStageTx(stage, pipeline, pipeline.popEvents());
        this.metrics.recordStageComplete(stage);

      } catch (error) {
        await this.handleStageError(pipeline, stage, error);
        if (stage.status === "blocked") {
          pipeline.status = "blocked";
          await this.store.saveStageTx(stage, pipeline, [StageBlockedEvent(stage, error)]);
          return;
        }
        if (stage.retryCount >= stage.maxRetries) {
          pipeline.status = "failed";
          await this.store.saveStageTx(stage, pipeline, [StageFailedFinalEvent(stage)]);
          return;
        }
        continue; // retry
      }
    }

    // 非 deploy Pipeline（理论上不到这里，deploy 通过 WAITING_MERGE 退出）
    pipeline.status = "completed";
    await this.store.saveStageTx(null, pipeline, [PipelineCompletedEvent(pipeline)]);
  }

  /** 单 repo 执行 — 带输出拦截 */
  private async executeSingleRepo(stage: Stage, pipeline: Pipeline): Promise<GuardedResult> {
    const skill = await this.loader.load<ISkill>(stage.skillId);
    const result = await skill.execute(stage.input, this.buildContext(pipeline));

    // Agent 输出拦截（dev/cr）
    if (stage.name === "dev" || stage.name === "cr") {
      const guard = await this.loader.load<IOutputGuard>("resource:claude:output-guard");
      const guardResult = await guard.check(result, pipeline.config.trdScope);
      if (!guardResult.pass) throw new OutputGuardError(guardResult.violations);
    }

    // 记录 rollback 指令 (L-06)
    stage.rollback = this.buildRollbackInstructions(stage, result, pipeline);

    return { ...result, gatesPass: true };
  }

  /** 多仓库编排 — per-repo 分发 skill */
  private async executeMultiRepo(stage: Stage, pipeline: Pipeline): Promise<GuardedResult> {
    const repoResults = await this.multiRepo.execute(
      pipeline.repos, stage.name, this.buildContext(pipeline),
    );
    if (!repoResults.allSuccess) throw new MultiRepoPartialFailure(repoResults);
    const mergedOutput = this.mergeRepoOutputs(repoResults.results);
    stage.rollback = this.buildMultiRepoRollback(stage, repoResults, pipeline);
    return { output: mergedOutput, gatesPass: true };
  }

  /** L-06: 生成 rollback 指令 */
  private buildRollbackInstructions(stage: Stage, result: SkillOutput, pipeline: Pipeline): RollbackInstruction {
    const commands: RollbackCommand[] = [];
    if (stage.name === "workspace") {
      for (const repo of pipeline.repos) {
        commands.push({ type: "remove_worktree", params: { repo: repo.name, path: result.output?.worktreePath } });
        commands.push({ type: "delete_branch", params: { repo: repo.name, branch: repo.branch } });
      }
    }
    if (stage.name === "deploy") {
      // F-06: 统一为 mrUrls: string[]（单仓也用数组）
      const mrUrls: string[] = result.output?.mrUrls ?? [result.output?.mrUrl].filter(Boolean);
      for (const url of mrUrls) {
        commands.push({ type: "close_mr", params: { mrUrl: url } });
      }
    }
    return { stageIndex: pipeline.stages.indexOf(stage), commands };
  }
}

// 注: 所有写入统一通过 store.saveStageTx(stage, pipeline, events)
// 不再使用 eventStore.append 或 eventStore.appendTx
```

### 4.2 断点续跑

```typescript
// packages/infrastructure/store/recovery.ts

class PipelineRecovery {
  constructor(private store: IPipelineStore, private executor: PipelineExecutor) {}

  /** 系统启动时调用 */
  async recoverAll(): Promise<void> {
    // 1. 查找所有 RUNNING 状态的 Pipeline
    const running = await this.store.findByStatus("running");

    for (const pipeline of running) {
      const lastCompleted = pipeline.stages.findLast(s => s.status === "completed");
      const resumeFrom = lastCompleted
        ? pipeline.stages.indexOf(lastCompleted) + 1
        : 0;

      // F7: 精细恢复 — 区分已 commit 和未 commit 的变更
      const interrupted = pipeline.stages.find(s => s.status === "running");
      if (interrupted) {
        if (interrupted.name === "dev" || interrupted.name === "cr") {
          const worktree = await this.loader.load<IWorktreeResource>("resource:worktree");
          for (const repo of pipeline.repos) {
            await worktree.cleanDirtyState(repo, pipeline.id);
          }
        }

        interrupted.status = "failed";
        interrupted.retryCount++;
        // 统一事务写入（不再使用 eventStore.append / store.saveStage）
        await this.store.saveStageTx(interrupted, pipeline, [
          WorktreeCleanedEvent(interrupted, "recovery: cleaned uncommitted changes"),
        ]);
      }

      pipeline.setCurrentStageIndex(resumeFrom);
      await this.executor.run(pipeline.id);
    }

    // F4: BLOCKED Pipeline 恢复后重新发送京ME提醒
    const blocked = await this.store.findByStatus("blocked");
    for (const p of blocked) {
      const blockedStage = p.stages.find(s => s.status === "blocked");
      if (blockedStage?.issue) {
        // 重启后京ME轮询已断开，需重建轮询或重新通知
        await this.humanLoop.renotify(p, blockedStage);
      }
    }
  }
}
```

### 4.3 串行队列 + 单实例锁

```typescript
// packages/infrastructure/store/queue.ts

class PipelineQueue {
  constructor(private db: Database) {}

  /** 入队 — 幂等（pipeline_id UNIQUE） */
  enqueue(pipelineId: string, priority = 0): void {
    this.db.prepare(`
      INSERT OR IGNORE INTO queue (pipeline_id, priority, enqueued_at)
      VALUES (?, ?, datetime('now'))
    `).run(pipelineId, priority);
  }

  /** 出队 — 原子操作，事务内取出 + 删除，防止重复消费 */
  dequeue(): string | null {
    return this.db.transaction(() => {
      const row = this.db.prepare(`
        SELECT pipeline_id FROM queue ORDER BY priority DESC, enqueued_at ASC LIMIT 1
      `).get() as { pipeline_id: string } | undefined;
      if (!row) return null;
      this.db.prepare(`DELETE FROM queue WHERE pipeline_id = ?`).run(row.pipeline_id);
      return row.pipeline_id;
    })();
  }
}

class PipelineWorker {
  private lockFile: string; // workspace/db/worker.lock

  /** 单实例互斥 — 文件锁防止多进程同时消费 */
  async start(): Promise<void> {
    if (!this.acquireLock()) {
      throw new Error("Another Poria worker is already running");
    }
    try {
      await this.recovery.recoverAll();
      while (!this.stopped) {
        const pipelineId = this.queue.dequeue();
        if (!pipelineId) { await sleep(5000); continue; }
        await this.executor.run(pipelineId);
      }
    } finally {
      this.releaseLock();
    }
  }

  private acquireLock(): boolean {
    // 创建 lockfile 写入 PID，若已存在检查 PID 是否活跃，死进程可抢占
  }
}
```

---

## 5. MR 合并准入门禁

```typescript
// packages/core/pipeline/gates.ts

interface GateRule {
  id: string;
  name: string;
  enabled: boolean;
  threshold: unknown;          // 类型由具体门禁决定
  onFail: "block" | "warn";   // block: 阻断流水线; warn: 标记但继续
}

interface GateResult {
  ruleId: string;
  pass: boolean;
  actual: unknown;
  threshold: unknown;
  message: string;
}

const DEFAULT_GATES: GateRule[] = [
  // deploy 门禁
  { id: "ci_build",       name: "CI 构建",   enabled: true, threshold: null, onFail: "block", gatePhase: "deploy" },
  { id: "test_coverage",  name: "测试覆盖率", enabled: true, threshold: 80,   onFail: "block", gatePhase: "deploy" },
  { id: "security_scan",  name: "安全扫描",   enabled: true, threshold: null, onFail: "block", gatePhase: "stage_exit" },
  { id: "diff_size",      name: "变更量",     enabled: true, threshold: 500,  onFail: "warn",  gatePhase: "deploy" },
  { id: "merge_conflict", name: "合并冲突",   enabled: true, threshold: null, onFail: "block", gatePhase: "deploy" },
  // 阶段出口门禁（cr stage 完成后检查）
  { id: "cr_score",       name: "CR 评分",   enabled: true, threshold: "B+", onFail: "regress", gatePhase: "stage_exit", regressTo: "dev" },
];

class GateEngine {
  /** phase 参数过滤：只执行匹配当前阶段的门禁规则 */
  static evaluate(result: StageResult, rules: GateRule[], phase: "stage_exit" | "deploy"): GateEvaluation {
    const applicable = rules.filter(r => r.enabled && r.gatePhase === phase);
    const results: GateResult[] = applicable
      .map(rule => this.evaluateOne(rule, result));

    const blockingFailures = results.filter(r => !r.pass && this.ruleFor(r.ruleId, rules).onFail === "block");
    const warnFailures = results.filter(r => !r.pass && this.ruleFor(r.ruleId, rules).onFail === "warn");

    return {
      allPass: blockingFailures.length === 0,
      details: results,
      blockingFailures,
      warnFailures, // warn 项不阻断 Pipeline，但触发京ME通知人工关注
    };
  }
}
```

### 门禁通过后的合并流程

```
门禁全部通过
  │
  ├─▶ 拼接 MR description（门禁结果 + warn 项提示 + TRD/CR 链接）
  │
  ├─▶ CodingChannel.createMergeRequest({
  │     title: "[Poria] {demandName}",
  │     description: mrDescription,     // 门禁结果写在 description 中，不用 labels
  │     sourceBranch, targetBranch,
  │     projectId,                       // 通过 projectIdFromGitUrl() 获取
  │   })
  │   → 返回 { url, iid, sourceBranch, targetBranch }
  │
  ├─▶ GitLab CI 自动触发（分支保护要求）
  │
  ├─▶ 京ME 通知 reviewer（demand.proposer?.erp）:
  │     "[Poria] Pipeline #{id} MR 已就绪
  │      需求: {demandName}
  │      门禁: ✅ 全部通过
  │      MR: {mrUrl}
  │      请在 GitLab 审批并合并"
  │
  ├─▶ Pipeline 进入 WAITING_MERGE，释放 Worker（不阻塞队列）
  │     （分支保护要求人工审批，Poria 不能自动合并）
  │
  └─▶ MrWatcher 轮询 MR 状态（每 60s）
        ├─ state: "merged" → Pipeline COMPLETED
        ├─ state: "closed" → Pipeline FAILED + 通知
        └─ 超时 24h → Pipeline BLOCKED + 升级通知
```

---

## 6. 安全与审计

### 6.1 Agent 输出拦截

```typescript
// packages/resources/claude/output-guard.ts

interface OutputGuardConfig {
  allowedPaths: string[];      // TRD 定义的目标文件路径模式
  maxDiffLines: number;        // 单次变更最大行数（默认 500）
  blockedDependencies: string[]; // 已知恶意包名单
}

class OutputGuard {
  check(agentOutput: AgentResult, config: OutputGuardConfig): GuardResult {
    const violations: Violation[] = [];

    // 1. 文件范围检查
    //    trdScope 由 design stage (skill:gen-trd) 生成，存入 pipeline.config.trdScope
    //    格式: ["src/pages/batch-import/**", "src/api/batch-import.ts", "package.json"]
    //    空值 fallback: trdScope 为空/undefined 时跳过文件范围检查（warn 而非 block），
    //    因为空 trdScope 更可能是 TRD 提取失败而非"禁止修改所有文件"
    if (config.allowedPaths.length === 0) {
      violations.push({ type: "trd_scope_empty", severity: "warn",
        message: "trdScope is empty — file scope guard skipped, all changes allowed" });
    } else {
      for (const file of agentOutput.changedFiles) {
        if (!config.allowedPaths.some(p => minimatch(file, p))) {
          violations.push({ type: "out_of_scope", file, severity: "block" });
        }
      }
    }

    // 2. 变更量检查
    if (agentOutput.totalDiffLines > config.maxDiffLines) {
      violations.push({
        type: "diff_too_large",
        actual: agentOutput.totalDiffLines,
        threshold: config.maxDiffLines,
        severity: "warn", // 不阻断，但标记需人工 review
      });
    }

    // 3. 依赖安全检查
    for (const dep of agentOutput.addedDependencies) {
      if (config.blockedDependencies.includes(dep.name)) {
        violations.push({ type: "blocked_dependency", dep, severity: "block" });
      }
    }

    return {
      pass: violations.filter(v => v.severity === "block").length === 0,
      violations,
    };
  }
}
```

### 6.2 审计日志

```typescript
// packages/infrastructure/store/audit-store.ts

// 审计表 (SQLite)
// CREATE TABLE audit_log (
//   id INTEGER PRIMARY KEY,
//   pipeline_id TEXT NOT NULL,
//   stage TEXT,
//   action TEXT NOT NULL,       -- "git_commit" | "git_push" | "mr_create" | "mr_merge" | "branch_delete"
//   operator TEXT NOT NULL,      -- SSO 用户名
//   detail TEXT,                 -- JSON: { branch, commitHash, mrUrl, ... }
//   created_at TEXT NOT NULL
// );

interface AuditEntry {
  pipelineId: string;
  stage?: string;
  action: AuditAction;
  operator: string;         // 实际操作身份（提交者的 SSO）
  detail: Record<string, unknown>;
}
```

### 6.3 回滚机制

```typescript
// packages/commands/pipeline/rollback.ts

interface RollbackInstruction {
  stageIndex: number;
  commands: Array<{
    type: "delete_branch" | "close_mr" | "revert_commit" | "remove_worktree" | "revert_mr";
    params: Record<string, string>;
  }>;
}

// poria pipeline rollback <id>
class PipelineRollback {
  async execute(pipelineId: string): Promise<RollbackResult> {
    const pipeline = await this.store.load(pipelineId);
    const mrStage = pipeline.stages.find(s => s.name === "deploy");
    const mrMerged = mrStage?.output?.mrMerged === true;

    if (mrMerged) {
      // 已合并：per-repo 创建 revert commit + revert MR (L-04: 处理所有仓库)
      const revertMrUrls: string[] = [];
      for (const repo of pipeline.repos) {
        const repoOutput = mrStage.output.perRepo?.find(r => r.repo === repo.name);
        if (!repoOutput?.mergeCommitHash) continue;

        const revertBranch = `revert/${pipeline.id}/${repo.name}`;
        await this.terminal.exec({
          command: `git checkout -b ${revertBranch} origin/${repo.baseBranch} && git revert --no-edit ${repoOutput.mergeCommitHash} && git push origin ${revertBranch}`,
          cwd: this.worktree.path(repo, pipeline.id),
          timeoutMs: 120_000,
        });
        const mr = await this.codingChannel.createMergeRequest({
          title: `[Poria Revert] ${pipeline.demandName} (${repo.name})`,
          description: `Reverts ${repoOutput.mergeCommitHash}`,
          sourceBranch: revertBranch,
          targetBranch: repo.baseBranch,
          projectId: repo.gitlabProjectPath,
        });
        revertMrUrls.push(mr.url);
      }
      await this.jme.send({
        message: `[Poria] Pipeline #${pipelineId} 回滚\n已创建 ${revertMrUrls.length} 个 revert MR:\n${revertMrUrls.join("\n")}\n请审批合并`,
        target: pipeline.operator,
      });
      return { type: "revert_mr_created", mrUrls: revertMrUrls };
    }

    // 未合并：按 Stage 逆序执行 rollback 指令
    const instructions = pipeline.stages
      .filter(s => s.rollback)
      .sort((a, b) => b.stageIndex - a.stageIndex);

    for (const inst of instructions) {
      for (const cmd of inst.rollback.commands) {
        await this.executeRollbackCommand(cmd);
      }
    }
    return { type: "rolled_back" };
  }
}
```

---

## 7. SQLite Schema

```sql
-- Pipeline 主表
CREATE TABLE pipelines (
  id TEXT PRIMARY KEY,
  demand_id INTEGER NOT NULL,
  demand_code TEXT NOT NULL,
  demand_name TEXT,
  status TEXT NOT NULL DEFAULT 'created',  -- created|running|waiting_merge|blocked|completed|failed|cancelled
  raw_link TEXT NOT NULL,
  operator TEXT NOT NULL,                   -- 提交者 SSO 身份
  has_regressed INTEGER DEFAULT 0,          -- dev→cr 回退是否已触发过（限 1 次）
  config TEXT,                              -- JSON: { gates: GateRule[], trdScope: string[], repos: RepoConfig[] }
                                            -- trdScope: design stage 从 TRD.md 提取的允许修改文件路径模式
                                            --   由 skill:gen-trd 产出，写入 config.trdScope
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

-- Stage 明细
CREATE TABLE stages (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  pipeline_id TEXT NOT NULL REFERENCES pipelines(id),
  name TEXT NOT NULL,                       -- init|review_prd|design|workspace|dev|cr|deploy
  status TEXT NOT NULL DEFAULT 'pending',   -- pending|running|completed|failed|blocked|skipped
  skill_id TEXT,
  retry_count INTEGER DEFAULT 0,
  max_retries INTEGER DEFAULT 3,
  input TEXT,                               -- JSON: 该阶段的输入快照
  output TEXT,                              -- JSON: 该阶段的输出
  gate_results TEXT,                        -- JSON: 门禁检查结果（deploy 阶段）
  issue TEXT,                               -- JSON: 如果 failed/blocked
  rollback TEXT,                            -- JSON: 回滚指令
  started_at TEXT,
  completed_at TEXT
);

-- 事件日志（追加写入，用于回放）
CREATE TABLE events (
  seq INTEGER PRIMARY KEY AUTOINCREMENT,
  pipeline_id TEXT NOT NULL REFERENCES pipelines(id),
  kind TEXT NOT NULL,
  payload TEXT NOT NULL,                    -- JSON
  created_at TEXT NOT NULL
);
CREATE INDEX idx_events_pipeline ON events(pipeline_id, seq);

-- 审计日志
CREATE TABLE audit_log (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  pipeline_id TEXT NOT NULL,
  stage TEXT,
  action TEXT NOT NULL,
  operator TEXT NOT NULL,
  detail TEXT,
  created_at TEXT NOT NULL
);
CREATE INDEX idx_audit_pipeline ON audit_log(pipeline_id);

-- 执行队列（串行模型）
CREATE TABLE queue (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  pipeline_id TEXT NOT NULL UNIQUE,
  priority INTEGER DEFAULT 0,
  enqueued_at TEXT NOT NULL
);
```

### 7.2 事件归档策略

```typescript
// packages/infrastructure/store/archiver.ts

class EventArchiver {
  /** 归档已完成 Pipeline 的事件（保留 N 天，默认 30） */
  async archive(retentionDays = 30): Promise<ArchiveResult> {
    const cutoff = new Date(Date.now() - retentionDays * 86400000).toISOString();

    // 1. 找到所有已完成且超过保留期的 Pipeline
    const archived = this.db.prepare(`
      SELECT id FROM pipelines
      WHERE status IN ('completed', 'cancelled', 'failed')
      AND updated_at < ?
    `).all(cutoff);

    // F8: 按 Pipeline ID 归档（每个 Pipeline 一个文件）
    // 目录: workspace/archive/{YYYY-MM}/events-{pipelineId}.jsonl
    for (const { id } of archived) {
      const events = this.db.prepare(`SELECT * FROM events WHERE pipeline_id = ?`).all(id);
      const month = id.slice(0, 7); // Pipeline ID 前缀含日期
      await this.writeJsonl(`workspace/archive/${month}/events-${id}.jsonl`, events);
    }

    // 3. 从 events 表删除（pipelines/stages 元数据保留）
    this.db.prepare(`
      DELETE FROM events WHERE pipeline_id IN (
        SELECT id FROM pipelines WHERE status IN ('completed','cancelled','failed') AND updated_at < ?
      )
    `).run(cutoff);

    return { archivedCount: archived.length };
  }
}

// F5: 事件回放接口 — 同时查 SQLite 和归档 JSONL
class EventReplayService {
  async replayAll(pipelineId: string): Promise<PipelineEvent[]> {
    // 1. 先查 SQLite events 表
    const liveEvents = this.db.prepare(
      `SELECT * FROM events WHERE pipeline_id = ? ORDER BY seq`
    ).all(pipelineId);

    if (liveEvents.length > 0) return liveEvents;

    // 2. SQLite 无数据 → 查归档 JSONL
    const archivePath = await this.findArchiveFile(pipelineId);
    if (archivePath) {
      return this.readJsonl(archivePath);
    }

    return []; // 无事件记录
  }

  private async findArchiveFile(pipelineId: string): Promise<string | null> {
    // 扫描 workspace/archive/*/events-{pipelineId}.jsonl
  }
}
```

### 7.3 SQLite 备份

```
备份策略（单文件数据库）:
  - 每日自动备份: workspace/db/backup/poria-YYYYMMDD.db
  - 使用 SQLite Online Backup API（不阻塞写入）
  - 保留最近 7 天备份，滚动删除
  - 可选: 配置外部备份目录（如 NFS/对象存储）
```

---

## 8. 京ME 人工回路（增强版）

### 8.1 异常分类完整清单

```typescript
enum IssueClass {
  // 可自动重试
  COMPILATION_ERROR  = "compilation_error",
  TEST_FAILURE       = "test_failure",
  AGENT_TIMEOUT      = "agent_timeout",
  LLM_RATE_LIMIT     = "llm_rate_limit",

  // 需人工介入
  REQUIREMENT_AMBIG  = "requirement_ambiguous",
  PRD_INVALID        = "prd_invalid",          // PRD 为空/格式异常/导出失败
  MERGE_CONFLICT     = "merge_conflict",
  LOW_CR_SCORE       = "low_cr_score",
  DIFF_TOO_LARGE     = "diff_too_large",

  // 需运维介入
  PERMISSION_DENIED  = "permission_denied",
  INFRA_FAILURE      = "infra_failure",

  // 安全相关
  SECURITY_VIOLATION = "security_violation",
  OUT_OF_SCOPE       = "out_of_scope_change",

  // 认证相关
  AUTH_EXPIRED       = "auth_expired",

  UNKNOWN            = "unknown",
}

// 每种 IssueClass 的处理策略
const ISSUE_POLICIES: Record<IssueClass, IssuePolicy> = {
  compilation_error:     { autoRetry: 3, notifyRole: "developer", escalateAt: "2h" },
  test_failure:          { autoRetry: 3, notifyRole: "developer", escalateAt: "2h" },
  agent_timeout:         { autoRetry: 1, notifyRole: "developer", escalateAt: "1h" },
  llm_rate_limit:        { autoRetry: 5, notifyRole: null,        escalateAt: null, retryDelay: "5m" },
  requirement_ambiguous: { autoRetry: 0, notifyRole: "product",   escalateAt: "4h" },
  prd_invalid:           { autoRetry: 0, notifyRole: "product",   escalateAt: "4h" },
  merge_conflict:        { autoRetry: 0, notifyRole: "developer", escalateAt: "2h" },
  low_cr_score:          { autoRetry: 1, notifyRole: "developer", escalateAt: "4h" },
  diff_too_large:        { autoRetry: 0, notifyRole: "developer", escalateAt: "4h" },
  permission_denied:     { autoRetry: 0, notifyRole: "ops",       escalateAt: "1h" },
  infra_failure:         { autoRetry: 0, notifyRole: "ops",       escalateAt: "1h" },
  security_violation:    { autoRetry: 0, notifyRole: "security",  escalateAt: "1h" },
  out_of_scope_change:   { autoRetry: 0, notifyRole: "developer", escalateAt: "2h" },
  auth_expired:          { autoRetry: 1, notifyRole: "developer", escalateAt: "1h", note: "自动尝试刷新浏览器 cookie" },
  unknown:               { autoRetry: 0, notifyRole: "developer", escalateAt: "2h" },
};
```

### 8.2 消息格式（京ME）

```
[Poria] Pipeline #{id} 需要协助
━━━━━━━━━━━━━━━━━━━
需求: {demandName} ({demandCode})
阶段: {stage}
问题: {issueClass} — {summary}
详情:
{detail (截断到 500 字)}
━━━━━━━━━━━━━━━━━━━
操作: 回复以下关键字
  ✅ "修复" — 已修复，请重试
  ⏭️ "跳过" — 跳过此阶段
  ❌ "取消" — 取消 Pipeline
  💬 或直接回复说明 → 系统记录后继续等待
```

---

## 9. 监控指标

```typescript
// packages/infrastructure/metrics/collector.ts

interface PoriaMetrics {
  // Pipeline 维度
  pipeline_total: Counter;                   // 创建总数
  pipeline_completed: Counter;               // 完成总数
  pipeline_failed: Counter;                  // 失败总数
  pipeline_duration_seconds: Histogram;      // 端到端耗时

  // Stage 维度
  stage_duration_seconds: Histogram;         // 按 stage name 分桶
  stage_retry_total: Counter;                // 重试次数
  stage_failure_total: Counter;              // 按 issue_class 分桶

  // Agent 维度
  agent_execution_seconds: Histogram;        // Agent 执行耗时
  agent_output_lines: Histogram;             // Agent 输出行数
  agent_output_guard_violations: Counter;    // 输出拦截次数

  // LLM 维度
  llm_tokens_total: Counter;                 // 按 stage + model 分桶
  llm_cost_total: Counter;                   // 估算成本

  // 人工介入维度
  human_loop_total: Counter;                 // 人工介入次数
  human_loop_response_seconds: Histogram;    // 人工回复耗时
  human_loop_escalation_total: Counter;      // 升级次数

  // 门禁维度
  gate_pass_total: Counter;                  // 门禁通过次数（按门禁项）
  gate_fail_total: Counter;                  // 门禁失败次数（按门禁项）
}
```

---

## 10. 多仓库编排 + 熔断

```typescript
// packages/core/pipeline/multi-repo.ts

interface RepoConfig {
  name: string;
  gitUrl: string;
  branch: string;
  baseBranch: string;
  dependsOn?: string[];    // 依赖的仓库名
  buildCmd?: string;
}

class MultiRepoOrchestrator {
  /**
   * 拓扑排序 + 依赖熔断
   *
   * 执行模型（MVP 串行）:
   * 1. 按 dependsOn 拓扑排序
   * 2. 逐个执行（串行，MVP 不做并行）
   * 3. 仓库 A 失败 → 依赖 A 的仓库 B 自动 SKIPPED
   * 4. 不依赖 A 的仓库 C 继续执行
   */
  async execute(repos: RepoConfig[], stage: StageEnum, ctx: Context): Promise<RepoResults> {
    const sorted = this.topologicalSort(repos);
    const failed = new Set<string>();
    const results: RepoResult[] = [];

    for (const repo of sorted) {
      // 熔断检查
      const blockedBy = repo.dependsOn?.find(d => failed.has(d));
      if (blockedBy) {
        results.push({ repo: repo.name, status: "skipped", reason: `依赖 ${blockedBy} 失败` });
        failed.add(repo.name); // 传递熔断
        continue;
      }

      const result = await this.executeForRepo(repo, stage, ctx);
      results.push(result);
      if (!result.success) {
        failed.add(repo.name);
      }
    }

    return { results, allSuccess: failed.size === 0 };
  }

  private topologicalSort(repos: RepoConfig[]): RepoConfig[] {
    // Kahn's algorithm
    // 循环依赖 → 抛错终止
  }
}
```

---

## 11. 灰度策略（P2 优化采纳）

| 阶段 | 范围 | 策略 |
|------|------|------|
| Alpha | 团队内部 + 非核心需求 | 全流程跑通，MR 不自动标记，手动 review |
| Beta | 扩展到合作团队 | 门禁自动 + 一键确认，限制每日 Pipeline 数量 |
| GA | 全量开放 | 完整门禁 + 一键确认，监控指标告警就绪 |

---

## 12. 关键决策汇总

| # | 决策 | 选择 | 理由 |
|---|------|------|------|
| D1 | 入口 | 行云卡片链接（唯一入口） | 含 demandId + demandCode，通过 API 获取完整元数据 |
| D2 | 合并策略 | 门禁自动 + 人工一键确认 | 平衡自动化和安全 |
| D3 | 持久化 | SQLite 本地文件 | 零部署、事务性好、断点续跑 |
| D4 | 并发 | 单 Pipeline 串行 + 队列 | MVP 够用，后续开放并发 |
| D5 | 包组织 | 按能力类型分组 | 独立发包/测试/安装 |
| D6 | 编排 | 异步 Executor + 每步持久化 | 非同步 for 循环，可恢复 |
| D7 | Agent 输出 | OutputGuard 拦截 | 文件范围/diff 量/依赖安全 |
| D8 | 回滚 | 每 Stage 记录 rollback 指令 | 一键回滚 git 变更 |
| D9 | 监控 | 结构化指标（成功率/耗时/token/人工率） | 可观测性基线 |
| D10 | 灰度 | Alpha → Beta → GA | 渐进开放，非核心需求先行 |

---

## 13. 代码审计修正（基于真实 API 实现）

以下修正来自对 `submodules/poria/packages/` 源码的逐文件审计。

### A1: MR 创建 — labels 字段不存在

**问题**：Design §5 使用 `labels: ["poria-auto", "gates-passed"]`，但 `CodingChannel.createMergeRequest` 接口和实现均无 `labels` 参数。底层是 GitLab API v4，虽然 GitLab 支持 labels，但 Poria wrapper 没有透传。

**修正**：
- P1 不依赖 MR labels。门禁状态写入 MR description（已实现）和 Pipeline 事件日志
- warn gate 信息追加到 `description` 参数（在调用 `createMergeRequest` 之前拼接）
- 后续如需 labels 支持，在 `channel-coding/mergeRequest.ts` 增加 `labels?: string` 字段透传到 GitLab API

```typescript
// 修正后的 deploy skill 调用方式：
const mrDescription = [
  `## 门禁结果`,
  ...gateResults.map(g => `- ${g.pass ? "✅" : "❌"} ${g.name}: ${g.message}`),
  warnGates.length > 0 ? `\n⚠️ 以下项需人工关注:\n${warnGates.map(g => `- ${g.name}`).join("\n")}` : "",
  `\n[TRD](${trdUrl}) | [CR Report](${crReportUrl})`,
].join("\n");

await codingChannel.createMergeRequest({
  title: `[Poria] ${demandName}`,
  description: mrDescription,
  sourceBranch,
  targetBranch,
  projectId, // 通过 projectIdFromGitUrl(repo.gitUrl) 获取
});
// 返回 { url, iid, sourceBranch, targetBranch } — 无 labels
```

### A2: init 阶段 JoySpace PRD 导出的具体调用链

**问题**：Design 未具体化 init stage 如何调用 `exportToMarkdown`。

**修正**：init skill 的 PRD 拉取流程：

```typescript
// packages/skills/init/index.ts — init skill 内部

// 1. 从 DemandMetadata 获取 PRD 链接
const prdUrl = metadata.prdUrl; // resolvePrdFromAttachments 的结果

// 2. 调用 JoySpace channel 导出 Markdown
const joyspace = await loader.load<IJoySpaceChannel>("channel:joyspace");
const { outputPath, title } = await joyspace.exportToMarkdown({
  url: prdUrl,
  outputDir: `${projectDir}/source`,
  outputName: "PRD",           // → source/PRD.md
});

// 3. 导出结果作为 stage output，供后续 stage 消费
stage.output = {
  projectDir,
  prdPath: outputPath,         // 绝对路径
  prdTitle: title,
  demandMetadata: metadata,
};
```

**注意**：`exportToMarkdown` 内部处理了图片/表格/流程图（Mermaid 转换），无需后处理。

### A3: Auth cookie 有效期与中途过期处理

**问题**：`poria-auth` 的 `getCredentials()` 只读本地文件 `~/.poria/auth.json`，不做服务端校验。cookie 可能已过期但本地检查通过，实际 API 调用时才报 401。Pipeline 跑到一半 cookie 过期无处理。

**修正**：

```typescript
// packages/infrastructure/auth/credential-guard.ts

class CredentialGuard {
  /** 在 Pipeline 每个 Stage 开始前调用，主动探测 cookie 有效性 */
  async ensureValid(credentials: JacpCredentials): Promise<JacpCredentials> {
    try {
      // 用轻量 API 探测（getDemandById 太重，用 /openapi/v3/user/info 或类似）
      await jacpFetch(credentials, "/openapi/v3/user/info", { method: "GET" }, "身份探测");
      return credentials;
    } catch (e) {
      if (isAuthExpired(e)) {
        // cookie 已过期 → 尝试自动刷新（重新读浏览器 cookie）
        try {
          const refreshed = await reExtractBrowserCookie();
          await saveCredentials(refreshed);
          return refreshed;
        } catch {
          // 刷新也失败 → Pipeline 进入 BLOCKED，通知用户重新登录
          throw new AuthExpiredDuringPipelineError(
            "Cookie 已过期且自动刷新失败。请运行 poria auth login 后，系统将自动恢复 Pipeline。"
          );
        }
      }
      throw e;
    }
  }
}

// Pipeline Executor 中的集成：
async run(pipelineId: string): Promise<void> {
  // ...
  while (pipeline.hasNextStage()) {
    // 每个 Stage 开始前校验 credentials
    const credentials = await this.credentialGuard.ensureValid(
      await this.authStore.getCredentials()
    );
    // ... 使用 credentials 执行 stage
  }
}
```

**IssueClass 新增**：`AUTH_EXPIRED = "auth_expired"` — autoRetry: 1（自动刷新浏览器 cookie），失败后通知用户。

### A4: Terminal exec 超时配置

**问题**：`TerminalResource.exec()` 默认 30s 超时。Design §6.3 rollback 的 `git revert` 可能超时。

**修正**：所有 git 操作显式指定超时：

```typescript
// 设计约定：不同操作的超时档位
const TIMEOUT = {
  GIT_SHORT: 30_000,     // git status, git branch（默认 30s 够用）
  GIT_MEDIUM: 120_000,   // git push, git revert, git merge（2 分钟）
  GIT_LONG: 300_000,     // git clone, 大仓 git push（5 分钟）
  BUILD: 600_000,        // npm run build（10 分钟）
  AGENT: 1_800_000,      // Claude Code agent 执行（30 分钟）
};

// 示例：rollback 中的 git revert
await terminal.exec({
  command: `git revert --no-edit ${mergeCommit}`,
  cwd: worktreePath,
  timeoutMs: TIMEOUT.GIT_MEDIUM,  // 120s
});

await terminal.exec({
  command: `git push origin ${revertBranch}`,
  cwd: worktreePath,
  timeoutMs: TIMEOUT.GIT_MEDIUM,  // 120s
});
```

### A5: bindBranch changeId 持久化 + fixture 模式复用

**问题 1**：`XingyunChannel.bindBranch()` 返回 `{ branch, changeId, baseBranch }`，`changeId` 是 EasyCI 变更记录 ID。Design workspace stage 未持久化 `changeId`。

**修正**：workspace stage output 必须包含 `changeId`：

```typescript
// workspace stage output
stage.output = {
  repos: pipeline.repos.map(repo => ({
    name: repo.name,
    branch: bindResult.branch,
    baseBranch: bindResult.baseBranch,
    changeId: bindResult.changeId,  // EasyCI 变更记录 ID，deploy 阶段可能需要
    worktreePath: worktree.path(repo),
  })),
};
```

**问题 2**：所有 channel 都有 fixture 模式（`PORIA_XINGYUN_FIXTURE=1` 等），Design 测试策略应复用。

**修正**：测试策略更新：

```
测试层级：
  1. 单元测试 — 纯逻辑，零 I/O
  2. 集成测试（fixture 模式）— 设置 PORIA_*_FIXTURE=1 环境变量
     所有 channel 返回内置 mock 数据，无需网络。
     比自建 In-Memory 替身更贴近真实行为。
  3. E2E 测试 — 真实 API（需 cookie）

fixture 模式是已有资产，优先复用，仅在 fixture 不覆盖的场景才写新 mock。
```

### A6: projectId 在 MR 创建和回滚中的获取

**问题**：`createMergeRequest` 需要 `projectId`（GitLab project path），Design 回滚的 revert MR 创建也需要。但 Design 未说明如何获取。

**修正**：已有实现 `projectIdFromGitUrl()` (channel-coding/src/mergeRequest.ts) 从 git remote URL 推导 GitLab project path：

```typescript
// 直接复用：
import { projectIdFromGitUrl } from "channel-coding/mergeRequest.js";

const projectId = projectIdFromGitUrl(repo.gitUrl);
// "git@coding.jd.com:group/repo.git" → "group/repo"
```

workspace stage output 应同时记录 `projectId` 供 deploy/rollback 使用。

### A7: Claude Code Agent 调度（基于 Agent SDK 验证）

**问题**：Design 假设通过 subprocess spawn `claude -p` 调度 Agent。实际 Anthropic 提供了 Agent SDK (`@anthropic-ai/claude-agent-sdk`)，是官方推荐的 Node.js 程序化调度方式。

**关键依赖**：`@anthropic-ai/claude-agent-sdk`（npm 包，内置 Claude Code 二进制，无需独立安装 CLI）。

**设计修正**：

```typescript
// packages/resources/claude/agent-pool.ts

import { startup, query, type Query, type SDKMessage } from "@anthropic-ai/claude-agent-sdk";

class ClaudeAgentPool {
  private warm: Awaited<ReturnType<typeof startup>> | null = null;

  /** Worker 进程启动时调用一次 — warm-start，加载二进制 */
  async initialize(): Promise<void> {
    this.warm = await startup({
      options: {
        allowedTools: ["Read", "Edit", "Bash", "Glob", "Grep"],
        permissionMode: "auto",
        permissionPrompts: "none",  // 永不阻塞等待人工确认
      },
      initializeTimeoutMs: 60_000,
    });
  }

  /** 调度一次 Agent 任务 */
  async dispatch(input: AgentTaskInput): Promise<AgentTaskResult> {
    if (!this.warm) throw new Error("AgentPool not initialized");

    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), input.timeoutMs ?? 1_800_000); // 30min

    const messages: SDKMessage[] = [];
    let sessionId: string | undefined;

    try {
      const q = this.warm.query(input.prompt, {
        cwd: input.worktreePath,
        maxBudgetUsd: input.maxBudgetUsd ?? 5.0,
        maxTurns: input.maxTurns ?? 50,
        abortController: controller,
        persistSession: true,       // 保存 session 用于断点续跑
        model: input.model ?? "sonnet",
        appendSystemPrompt: input.systemPrompt,
        // 安全约束: 不用 bypassPermissions，用 auto + none
        allowedTools: [
          "Read", "Edit", "Bash", "Glob", "Grep",
          // 按 stage 动态添加:
          ...input.extraTools ?? [],
        ],
      });

      sessionId = q.sessionId;

      for await (const msg of q) {
        messages.push(msg);

        // 实时上报 progress 事件到 Pipeline 事件日志
        if (msg.type === "assistant" && msg.message?.content) {
          await input.onProgress?.(msg);
        }
      }

      // 最后一条 result 消息
      const resultMsg = messages.findLast(m => m.type === "result");
      return {
        success: true,
        result: resultMsg?.result ?? "",
        sessionId,
        costUsd: resultMsg?.total_cost_usd,
        messages,
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : String(error),
        sessionId,
        messages,
      };
    } finally {
      clearTimeout(timer);
    }
  }

  /** 断点续跑 — 恢复被中断的 Agent session */
  async resume(sessionId: string, prompt: string): Promise<AgentTaskResult> {
    // Agent SDK 支持 --resume <session_id>
    // warm.query() 暂不直接支持 resume，降级为 CLI subprocess
    // claude -p "<prompt>" --resume <sessionId> --output-format json
  }
}

interface AgentTaskInput {
  prompt: string;               // 给 Agent 的指令
  worktreePath: string;         // Agent 的工作目录
  systemPrompt?: string;        // 追加系统指令（如 TRD 内容）
  model?: string;               // 默认 sonnet
  maxBudgetUsd?: number;        // 默认 $5
  maxTurns?: number;            // 默认 50
  timeoutMs?: number;           // 默认 30min
  extraTools?: string[];        // 额外允许的工具
  onProgress?: (msg: SDKMessage) => Promise<void>; // 进度回调
}

interface AgentTaskResult {
  success: boolean;
  result?: string;
  error?: string;
  sessionId?: string;           // 用于断点续跑
  costUsd?: number;             // LLM 成本
  messages: SDKMessage[];       // 完整消息流（存入事件日志）
}
```

**各 Stage 的 Agent 调度策略**：

| Stage | Agent 用途 | 关键 allowedTools | maxBudgetUsd | maxTurns | timeoutMs |
|-------|-----------|-------------------|-------------|----------|-----------|
| review_prd | 分析 PRD 文本 | Read, Grep | $1 | 10 | 5min |
| design | 生成 TRD | Read, Edit, Grep | $3 | 20 | 10min |
| dev | 编码 | Read, Edit, Bash, Glob, Grep | $10 | 100 | 30min |
| cr | 代码审查 | Read, Bash(npm test), Grep | $5 | 30 | 15min |

**与 OutputGuard 的集成**：

```typescript
// dev/cr stage 执行后，检查 Agent 产出
const agentResult = await this.agentPool.dispatch({ ... });
if (agentResult.success) {
  // OutputGuard 检查 Agent 的代码变更
  const guard = await loader.load<IOutputGuard>("resource:claude:output-guard");
  const diff = await terminal.exec({ command: "git diff", cwd: worktreePath });
  const guardResult = guard.check(diff, pipeline.trdScope);
  if (!guardResult.pass) {
    // Agent 变更超出允许范围 → 回滚 worktree + 标记异常
    await worktree.cleanDirtyState(repo, pipeline.id);
    throw new OutputGuardError(guardResult.violations);
  }
}
```

**Session 持久化（断点续跑）**：

```sql
-- 新增列到 stages 表
ALTER TABLE stages ADD COLUMN agent_session_id TEXT;
-- Agent 执行后记录 sessionId，中断恢复时可用 --resume 续跑
```

### A8: 分支保护与 MR 审批（基于用户确认）

**事实**：master/main 有分支保护 + 强制审批。MR 合并需要至少 1 个 reviewer approve + CI 通过。

**对设计的影响**：

```
Deploy 阶段的完整流程（修订版）：
  1. Build（npm run build / 各仓库 buildCmd）
  2. 安全扫描（skill:security-scan）
  3. Push feature branch（不直接 push master，分支保护禁止）
  4. createMergeRequest → GitLab MR
  5. 门禁结果写入 MR description
  6. GitLab CI 自动触发（分支保护要求）
  7. 等待 CI 完成（轮询 MR 状态或 webhook）
  8. 京ME 通知 reviewer:
     - reviewer 来源: demand.proposer?.erp（需求提出人）或 demand.processor?.erp
     - 消息内容: MR 链接 + 门禁结果 + "请审批合并"
  9. Pipeline 进入 WAITING_MERGE 状态，释放 Worker（不阻塞队列）
  10. 检测到 MR 已合并（轮询 GitLab API）→ Pipeline COMPLETED

注意：Poria 不能自动合并 MR（分支保护要求人工 approve）。
"一键确认"实际是 reviewer 在 GitLab/EasyCI 上点 approve + merge。
京ME 消息只是通知，实际操作在 GitLab 上完成。
```

**MR 状态轮询**：

```typescript
// packages/skills/deploy/mr-watcher.ts

class MrWatcher {
  /** 轮询 MR 状态直到合并或超时 */
  async waitForMerge(mrUrl: string, timeoutMs = 24 * 3600 * 1000): Promise<MrFinalStatus> {
    const startTime = Date.now();
    while (Date.now() - startTime < timeoutMs) {
      // GitLab API: GET /api/v4/projects/{id}/merge_requests/{iid}
      // 检查 state: "merged" | "closed" | "opened"
      const status = await this.codingChannel.getMrStatus(mrUrl);
      if (status === "merged") return { merged: true };
      if (status === "closed") return { merged: false, reason: "MR was closed" };
      await sleep(60_000); // 每分钟轮询一次
    }
    return { merged: false, reason: "Timeout waiting for merge" };
  }
}
```

**注意**：`CodingChannel` 目前无 `getMrStatus` 方法，需新增。底层是 GitLab API `GET /api/v4/projects/{id}/merge_requests/{iid}`。

### A9: 京ME 消息通道（基于 JoyClaw 实际架构）

**事实**：JoyClaw 是一个 **LLM Agent 系统**（用 JoyAI 模型），不是消息 API。配置在 `~/.joyclaw/openclaw.json`，通过 `jmechat` channel 访问京ME。

**调用方式**：

```typescript
// packages/channels/jme/joyclaw-bridge.ts

// JoyClaw 不是一个 HTTP API — 它是一个本地 agent，通过 CLI 调用
// 需要 JoyClaw gateway 常驻运行（:18810）

import { spawn } from "node:child_process";

class JoyClawBridge {
  private node = `${process.env.HOME}/.joyclaw/node/node-v22.16.0-darwin-arm64/bin/node`;
  private openclaw = `${process.env.HOME}/.joyclaw/apps/v2.5.6/openclaw.mjs`;

  async send(message: string, timeoutSec = 120): Promise<string> {
    // 给 JoyClaw agent 发自然语言指令
    // JoyClaw 内部用 joyme_joychat 工具完成发送
    return this.runAgent(
      `向 ${target} 发送京ME消息: ${message}`,
      timeoutSec,
    );
  }

  async readReplies(chatName: string, since: Date): Promise<string[]> {
    return this.runAgent(
      `查看 ${chatName} 聊天中 ${since.toISOString()} 之后的消息`,
    );
  }

  private async runAgent(message: string, timeoutSec = 300): Promise<string> {
    // spawn(node, [openclaw, "agent", "--agent", "main", "--timeout", timeoutSec, "--json", "--message", message])
    // 解析 stdout 末尾 JSON { payloads: [{ text }] }
  }
}
```

**设计约束**：
1. **消息格式是纯文本** — 调用方传自然语言指令给 JoyClaw agent，agent 自行决定如何操作京ME。不能直接控制消息格式（富文本/卡片/按钮）。
2. **依赖 JoyClaw gateway 常驻** — 如果 gateway 挂了，消息发送超时。需要健康检查。
3. **回复监听是轮询** — 没有 webhook，只能定期调 JoyClaw "查看最近消息" 来检测回复。
4. **JoyClaw 用的是内网 LLM**（JoyAI/Dr.Joy），不是 Claude。消息解析质量取决于这个模型。

**健康检查**：

```typescript
// 启动时和每次发消息前检查 JoyClaw gateway
async ensureGatewayAlive(): Promise<void> {
  // lsof -i :18810 检查端口
  // 或 fetch("http://127.0.0.1:18810/health")
  // 不可用 → 尝试重启 / 通知用户
}
```

**对人工回路设计的影响**：
- 原设计的结构化回复格式（"修复"/"跳过"/"取消"）需要 JoyClaw agent 理解并路由。由于 JoyClaw 用内网 LLM，理解能力有限，建议：
  - 通知消息保持简单纯文本
  - 回复解析放宽：不要求精确关键字，用模糊匹配（包含"修复"/"fix" → resume；包含"跳过"/"skip" → skip；包含"取消"/"cancel" → cancel）
  - 无法解析的回复 → 记录到事件日志 + 继续等待

### A10: 行云需求状态码（基于代码审计 + fixture 数据）

**事实**：代码中无显式状态枚举。从 fixture 和 action 名推断的部分映射：

| 状态码 | 含义 | 来源 |
|--------|------|------|
| 20 | 沟通中 (communicating) | fixture.ts FIXTURE_DEMAND.status + communicate() 返回 |
| 30 | 已受理 (accepted) | accept() fixture 返回 demandStatusCode: 30 |

**代码中从未检查 status 值** — `getDemandById` 返回后不做状态过滤。

**对设计的影响**：

```typescript
// packages/channels/xingyun/demand-guard.ts

// 行云需求状态判定 — 基于已知映射 + 安全默认
const KNOWN_ACTIVE_STATUSES = new Set([20, 30]); // 沟通中、已受理
// 其他已知但未通过代码验证的：10=草稿? 40=已完成? 50=已关闭?
// 由于映射不完整，采用排除法而非白名单

function isDemandActive(status: number | undefined): boolean {
  if (status == null) return true;  // status 缺失视为活跃（API 可能省略）
  // 目前不做状态过滤 — 等收集到完整映射后再加白名单
  // 暂时只在 UI 层展示状态提示，不阻断 Pipeline 创建
  return true;
}

// TODO: 用真实 API 收集完整状态枚举后启用过滤
// 建议：第一次调用 getDemandById(4840029) 记录返回的 status 值
```

**行动项**：首次实现时用真实 cookie 调一次 `getDemandById(4840029)`，记录返回的 status 字段值，逐步建立映射表。

### A11: CodingChannel 接口扩展清单

**事实**：当前 `CodingChannel` 接口不支持以下 deploy 阶段所需能力：

| 能力 | 当前状态 | 需新增 |
|------|---------|--------|
| MR labels | 不支持 | 透传 `labels?: string` 到 GitLab API |
| MR reviewer | 不支持 | 透传 `reviewer_ids?: number[]` |
| MR 状态查询 | 不支持 | 新增 `getMrStatus(projectId, iid): Promise<MrStatus>` |
| MR auto merge | 不支持 | 透传 `merge_when_pipeline_succeeds?: boolean` |
| Push 失败处理 | 无重试 | `git push` 被 pre-receive hook 拒绝时无处理 |

**实施建议**：P1 只需新增 `getMrStatus`（用于等待 MR 合并）。其余在 P2+ 按需添加。

---

## 14. R5 终审修正（F1-F12）

### F1 [CRITICAL]: Worker 死锁 — WAITING_MERGE 挂起态

**问题**：deploy stage 的 MrWatcher 轮询 MR 状态最长 24h，期间 Worker 持有执行槽，队列死锁。

**修正**：Pipeline 新增 `WAITING_MERGE` 状态。deploy stage 创建 MR 后不阻塞等待，而是将 Pipeline 挂起并释放 Worker。独立的 MrPoller 定时检查所有 WAITING_MERGE Pipeline。

```typescript
// §4.1 Executor — deploy 阶段修正

// deploy stage 完成时不等 MR 合并，直接挂起
if (stage.name === "deploy" && result.gatesPass) {
  stage.status = "completed";
  stage.output = result.output; // 含 mrUrls: string[]
  pipeline.status = "waiting_merge";
  await this.store.savePipeline(pipeline);
  await this.eventStore.appendTx(pipeline.id, pipeline.popEvents());
  return; // 释放 Worker，Worker 可消费下一条队列
}

// §4.3 PipelineWorker — 修正后的主循环
class PipelineWorker {
  async start(): Promise<void> {
    if (!this.acquireLock()) throw new Error("Another worker running");
    try {
      await this.recovery.recoverAll();

      // 两个循环并行：队列消费 + MR 轮询
      await Promise.all([
        this.consumeQueue(),
        this.pollMergeRequests(),
      ]);
    } finally {
      this.releaseLock();
    }
  }

  /** 消费队列（只处理到 deploy stage 完成） */
  private async consumeQueue(): Promise<void> {
    while (!this.stopped) {
      const pipelineId = this.queue.dequeue();
      if (!pipelineId) { await sleep(5000); continue; }
      await this.executor.run(pipelineId);
      // run() 在 deploy 创建 MR 后返回（status = waiting_merge）
      // 或在其他阶段异常后返回（status = blocked/failed）
    }
  }

  /** 独立轮询所有 WAITING_MERGE Pipeline 的 MR 状态 */
  private async pollMergeRequests(): Promise<void> {
    while (!this.stopped) {
      const waiting = await this.store.findByStatus("waiting_merge");
      for (const pipeline of waiting) {
        const deployOutput = pipeline.stages.find(s => s.name === "deploy")?.output;
        const mrUrls: string[] = deployOutput?.mrUrls ?? [deployOutput?.mrUrl].filter(Boolean);

        const statuses = await Promise.all(
          mrUrls.map(url => this.codingChannel.getMrStatus(url))
        );

        if (statuses.every(s => s === "merged")) {
          pipeline.status = "completed";
          await this.store.savePipeline(pipeline);
          await this.eventStore.appendTx(pipeline.id, [PipelineCompletedEvent(pipeline)]);
        } else if (statuses.some(s => s === "closed")) {
          pipeline.status = "failed";
          await this.store.savePipeline(pipeline);
          // 京ME 通知
        }
        // 超时检查
        const deployStage = pipeline.stages.find(s => s.name === "deploy");
        if (deployStage && Date.now() - new Date(deployStage.completedAt!).getTime() > 24 * 3600_000) {
          // 24h 超时 → 升级通知
          await this.humanLoop.escalate(pipeline, "MR 超过 24 小时未合并");
        }
      }
      await sleep(60_000); // 每分钟检查一次
    }
  }
}
```

**状态机更新**：

```
Pipeline 级（v4.2 最终版）:
  CREATED ──▶ RUNNING ──▶ WAITING_MERGE ──▶ COMPLETED
    │           │    ▲          │
    │           │    │ resume   │ MR closed / timeout
    │           ▼    │          ▼
    │        BLOCKED ─┘       FAILED
    │           │                │
    └───────────┴────────────────┘
                        │
                        ▼
                    CANCELLED

转换规则:
  CREATED → RUNNING (submit)
  CREATED → CANCELLED (cancel before start)
  RUNNING → WAITING_MERGE (deploy MR created)
  RUNNING → BLOCKED (stage needs human)
  RUNNING → FAILED (retries exhausted)
  RUNNING → CANCELLED (manual cancel)
  BLOCKED → RUNNING (human reply)
  BLOCKED → CANCELLED (manual cancel)
  WAITING_MERGE → COMPLETED (all MRs merged)
  WAITING_MERGE → FAILED (MR closed / 24h timeout)
  WAITING_MERGE → CANCELLED (manual cancel)
  FAILED → CANCELLED (manual cancel)
```

### F2 [HIGH]: CR 评分 — 字符串比较

**问题**：`crScore < crThreshold` 用 `<` 比较字符串 "B+" 是 lexicographic，"A" < "B" 为 true，导致 A 评分被误判为不达标。

**修正**：定义评分等级序号，用数值比较。

```typescript
// packages/core/pipeline/gates.ts

const CR_GRADE_ORDER: Record<string, number> = {
  "A+": 10, "A": 9, "A-": 8,
  "B+": 7,  "B": 6, "B-": 5,
  "C+": 4,  "C": 3, "C-": 2,
  "D": 1,   "F": 0,
};

function crScoreMeetsThreshold(actual: string, threshold: string): boolean {
  const actualNum = CR_GRADE_ORDER[actual] ?? -1;
  const thresholdNum = CR_GRADE_ORDER[threshold] ?? -1;
  return actualNum >= thresholdNum;
}

// GateEngine 中使用：
if (rule.id === "cr_score") {
  return { pass: crScoreMeetsThreshold(actual, threshold), ... };
}
```

### F3 [HIGH]: 门禁统一 — CR 评分检查只走 GateEngine

**问题**：§4.1 Executor 中有 inline crScore 检查（触发回退），§5 GateEngine 也有 cr_score 门禁，两处逻辑重复且可能冲突。

**修正**：移除 Executor 的 inline crScore 检查。引入 GateRule.gatePhase 字段，区分阶段出口门禁和 deploy 门禁。回退逻辑由门禁引擎驱动。

```typescript
interface GateRule {
  id: string;
  name: string;
  enabled: boolean;
  threshold: unknown;
  onFail: "block" | "warn" | "regress"; // 新增 "regress"
  gatePhase: "stage_exit" | "deploy";   // 新增
  regressTo?: StageEnum;                 // regress 目标阶段
}

const DEFAULT_GATES: GateRule[] = [
  // deploy 门禁
  { id: "ci_build",       ..., gatePhase: "deploy", onFail: "block" },
  { id: "test_coverage",  ..., gatePhase: "deploy", onFail: "block" },
  { id: "security_scan",  ..., gatePhase: "deploy", onFail: "block" },
  { id: "merge_conflict", ..., gatePhase: "deploy", onFail: "block" },
  { id: "diff_size",      ..., gatePhase: "deploy", onFail: "warn" },
  // 阶段出口门禁
  { id: "cr_score",       ..., gatePhase: "stage_exit", onFail: "regress", regressTo: "dev" },
];

// Executor 修正：移除 inline crScore 检查，统一走 GateEngine
// cr stage 完成后调 GateEngine.evaluate(result, rules.filter(r => r.gatePhase === "stage_exit"))
// 如果 onFail === "regress" → pipeline.regressTo(rule.regressTo)
// deploy stage 仍走 GateEngine.evaluate(result, rules.filter(r => r.gatePhase === "deploy"))
```

### F4 [HIGH]: 回退注入 CR 反馈

**问题**：dev→cr 回退后，dev Agent 重跑但不知道为什么被退回，会产出类似代码。

**修正**：

```typescript
// Executor — regressTo 时注入 CR 反馈到 dev stage input
if (gateResult.onFail === "regress") {
  const crOutput = pipeline.stages.find(s => s.name === "cr")?.output;
  const devStage = pipeline.stages.find(s => s.name === gateResult.regressTo);

  // 重置 dev stage 但注入 CR 反馈
  devStage.status = "pending";
  devStage.input = {
    ...devStage.input,
    crFeedback: {
      previousScore: crOutput.crScore,
      findings: crOutput.findings,       // CR 具体问题列表
      instruction: "上次 CR 评分不达标，请根据以下问题修改代码后重新提交",
    },
  };
  // 同时重置 cr stage
  const crStage = pipeline.stages.find(s => s.name === "cr");
  crStage.status = "pending";
  crStage.output = undefined;
}
```

### F5 [HIGH]: 多仓库多 MR 等待

**问题**：多仓库 deploy 产出 N 个 MR，MrWatcher 只处理单个。

**修正**：deploy stage output 改为 `mrUrls: string[]`（F1 的 pollMergeRequests 已处理多 MR）。

```typescript
// deploy skill — 多仓库 MR
const mrUrls: string[] = [];
for (const repo of pipeline.repos) {
  const mr = await codingChannel.createMergeRequest({ ... });
  mrUrls.push(mr.url);
}
stage.output = { ...stage.output, mrUrls };

// F1 的 pollMergeRequests 已覆盖：
// statuses.every(s => s === "merged") → COMPLETED
// statuses.some(s => s === "closed") → FAILED（附带 per-repo 状态）
```

### F6 [MEDIUM]: D1 决策理由修正

```
D1 理由修正: "含 demandId + demandCode，通过 API 获取完整元数据"
（删除 "含 projectId" 错误描述）
```

### F7 [MEDIUM]: 事件归档目录 — 按月分组用 Pipeline 创建时间

```typescript
// 修正 EventArchiver — 用 pipeline.created_at 提取月份
const pipeline = this.db.prepare(`SELECT created_at FROM pipelines WHERE id = ?`).get(id);
const month = pipeline.created_at.slice(0, 7); // "2026-09" from ISO date
await this.writeJsonl(`workspace/archive/${month}/events-${id}.jsonl`, events);
```

### F8 [MEDIUM]: Pipeline ID 格式 — 显式定义

```typescript
// packages/core/pipeline/id.ts

// ID 格式: "pl-{YYYYMMDD}-{nanoid(8)}"
// 示例: "pl-20260916-a1b2c3d4"
// 确保 ID 可排序、含日期前缀、无冲突

import { nanoid } from "nanoid";

function createPipelineId(): string {
  const date = new Date().toISOString().slice(0, 10).replace(/-/g, "");
  return `pl-${date}-${nanoid(8)}`;
}
```

### F9 [MEDIUM]: 事件完整性 — 列出所有事件类型

```typescript
// packages/core/pipeline/events.ts — 完整事件类型清单

type PipelineEvent =
  // Pipeline 生命周期
  | { kind: "pipeline_created"; demandRef: DemandMetadata }
  | { kind: "pipeline_started" }
  | { kind: "pipeline_completed" }
  | { kind: "pipeline_failed"; reason: string }
  | { kind: "pipeline_cancelled"; operator: string }
  | { kind: "pipeline_waiting_merge"; mrUrls: string[] }

  // Stage 生命周期
  | { kind: "stage_started"; stage: StageEnum }
  | { kind: "stage_completed"; stage: StageEnum; output: StageOutput }
  | { kind: "stage_failed"; stage: StageEnum; error: string; retryCount: number }
  | { kind: "stage_blocked"; stage: StageEnum; issueClass: IssueClass }
  | { kind: "stage_resumed"; stage: StageEnum; resolution: string }
  | { kind: "stage_regressed"; from: StageEnum; to: StageEnum; reason: string }

  // Agent 执行
  | { kind: "agent_dispatched"; stage: StageEnum; sessionId: string }
  | { kind: "agent_progress"; stage: StageEnum; message: string }
  | { kind: "agent_completed"; stage: StageEnum; costUsd: number }
  | { kind: "agent_failed"; stage: StageEnum; error: string }

  // 门禁
  | { kind: "gate_evaluated"; stage: StageEnum; results: GateResult[] }
  | { kind: "gate_regress_triggered"; rule: string; from: StageEnum; to: StageEnum }

  // 人工回路
  | { kind: "human_assist_requested"; issueClass: IssueClass; target: string }
  | { kind: "human_assist_received"; action: string; message: string }
  | { kind: "human_assist_escalated"; level: number }

  // 凭证
  | { kind: "credential_refreshed" }
  | { kind: "credential_expired"; stage: StageEnum }

  // Git 操作（审计）
  | { kind: "git_commit"; repo: string; hash: string }
  | { kind: "git_push"; repo: string; branch: string }
  | { kind: "mr_created"; repo: string; url: string; iid: number }
  | { kind: "mr_merged"; repo: string; url: string }
  | { kind: "worktree_created"; repo: string; path: string }
  | { kind: "worktree_cleaned"; repo: string; reason: string }
  | { kind: "rollback_executed"; type: string; detail: string };

// 每种事件都有 base 字段: { seq, pipelineId, timestamp, kind }
// 这个完整列表确保事件回放可重建全部 Pipeline 状态
```

### F10 [MEDIUM]: SQLite 事务一致性

**修正**：所有 stage 完成时的 saveStage + savePipeline + appendEvent 包在同一事务中。

```typescript
// packages/infrastructure/store/pipeline-repo.ts

class SqlitePipelineStore {
  /** 事务性写入 — stage + pipeline + events 原子提交 */
  saveStageTx(stage: Stage, pipeline: Pipeline, events: PipelineEvent[]): void {
    this.db.transaction(() => {
      this.db.prepare(`UPDATE stages SET ...`).run(stage);
      this.db.prepare(`UPDATE pipelines SET ...`).run(pipeline);
      for (const event of events) {
        this.db.prepare(`INSERT INTO events ...`).run(event);
      }
    })();
  }
}

// Executor 中统一使用 saveStageTx 替代分散的 save 调用
```

### F11 [MEDIUM]: cancel 状态可达

```typescript
// packages/commands/pipeline/cancel.ts

class PipelineCancelCommand implements ICommand {
  async execute(args: { pipelineId: string }, ctx: CommandContext): Promise<void> {
    const pipeline = await this.store.load(args.pipelineId);

    if (pipeline.status === "completed" || pipeline.status === "cancelled") {
      throw new Error(`Pipeline ${args.pipelineId} already ${pipeline.status}`);
    }

    pipeline.status = "cancelled";
    // F-10: cancel → pending stages 标记 skipped
    for (const stage of pipeline.stages) {
      if (stage.status === "pending") stage.status = "skipped";
    }
    await this.store.saveStageTx(null, pipeline, [
      { kind: "pipeline_cancelled", operator: ctx.operator },
    ]);

    // 执行回滚
    await this.rollback.execute(args.pipelineId);
  }
}

// 京ME 回复 "取消" → human-loop skill 调用 cancel command
```

### F12 [LOW]: Agent resume 继承 SDK 安全配置

```typescript
// Agent resume 修正 — 不直接降级到 CLI，通过 SDK query 传 resume 参数
async resume(sessionId: string, prompt: string, config: AgentTaskInput): Promise<AgentTaskResult> {
  // SDK warm.query 传 --resume 等价配置
  const q = this.warm!.query(prompt, {
    ...this.buildOptions(config), // allowedTools, permissionMode 等安全配置保持
    resume: sessionId,            // SDK resume 参数
  });
  // ...标准消费流程
}
```

---

## 15. R6 终审修正（F-01~F-11）

### F-02 [HIGH]: 多仓库 deploy MR N*N 问题

**问题**：MultiRepoOrchestrator per-repo 调用 deploy skill，如果 deploy skill 内部又循环 pipeline.repos，产出 N*N 个 MR。

**修正**：deploy skill 只处理**单个 repo**。MultiRepoOrchestrator 负责 per-repo 分发，`mergeRepoOutputs` 聚合 mrUrls。

```typescript
// deploy skill — 始终单 repo 逻辑
class DeploySkill implements ISkill {
  async execute(input: SkillInput, ctx: SkillContext): Promise<SkillOutput> {
    // input.repo: 单个 RepoConfig（由 MultiRepoOrchestrator 传入）
    const repo = input.repo;
    await this.build(repo);
    await this.push(repo);
    const mr = await this.createMr(repo, input.mrDescription);
    return { mrUrl: mr.url, mrIid: mr.iid, repo: repo.name };
  }
}

// MultiRepoOrchestrator.execute("deploy", ...) 逐 repo 调用 deploy skill
// mergeRepoOutputs 聚合:
function mergeRepoOutputs(results: RepoResult[]): StageOutput {
  return {
    mrUrls: results.map(r => r.output.mrUrl).filter(Boolean),
    perRepo: results.map(r => ({ repo: r.repo, mrUrl: r.output.mrUrl, mrIid: r.output.mrIid })),
  };
}
```

### F-03 [HIGH]: Recovery 重复 MR — 幂等检查

**问题**：Pipeline 在 deploy stage running 时中断 → recovery 重跑 deploy → 重复创建 MR。

**修正**：deploy skill 开始前检查是否已有同分支 open MR。CodingChannel 新增 `findMr` 方法。

```typescript
// deploy skill — 幂等 MR 创建
async createMrIdempotent(repo: RepoConfig, description: string): Promise<MrResult> {
  // 先查已有 MR
  const existing = await this.codingChannel.findMr({
    projectId: repo.gitlabProjectPath,
    sourceBranch: repo.branch,
    targetBranch: repo.baseBranch,
    state: "opened",
  });
  if (existing) {
    return { url: existing.url, iid: existing.iid, reused: true };
  }
  return this.codingChannel.createMergeRequest({ ... });
}

// CodingChannel 扩展（加入 A11 清单）:
// findMr(query: { projectId, sourceBranch, targetBranch, state }): Promise<MrInfo | null>
// 底层: GET /api/v4/projects/{id}/merge_requests?source_branch=...&target_branch=...&state=opened
```

### F-05 [MEDIUM]: Recovery retry off-by-one

**修正**：Executor while 循环入口增加重试上限检查。

```typescript
// Executor.run() — while 循环入口
while (pipeline.hasNextStage()) {
  const stage = pipeline.currentStage();

  // F-05: 入口检查 — 如果 stage 已是 failed 且重试耗尽，直接退出
  if (stage.status === "failed" && stage.retryCount >= stage.maxRetries) {
    pipeline.status = "failed";
    await this.store.saveStageTx(stage, pipeline, [
      StageFailedFinalEvent(stage, "Max retries exceeded"),
    ]);
    return;
  }

  stage.status = "running";
  // ... 继续执行
}
```

### F-06 [MEDIUM]: 事务模式统一

**修正**：删除 `eventStore.appendTx()`，统一使用 `store.saveStageTx()`。所有 §14 F1 中对 `eventStore.appendTx` 的调用改为 `store.saveStageTx`。

```typescript
// 统一事务方法 — 唯一的写入路径
class SqlitePipelineStore {
  saveStageTx(stage: Stage | null, pipeline: Pipeline, events: PipelineEvent[]): void {
    this.db.transaction(() => {
      if (stage) this.updateStage(stage);
      this.updatePipeline(pipeline);
      for (const event of events) this.insertEvent(pipeline.id, event);
    })();
  }
}

// 所有写入场景统一调用:
await this.store.saveStageTx(stage, pipeline, pipeline.popEvents());
// 而非分散的 store.saveStage() + eventStore.append()
```

### F-07 [MEDIUM]: MR 轮询安全 — 忽略非本 Pipeline 的 MR

**修正**：MR 轮询只查本 Pipeline 产出的 MR URL，URL 存储在 stages.output.mrUrls 中（来自 deploy stage output），不做外部匹配。

```typescript
// pollMergeRequests — 安全边界明确
const mrUrls: string[] = deployOutput?.mrUrls ?? [];
// mrUrls 来自 deploy stage 自己创建的 MR，不是外部查询
// 如果 mrUrls 为空（deploy output 丢失），Pipeline 标记 FAILED 而非无限等待
if (mrUrls.length === 0) {
  pipeline.status = "failed";
  await this.store.saveStageTx(null, pipeline, [
    PipelineFailedEvent("deploy output missing mrUrls"),
  ]);
  continue;
}
```

### F-08 [LOW]: Agent 超时语义 — 总时间 + 空闲

**修正**：PRD R6 #7 改为"总执行时间 >30min 或连续 5min 无输出"。Design Agent 增加空闲超时检测。

```typescript
// Agent dispatch — 双重超时
const IDLE_TIMEOUT_MS = 5 * 60_000; // 5min 无消息 → kill
let lastMessageAt = Date.now();

for await (const msg of q) {
  lastMessageAt = Date.now();
  // ... 处理消息
}

// 在 onProgress 回调中检查空闲
const idleChecker = setInterval(() => {
  if (Date.now() - lastMessageAt > IDLE_TIMEOUT_MS) {
    controller.abort(); // 空闲超时 → kill agent
  }
}, 30_000);
```

### F-09 [LOW]: projectId 命名歧义

**修正**：全局重命名——行云用 `demandProjectId: number`，GitLab 用 `gitlabProjectPath: string`。

```typescript
// DemandMetadata
interface DemandMetadata {
  demandProjectId?: number;    // 行云项目 ID（原 projectId）
  // ...
}

// RepoConfig / workspace output
interface RepoConfig {
  gitlabProjectPath: string;   // GitLab project path（原 projectId）
  // ...
}
```

### F-10 [LOW]: skipped 状态可达

**修正**：cancel 时将未执行的 pending stage 标记为 skipped。

```typescript
// PipelineCancelCommand — 补充 skipped 逻辑
for (const stage of pipeline.stages) {
  if (stage.status === "pending") {
    stage.status = "skipped";
  }
}
// 多仓库熔断中已有 skipped（§10 MultiRepoOrchestrator），这里补上 cancel 路径
```

### F-11 [LOW]: R5 Dashboard/Replay 标注阶段

**修正**：PRD R5 补充阶段标注。

### CodingChannel 扩展清单（更新）

| 能力 | 优先级 | 用途 |
|------|--------|------|
| `getMrStatus(projectPath, iid)` | P1 | MR 合并轮询 |
| `findMr(query)` | P1 | 幂等 MR 创建（F-03） |
| `labels?: string` | P2 | MR 标签 |
| `reviewer_ids?: number[]` | P2 | MR 审批人 |
| `merge_when_pipeline_succeeds?: boolean` | P3 | CI 通过后自动合并 |

---

## 16. 源头修正补遗（F-07~F-10）

### F-07: IssuePolicy 支持多通知角色

```typescript
interface IssuePolicy {
  autoRetry: number;
  notifyRoles: string[];          // 改为数组（原 notifyRole: string）
  escalateAt: string | null;
  retryDelay?: string;
}

// 示例更新：
security_violation: { autoRetry: 0, notifyRoles: ["developer", "security"], escalateAt: "1h" },
// PRD R3 "通知开发者 + 安全团队" 现在可以一次通知多个角色
```

### F-08: Agent idle timer 清理

```typescript
// packages/resources/claude/agent-pool.ts — dispatch 方法修正

async dispatch(input: AgentTaskInput): Promise<AgentTaskResult> {
  let lastMessageAt = Date.now();
  const IDLE_TIMEOUT_MS = 5 * 60_000;

  const idleChecker = setInterval(() => {
    if (Date.now() - lastMessageAt > IDLE_TIMEOUT_MS) {
      controller.abort();
    }
  }, 30_000);

  try {
    for await (const msg of q) {
      lastMessageAt = Date.now();
      // ... 处理消息
    }
    // ... 返回结果
  } finally {
    clearTimeout(timer);       // 总超时
    clearInterval(idleChecker); // F-08: 清理空闲检测器
  }
}
```

### F-09: handleStageError 实现

```typescript
// packages/commands/pipeline/executor.ts

private async handleStageError(pipeline: Pipeline, stage: Stage, error: unknown): Promise<void> {
  // 1. 分类异常
  const issueClass = ExceptionClassifier.classify(error);
  const policy = ISSUE_POLICIES[issueClass];

  // 2. 判断是否可自动重试
  if (policy.autoRetry > 0 && stage.retryCount < policy.autoRetry) {
    stage.status = "failed";
    stage.retryCount++;
    stage.issue = { class: issueClass, message: String(error), retryable: true };
    // 不改 pipeline.status — 留在 running，下次 while 循环重试
    return;
  }

  // 3. 重试耗尽或不可重试 → 需人工
  if (policy.notifyRoles.length > 0) {
    stage.status = "blocked";
    stage.issue = { class: issueClass, message: String(error), retryable: false };
    pipeline.status = "blocked";
    await this.humanLoop.notify(pipeline, stage, issueClass);
  } else {
    // 无通知对象（如 llm_rate_limit 已重试完）→ failed
    stage.status = "failed";
    stage.issue = { class: issueClass, message: String(error), retryable: false };
    pipeline.status = "failed";
  }
}
```

### F-10: Rollback 命令幂等保护

```typescript
// packages/commands/pipeline/rollback.ts — executeRollbackCommand

private async executeRollbackCommand(cmd: RollbackCommand): Promise<void> {
  try {
    switch (cmd.type) {
      case "close_mr": {
        const status = await this.codingChannel.getMrStatus(cmd.params.projectPath, cmd.params.iid);
        if (status === "closed" || status === "merged") return; // 已关闭/已合并，跳过
        await this.codingChannel.closeMr(cmd.params.projectPath, cmd.params.iid);
        break;
      }
      case "delete_branch": {
        // git push origin --delete 是幂等的（分支不存在时 no-op 或报 warning）
        await this.terminal.exec({
          command: `git push origin --delete ${cmd.params.branch} 2>/dev/null || true`,
          timeoutMs: 120_000,
        });
        break;
      }
      case "remove_worktree": {
        // fs.existsSync 先检查
        if (existsSync(cmd.params.path)) {
          await this.terminal.exec({ command: `git worktree remove --force ${cmd.params.path}` });
        }
        break;
      }
    }
  } catch (error) {
    // rollback 单步失败不阻断整体回滚，记录并继续
    this.logger.warn(`Rollback step ${cmd.type} failed: ${error}, continuing`);
  }
}
```
