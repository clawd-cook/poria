# Infrastructure Persistence — 实施计划

## 前置条件

- `@poria/core` 已实现（Wave 1 完成），提供 Pipeline/Stage/PipelineEvent 等类型
- `packages/infrastructure/` 目录已存在但为空
- submodules/poria/packages/auth 作为只读参考（源码迁移）

## 依赖

- `@poria/core`（workspace 内部依赖）
- `better-sqlite3`（SQLite 驱动）
- `nanoid`（通过 @poria/core 传递）

## 实施步骤

### Step 0: 包脚手架

**文件清单**:
- `packages/infrastructure/package.json` — name: `@poria/infrastructure`, deps: `@poria/core`, `better-sqlite3`; devDeps: `typescript`, `vitest`, `@types/better-sqlite3`
- `packages/infrastructure/tsconfig.json` — extends `../../tsconfig.base.json`
- `packages/infrastructure/vitest.config.ts`
- `packages/infrastructure/src/index.ts` — 空桶文件

**验证**: `pnpm install && pnpm --filter @poria/infrastructure run typecheck`

### Step 1: SQLite Schema (`store/schema.ts`)

**文件清单**:
- `src/store/schema.ts`

**内容**:
- `initDatabase(dbPath: string): Database` — 幂等建表函数
- 5 张表: `pipelines`, `stages`, `events`, `audit_log`, `queue`
- 索引: `idx_events_pipeline(pipeline_id, seq)`, `idx_audit_pipeline(pipeline_id)`
- `stages` 含 `agent_session_id TEXT` 列
- `schema_version` 表用于 migration 版本控制
- `CREATE TABLE IF NOT EXISTS` 保证幂等

**验证**: `pnpm --filter @poria/infrastructure run typecheck`

### Step 2: Pipeline CRUD (`store/pipeline-repo.ts`)

**文件清单**:
- `src/store/pipeline-repo.ts`

**内容**:
- `SqlitePipelineStore` 类:
  - `create(pipeline): void` — INSERT pipeline + 7 个初始 stage 行
  - `load(pipelineId): Pipeline` — SELECT pipeline + JOIN stages，JSON 字段反序列化
  - `findByStatus(status): Pipeline[]` — 批量查询
  - `saveStageTx(stage | null, pipeline, events[]): void` — 唯一写入路径，同一 `db.transaction()` 内原子更新 stage + pipeline + 追加 events
- JSON 字段序列化/反序列化: config, input, output, gate_results, issue, rollback
- `saveStageTx` 中 stage 可为 null（仅更新 pipeline + events）

**验证**: typecheck 通过

### Step 3: 事件存储 (`store/event-store.ts`)

**文件清单**:
- `src/store/event-store.ts`

**内容**:
- `EventStore` 类（只读查询，写入通过 saveStageTx）:
  - `queryByPipeline(pipelineId): PipelineEvent[]` — 按 seq 升序
  - `queryByKind(pipelineId, kind): PipelineEvent[]` — 按事件类型过滤
- payload JSON 反序列化为 PipelineEvent union 类型

**验证**: typecheck 通过

### Step 4: 审计日志 (`store/audit-store.ts`)

**文件清单**:
- `src/store/audit-store.ts`

**内容**:
- `AuditStore` 类:
  - `record(entry: AuditEntry): void` — INSERT
  - `queryByPipeline(pipelineId): AuditEntry[]` — 按 created_at 升序
- `AuditAction` 类型: `"git_commit" | "git_push" | "mr_create" | "mr_merge" | "branch_delete"`
- `AuditEntry` 接口: `{ pipelineId, stage?, action, operator, detail }`

**验证**: typecheck 通过

### Step 5: 串行队列 + 单实例锁 (`store/queue.ts`)

**文件清单**:
- `src/store/queue.ts`

**内容**:
- `PipelineQueue` 类:
  - `enqueue(pipelineId, priority = 0): void` — `INSERT OR IGNORE`
  - `dequeue(): string | null` — `db.transaction()` 内 SELECT + DELETE
  - `size(): number`
- `WorkerLock` 类:
  - `acquireLock(lockPath): boolean` — 创建 lockfile + PID，死进程可抢占
  - `releaseLock(): void` — 删除 lockfile
  - `isLocked(): boolean`

**验证**: typecheck 通过

### Step 6: 断点续跑 (`store/recovery.ts`)

**文件清单**:
- `src/store/recovery.ts`

**内容**:
- `PipelineRecovery` 类（依赖注入 store + worktree cleaner + humanLoop）:
  - `recoverAll(): Promise<void>`:
    1. `store.findByStatus("running")` → 遍历
    2. 找最后 completed stage → 计算 resumeFrom
    3. interrupted running stage → failed + retryCount++
    4. dev/cr 阶段 → 调用 worktree cleaner（注入接口）
    5. `store.saveStageTx()` 写入变更
    6. blocked pipelines → `humanLoop.renotify()`（注入接口）

**依赖注入接口**（避免直接依赖 resources/channels）:
```typescript
interface IWorktreeCleaner { cleanDirtyState(repo, pipelineId): Promise<void> }
interface IHumanLoopNotifier { renotify(pipeline, stage): Promise<void> }
```

**验证**: typecheck 通过

### Step 7: 事件归档 (`store/archiver.ts`)

**文件清单**:
- `src/store/archiver.ts`

**内容**:
- `EventArchiver` 类:
  - `archive(retentionDays = 30): Promise<ArchiveResult>` — 查找过期 pipeline → 写 JSONL → 删 SQLite events
  - 归档路径: `workspace/archive/{YYYY-MM}/events-{pipelineId}.jsonl`
  - 月份取自 pipeline.created_at
- `EventReplayService` 类:
  - `replayAll(pipelineId): Promise<PipelineEvent[]>` — 先查 SQLite，无数据查归档 JSONL
  - `findArchiveFile(pipelineId): Promise<string | null>` — 扫描 archive 目录

**验证**: typecheck 通过

### Step 8: Auth 迁移 (`auth/`)

**参考源**: `submodules/poria/packages/auth/src/index.ts`

**文件清单**:
- `src/auth/credentials.ts` — getCredentials / saveCredentials / JacpCredentials 类型
- `src/auth/browser-cookie.ts` — 浏览器 cookie 提取（从 submodules 迁移）

**内容**:
- `getCredentials(): JacpCredentials` — 读取 `~/.poria/auth.json`
- `saveCredentials(creds): void` — 写入凭证文件
- `JacpCredentials` 接口（从 submodules 源码提取字段）
- 迁移后无 `@dj-lib/` 导入

**验证**: typecheck 通过

### Step 9: Credential Guard (`auth/credential-guard.ts`)

**文件清单**:
- `src/auth/credential-guard.ts`

**内容**:
- `CredentialGuard` 类:
  - `ensureValid(credentials): Promise<JacpCredentials>` — 用轻量 API 探测 cookie 有效性
  - 过期 → 尝试自动刷新（`reExtractBrowserCookie()`）
  - 刷新失败 → 抛 `AuthExpiredDuringPipelineError`
- `AuthExpiredDuringPipelineError` 异常类
- API 探测通过依赖注入的 fetch 函数（可 mock）

**验证**: typecheck 通过

### Step 10: Config + Logger + Plugin Loader + Metrics

**文件清单**:
- `src/config/index.ts` — `PoriaConfig` 接口 + `loadConfig()` + 默认值
- `src/logger/index.ts` — `ILogger` 接口 + `createLogger()` JSON 日志实现
- `src/plugin-loader/index.ts` — `IPluginLoader` 接口 + `PluginLoader` 基础实现
- `src/metrics/collector.ts` — `IMetricsCollector` 接口 + `InMemoryMetricsCollector`
- `src/metrics/reporters/stdout.ts` — stdout reporter
- `src/metrics/reporters/file.ts` — file reporter（JSON lines）

**验证**: typecheck 通过

### Step 11: 备份 (`store/backup.ts`)

**文件清单**:
- `src/store/backup.ts`

**内容**:
- `DatabaseBackup` 类:
  - `backup(dbPath, backupDir): Promise<void>` — SQLite Online Backup API
  - 文件名: `poria-YYYYMMDD.db`
  - `cleanup(backupDir, retainDays = 7): void` — 滚动删除旧备份

**验证**: typecheck 通过

### Step 12: 入口文件 + 测试 + 最终验证

**文件清单**:
- `src/index.ts` — 从 store/, auth/, config/, logger/, plugin-loader/, metrics/ 统一 re-export
- `src/store/__tests__/schema.test.ts` — 建表幂等、表结构正确
- `src/store/__tests__/pipeline-repo.test.ts` — CRUD round-trip、saveStageTx 原子性
- `src/store/__tests__/event-store.test.ts` — 查询
- `src/store/__tests__/audit-store.test.ts` — 记录 + 查询
- `src/store/__tests__/queue.test.ts` — 入队幂等、出队原子、空队列
- `src/store/__tests__/recovery.test.ts` — RUNNING pipeline 恢复
- `src/store/__tests__/archiver.test.ts` — 归档 + 回放
- `src/auth/__tests__/credential-guard.test.ts` — 有效/过期/刷新失败

**验证命令**:
```bash
pnpm --filter @poria/infrastructure run typecheck
pnpm --filter @poria/infrastructure run test
```

**验收标准对照**:
- [ ] initDatabase 幂等
- [ ] CRUD round-trip
- [ ] saveStageTx 原子性
- [ ] Recovery 恢复
- [ ] Queue 幂等 + 原子
- [ ] CredentialGuard 过期处理
- [ ] EventArchiver 归档 + 回放
- [ ] 无 @dj-lib/ 依赖
- [ ] typecheck 通过
- [ ] 测试通过
