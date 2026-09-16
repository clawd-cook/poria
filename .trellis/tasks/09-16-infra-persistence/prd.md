# Infrastructure Persistence — PRD

> 子任务，隶属 parent: `poria-platform-arch`

## Goal

为 Poria 平台提供**基建层**（`packages/infrastructure/`），涵盖 SQLite 持久化、事件存储、串行队列、SSO 鉴权、全局配置、结构化日志、能力包加载和监控指标采集。所有上层模块（Pipeline Executor、Channels、Resources、Commands）依赖本层提供的存储和基础设施原语。

---

## 依赖

- **core-foundation**（`packages/core/`）：导入 Pipeline/Stage/PipelineEvent/GateResult/IssueClass/RepoConfig 等类型和契约接口。

---

## 核心需求

### R1: SQLite Schema（`store/schema.ts`）

- 使用 `better-sqlite3` 创建以下表（参考 Design §7）：
  - `pipelines` — Pipeline 主表（id, demand_id, demand_code, demand_name, status, raw_link, operator, has_regressed, config JSON, created_at, updated_at）
  - `stages` — Stage 明细（id, pipeline_id FK, name, status, skill_id, retry_count, max_retries, input JSON, output JSON, gate_results JSON, issue JSON, rollback JSON, agent_session_id, started_at, completed_at）
  - `events` — 事件日志（seq, pipeline_id FK, kind, payload JSON, created_at）+ 索引 `idx_events_pipeline(pipeline_id, seq)`
  - `audit_log` — 审计日志（id, pipeline_id, stage, action, operator, detail JSON, created_at）+ 索引 `idx_audit_pipeline(pipeline_id)`
  - `queue` — 执行队列（id, pipeline_id UNIQUE, priority, enqueued_at）
- 提供 `initDatabase(dbPath: string): Database` 函数，幂等建表（`CREATE TABLE IF NOT EXISTS`）
- 提供 migration 机制（版本号表 `schema_version`），支持后续 schema 升级

### R2: Pipeline CRUD + 事务写入（`store/pipeline-repo.ts`）

- `SqlitePipelineStore` 类，提供：
  - `create(pipeline): void` — 插入 Pipeline + 初始 7 个 Stage 行
  - `load(pipelineId): Pipeline` — 加载 Pipeline + 关联 Stages
  - `findByStatus(status): Pipeline[]` — 按状态批量查询
  - `saveStageTx(stage | null, pipeline, events[]): void` — **唯一写入路径**：在同一 SQLite 事务中原子更新 stage + pipeline + 追加 events（Design §14 F10）
  - `getCredentials(): JacpCredentials` — 读取本地凭证
- `saveStageTx` 部分失败时**全部回滚**，不留脏数据

### R3: 事件存储（`store/event-store.ts`）

- 事件追加写入 `events` 表（通过 `saveStageTx` 统一路径，不单独暴露 `append`）
- 查询接口：
  - `queryByPipeline(pipelineId): PipelineEvent[]` — 按 seq 升序
  - `queryByKind(pipelineId, kind): PipelineEvent[]` — 按事件类型过滤

### R4: 审计日志（`store/audit-store.ts`）

- `AuditStore.record(entry: AuditEntry): void` — 插入审计记录
- `AuditStore.queryByPipeline(pipelineId): AuditEntry[]` — 查询指定 Pipeline 的全部审计记录
- AuditEntry 结构：`{ pipelineId, stage?, action, operator, detail }`
- action 枚举：`git_commit | git_push | mr_create | mr_merge | branch_delete`

### R5: 断点续跑恢复（`store/recovery.ts`）

- `PipelineRecovery.recoverAll()` — 系统启动时调用：
  1. 查找所有 `status = "running"` 的 Pipeline
  2. 找到最后一个 `completed` Stage，计算 `resumeFrom` 索引
  3. 被中断的 `running` Stage → 标记 `failed` + `retryCount++`
  4. dev/cr 阶段中断 → 调用 worktree 清理未提交变更（通过依赖注入）
  5. 统一事务写入状态变更
  6. 恢复 `blocked` Pipeline：重新发送京ME 提醒（通过依赖注入 humanLoop）

### R6: 串行队列 + 单实例锁（`store/queue.ts`）

- `PipelineQueue` 类：
  - `enqueue(pipelineId, priority?): void` — 幂等入队（`INSERT OR IGNORE`）
  - `dequeue(): string | null` — 原子出队（事务内 SELECT + DELETE）
  - 出队优先级：`priority DESC, enqueued_at ASC`
- `PipelineWorker` 文件锁：
  - 锁文件路径：`workspace/db/worker.lock`
  - 写入 PID，若已存在检查 PID 是否活跃，死进程可抢占
  - `acquireLock(): boolean` / `releaseLock(): void`

### R7: 事件归档（`store/archiver.ts`）

- `EventArchiver.archive(retentionDays = 30): ArchiveResult`
  - 查找已完成（completed/cancelled/failed）且超过保留期的 Pipeline
  - 按 Pipeline ID 归档：`workspace/archive/{YYYY-MM}/events-{pipelineId}.jsonl`
  - 月份取自 `pipeline.created_at`（Design §14 F7）
  - 从 `events` 表删除已归档事件（pipelines/stages 元数据保留）
- `EventReplayService.replayAll(pipelineId): PipelineEvent[]`
  - 先查 SQLite `events` 表
  - SQLite 无数据 → 查归档 JSONL 文件
  - 两处都无 → 返回空数组

### R8: SSO 鉴权（`auth/`）

- 从 `@dj-lib/poria-auth` 迁移 SSO cookie 提取逻辑到 `packages/infrastructure/auth/`
- `getCredentials(): JacpCredentials` — 读取 `~/.poria/auth.json`
- `saveCredentials(creds): void` — 写入凭证文件
- `CredentialGuard.ensureValid(credentials): JacpCredentials`（Design A3）：
  - 用轻量 API 探测 cookie 有效性（`/openapi/v3/user/info`）
  - 过期 → 尝试自动刷新（重新读浏览器 cookie）
  - 刷新失败 → 抛 `AuthExpiredDuringPipelineError`

### R9: 全局配置（`config/`）

- 配置项包括：门禁阈值（CR 评分、测试覆盖率、变更量）、超时档位（GIT_SHORT/GIT_MEDIUM/GIT_LONG/BUILD/AGENT）、重试次数、队列轮询间隔
- 支持从文件/环境变量加载，提供类型安全的访问接口
- 默认值符合 Design 定义（CR ≥ B+、覆盖率 ≥ 80%、diff ≤ 500 行、Agent 30min 超时）

### R10: 结构化日志（`logger/`）

- JSON 格式日志输出（timestamp, level, module, message, context）
- 支持 debug/info/warn/error 级别
- Pipeline/Stage 上下文自动注入（pipelineId, stageName）

### R11: 能力包加载器（`plugin-loader/`）

- `IPluginLoader.load<T>(id: string): Promise<T>` — 按 ID 加载能力包实例
- ID 格式：`channel:xingyun`、`resource:terminal`、`skill:gen-code`
- 发现机制：扫描 `packages/` 目录结构，按约定注册

### R12: 监控指标（`metrics/`）

- `IMetricsCollector` 接口（Design §9）：
  - Pipeline 维度：pipeline_total, pipeline_completed, pipeline_failed, pipeline_duration_seconds
  - Stage 维度：stage_duration_seconds, stage_retry_total, stage_failure_total
  - Agent 维度：agent_execution_seconds, agent_output_guard_violations
  - LLM 维度：llm_tokens_total, llm_cost_total
  - 人工介入维度：human_loop_total, human_loop_response_seconds
  - 门禁维度：gate_pass_total, gate_fail_total
- Reporter 适配器：stdout（默认）、file（JSON lines）

### R13: SQLite 备份（`store/backup.ts`）

- 每日自动备份：`workspace/db/backup/poria-YYYYMMDD.db`
- 使用 SQLite Online Backup API（不阻塞写入）
- 保留最近 7 天备份，滚动删除

---

## 约束

- **C1**: 使用 `better-sqlite3`（同步 API，事务性好，Node.js 24.20.0 兼容）
- **C2**: 数据库文件位于 `workspace/db/poria.db`（gitignored）
- **C3**: 所有写入通过 `saveStageTx` 统一事务路径，不允许分散的 `UPDATE` 调用
- **C4**: 迁移自 `@dj-lib/poria-auth` 为源码复制 + 重构，不依赖外部 `@dj-lib/` 包
- **C5**: 日志、指标等模块通过接口定义，不强依赖具体实现（可替换）

---

## Acceptance Criteria

- [ ] `initDatabase()` 幂等建表成功，包含 pipelines/stages/events/audit_log/queue 五张表 + agent_session_id 列
- [ ] Pipeline CRUD round-trip：create → load → 数据一致
- [ ] `saveStageTx` 原子性：模拟中途异常后，stage/pipeline/events 均未写入
- [ ] Recovery：设置 Pipeline 为 RUNNING + 一个 running Stage → `recoverAll()` → Stage 变为 failed + retryCount++，Pipeline 从下一个 Stage 恢复
- [ ] Queue：`enqueue` 幂等（重复 pipelineId 不报错），`dequeue` 原子（并发调用不重复消费），空队列返回 null
- [ ] CredentialGuard：mock 过期 API → 尝试刷新 → 刷新失败抛 AuthExpiredDuringPipelineError
- [ ] EventArchiver：归档后 JSONL 文件存在于 `workspace/archive/{YYYY-MM}/events-{pipelineId}.jsonl`，SQLite events 表中对应记录已删除
- [ ] EventReplayService：SQLite 有数据 → 返回；SQLite 无数据但归档有 → 从 JSONL 返回
- [ ] 全部模块 TypeScript 编译通过，导出接口符合 core-foundation 契约
- [ ] 单元测试覆盖率 ≥ 80%
