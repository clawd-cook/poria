# Infrastructure Package — Detailed Rust Migration Guide

## 1. Store Layer

### 1.1 `store/schema.ts` — Database Initialization

**Exports:** `initDatabase(dbPath: string): Database.Database`

**Logic:**
1. Opens SQLite at `dbPath` with `better-sqlite3`
2. Enables WAL mode: `PRAGMA journal_mode = WAL`
3. Enables foreign keys: `PRAGMA foreign_keys = ON`
4. Runs `CREATE TABLE IF NOT EXISTS` DDL for 5 tables + 2 indexes (see SQL below)
5. Calls `applyMigrations()` — checks `schema_version` for max version, runs any unapplied migrations in a transaction
6. Current schema version = 1 (initial tables, no further migrations yet)

**SQL DDL (exact):**
```sql
CREATE TABLE IF NOT EXISTS schema_version (
  version INTEGER NOT NULL,
  applied_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS pipelines (
  id TEXT PRIMARY KEY,
  demand_id INTEGER NOT NULL,
  demand_code TEXT NOT NULL,
  demand_name TEXT,
  status TEXT NOT NULL DEFAULT 'created',
  raw_link TEXT NOT NULL,
  operator TEXT NOT NULL,
  has_regressed INTEGER DEFAULT 0,
  config TEXT,                -- JSON string of PipelineConfig
  created_at TEXT NOT NULL,   -- ISO 8601
  updated_at TEXT NOT NULL    -- ISO 8601
);

CREATE TABLE IF NOT EXISTS stages (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  pipeline_id TEXT NOT NULL REFERENCES pipelines(id),
  name TEXT NOT NULL,         -- StageEnum value
  status TEXT NOT NULL DEFAULT 'pending',
  skill_id TEXT,
  retry_count INTEGER DEFAULT 0,
  max_retries INTEGER DEFAULT 3,
  input TEXT,                 -- JSON
  output TEXT,                -- JSON
  gate_results TEXT,          -- JSON
  issue TEXT,                 -- JSON of StageIssue
  rollback TEXT,              -- JSON of RollbackInstruction
  agent_session_id TEXT,
  started_at TEXT,            -- ISO 8601
  completed_at TEXT           -- ISO 8601
);

CREATE TABLE IF NOT EXISTS events (
  seq INTEGER PRIMARY KEY AUTOINCREMENT,
  pipeline_id TEXT NOT NULL REFERENCES pipelines(id),
  kind TEXT NOT NULL,         -- PipelineEvent variant kind
  payload TEXT NOT NULL,      -- Full JSON of the PipelineEvent
  created_at TEXT NOT NULL    -- ISO 8601
);

CREATE TABLE IF NOT EXISTS audit_log (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  pipeline_id TEXT NOT NULL,
  stage TEXT,
  action TEXT NOT NULL,       -- AuditAction value
  operator TEXT NOT NULL,
  detail TEXT,                -- JSON
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS queue (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  pipeline_id TEXT NOT NULL UNIQUE,
  priority INTEGER DEFAULT 0,
  enqueued_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_events_pipeline ON events(pipeline_id, seq);
CREATE INDEX IF NOT EXISTS idx_audit_pipeline ON audit_log(pipeline_id);
```

**Migration logic:**
```
SELECT MAX(version) as version FROM schema_version
-- If currentVersion < 1:
INSERT INTO schema_version (version, applied_at) VALUES (1, <now ISO>)
```

**Dependencies:** `better-sqlite3`
**Rust equivalent:** `rusqlite` with `Connection::open()`, `conn.execute_batch()`, transaction

---

### 1.2 `store/pipeline-repo.ts` — SqlitePipelineStore

**Exports:** `class SqlitePipelineStore`

**Row types (internal):**
- `PipelineRow`: `{ id, demand_id, demand_code, demand_name, status, raw_link, operator, has_regressed, config, created_at, updated_at }`
- `StageRow`: `{ id, pipeline_id, name, status, skill_id, retry_count, max_retries, input, output, gate_results, issue, rollback, agent_session_id, started_at, completed_at }`

**Helpers:**
- `toJson(value)` → `JSON.stringify` or `null`
- `fromJson<T>(value)` → `JSON.parse` or `undefined`
- `toIso(date)` → ISO string or `null`
- `fromIso(value)` → `new Date()` or `undefined`
- `rowToStage(row)` → deserializes JSON fields back to Stage
- `rowToPipeline(pRow, stageRows)` → assembles Pipeline from row + stages

**Prepared Statements (all set up in constructor):**

| Statement | SQL |
|---|---|
| `insertPipeline` | `INSERT INTO pipelines (id, demand_id, demand_code, demand_name, status, raw_link, operator, has_regressed, config, created_at, updated_at) VALUES (...)` |
| `insertStage` | `INSERT INTO stages (pipeline_id, name, status, skill_id, retry_count, max_retries, input, output, gate_results, issue, rollback, agent_session_id, started_at, completed_at) VALUES (...)` |
| `selectPipeline` | `SELECT * FROM pipelines WHERE id = ?` |
| `selectStages` | `SELECT * FROM stages WHERE pipeline_id = ? ORDER BY id ASC` |
| `selectByStatus` | `SELECT * FROM pipelines WHERE status = ?` |
| `updatePipelineStmt` | `UPDATE pipelines SET status=@status, has_regressed=@has_regressed, config=@config, demand_name=@demand_name, updated_at=@updated_at WHERE id=@id` |
| `updateStageStmt` | `UPDATE stages SET status=@status, skill_id=@skill_id, retry_count=@retry_count, max_retries=@max_retries, input=@input, output=@output, gate_results=@gate_results, issue=@issue, rollback=@rollback, agent_session_id=@agent_session_id, started_at=@started_at, completed_at=@completed_at WHERE id=@id` |
| `insertEvent` | `INSERT INTO events (pipeline_id, kind, payload, created_at) VALUES (...)` |

**Methods:**

1. **`create(pipeline: Pipeline): void`** — Transaction:
   - INSERT pipeline row (bool → int for `has_regressed`, config → JSON)
   - INSERT 7 stages in STAGE_ORDER, merging existing stage data if present

2. **`load(pipelineId: string): Pipeline | undefined`**
   - SELECT pipeline + SELECT stages → assemble with `rowToPipeline`

3. **`findByStatus(status: PipelineStatus): Pipeline[]`**
   - SELECT pipelines by status, for each load stages

4. **`saveStageTx(stage | null, pipeline, events[]): void`** — The ONLY write path. Transaction:
   - If stage: UPDATE stage row
   - UPDATE pipeline row
   - For each event: INSERT into events (payload = full JSON of the event)

**State pattern:** Prepared statements cached in constructor, all parameterized.

---

### 1.3 `store/event-store.ts` — EventStore (read-only)

**Exports:** `class EventStore`

**SQL:**
- `SELECT * FROM events WHERE pipeline_id = ? ORDER BY seq ASC`
- `SELECT * FROM events WHERE pipeline_id = ? AND kind = ? ORDER BY seq ASC`

**Methods:**
1. `queryByPipeline(pipelineId)` → `PipelineEvent[]`
2. `queryByKind(pipelineId, kind)` → `PipelineEvent[]`

Both deserialize `payload` JSON → PipelineEvent, restoring `timestamp` from ISO string to Date.

---

### 1.4 `store/audit-store.ts` — AuditStore

**Exports:** `class AuditStore`, `type AuditAction`, `interface AuditEntry`

**Types:**
- `AuditAction = "git_commit" | "git_push" | "mr_create" | "mr_merge" | "branch_delete"`
- `AuditEntry = { pipelineId, stage?, action: AuditAction, operator, detail: Record<string, unknown> }`

**SQL:**
- INSERT: `INSERT INTO audit_log (pipeline_id, stage, action, operator, detail, created_at) VALUES (...)`
- SELECT: `SELECT * FROM audit_log WHERE pipeline_id = ? ORDER BY created_at ASC`

**Methods:**
1. `record(entry)` — INSERT with detail as JSON string
2. `queryByPipeline(pipelineId)` → `(AuditEntry & { id, createdAt })[]`

---

### 1.5 `store/queue.ts` — PipelineQueue + WorkerLock

**Exports:** `class PipelineQueue`, `class WorkerLock`

#### PipelineQueue

**SQL:**
- Enqueue: `INSERT OR IGNORE INTO queue (pipeline_id, priority, enqueued_at) VALUES (?, ?, datetime('now'))`
- Dequeue select: `SELECT pipeline_id FROM queue ORDER BY priority DESC, enqueued_at ASC LIMIT 1`
- Dequeue delete: `DELETE FROM queue WHERE pipeline_id = ?`
- Size: `SELECT COUNT(*) as count FROM queue`

**Methods:**
1. `enqueue(pipelineId, priority=0)` — Idempotent via `INSERT OR IGNORE`
2. `dequeue()` → `string | null` — Atomic SELECT + DELETE in transaction
3. `size()` → `number`

#### WorkerLock

File-based PID lock at `<lockDir>/worker.lock`.

**Dependencies:** `fs`, `path`, `process.pid`, `process.kill(pid, 0)` for liveness check

**Methods:**
1. `acquireLock()` → `bool` — Read PID from file, check `kill(pid, 0)`, write own PID if dead/absent
2. `releaseLock()` — Delete lock file
3. `isLocked()` → `bool` — Check if file exists with live PID

**Rust equivalent:** `std::fs`, `std::process::id()`, check PID with `libc::kill(pid, 0)` or `/proc/<pid>` on linux / `kill(pid, 0)` on macOS.

---

### 1.6 `store/recovery.ts` — PipelineRecovery

**Exports:** `class PipelineRecovery`, `interface IWorktreeCleaner`, `interface IHumanLoopNotifier`, `interface RecoveryResult`

**Injected deps (DI interfaces):**
- `IWorktreeCleaner { cleanDirtyState(repoName, pipelineId): Promise<void> }`
- `IHumanLoopNotifier { renotify(pipeline, stage): Promise<void> }`

**Method: `async recoverAll(): Promise<RecoveryResult>`**
1. Find all RUNNING pipelines via `store.findByStatus("running")`
2. For each: find interrupted stage (status=running), clean worktree for dev/cr stages, mark as failed, increment retryCount, save with events
3. Find all BLOCKED pipelines, re-notify via humanLoop if has blocked stage with issue
4. Returns `{ runningRecovered, blockedRenotified, errors[] }`

---

### 1.7 `store/archiver.ts` — EventArchiver + EventReplayService

**Exports:** `class EventArchiver`, `class EventReplayService`, `interface ArchiveResult`

#### EventArchiver

**Method: `async archive(retentionDays=30): Promise<ArchiveResult>`**
- SELECT terminal pipelines past retention: `SELECT id, created_at FROM pipelines WHERE status IN ('completed','cancelled','failed') AND updated_at < ?`
- For each: SELECT events → write to `<archiveDir>/<YYYY-MM>/events-<pipelineId>.jsonl` (one JSON line per event)
- Bulk DELETE: `DELETE FROM events WHERE pipeline_id IN (SELECT id FROM pipelines WHERE status IN (...) AND updated_at < ?)`

**Dependencies:** `fs`, `path`, `readline`

#### EventReplayService

**Method: `async replayAll(pipelineId): Promise<PipelineEvent[]>`**
- Check live SQLite events first
- Fall back to scanning `<archiveDir>/*/events-<pipelineId>.jsonl`
- Read JSONL via `readline.createInterface(fs.createReadStream(...))`

---

### 1.8 `store/backup.ts` — DatabaseBackup

**Exports:** `class DatabaseBackup`

**Methods:**
1. `async backup(dbPath, backupDir): Promise<string>` — Uses `better-sqlite3`'s `db.backup()` API, writes to `poria-YYYYMMDD.db`
2. `cleanup(backupDir, retainDays=7): number` — Parse dates from `poria-YYYYMMDD.db` filenames, delete old ones

**Rust equivalent:** `rusqlite`'s `backup::Backup` API, or file copy in WAL mode.

---

## 2. Auth Module

### 2.1 `auth/credentials.ts`

**Exports (all functions, no class):**

**Types:**
- `JacpCredentials = { username: string, cookie: string }`
- `AuthStatus = { loggedIn: boolean, username?: string }`
- `StoredAuth` (internal) = JacpCredentials + `updatedAt: string`

**Constants:**
- `USER_DIR_NAME = ".poria"`
- `AUTH_FILE_NAME = "auth.json"`
- `ERP_COOKIE_NAME = "erp_erp"`

**Functions:**
1. `getUserRoot(home?)` → `path.join(home || homedir(), ".poria")`
2. `getAuthFilePath(userRoot?)` → `path.join(userRoot, "auth.json")`
3. `redactCookie(cookie)` → `"xxxx...yyyy (N chars)"`
4. `authHeaders(creds)` → `{ Cookie: creds.cookie }`
5. `parseUsernameFromCookie(cookie)` → parses `erp_erp=<value>` from semicolon-separated cookie string
6. `getCredentials(userRoot?)` → reads `auth.json`, returns `JacpCredentials | undefined`
7. `getStatus(userRoot?)` → `{ loggedIn, username? }`
8. `saveCredentials(creds, userRoot?, options?)` → atomic write: write to `.auth.json.<pid>.tmp`, rename, chmod 0o600
9. `assertSafeAuthRoot(userRoot, home?)` → prevents writing auth into project `.poria/` directories
10. `logout(userRoot?)` → delete `auth.json`

**File I/O pattern:** `writeFileAtomic()` — tmp file → rename → chmod

**Dependencies:** `fs`, `os.homedir()`, `path`, `process.pid`

---

### 2.2 `auth/browser-cookie.ts`

**Exports:** `readBrowserCookies(domains?, browser?)`, `type BrowserCookie`, `DEFAULT_COOKIE_DOMAINS`

**Types:**
- `BrowserCookie = { name?, value?, domain?, path?, secure?, expires? }`
- `DEFAULT_COOKIE_DOMAINS = ["jd.com", "coding.jd.com"]`

**Logic:**
1. Dynamic import of `@rookie-rs/api` (optional dep — itself a Rust crate via napi)
2. Calls `rookie.chrome(domains)` or `rookie.load(domains)` etc.
3. Filters cookies by domain matching, deduplicates by name (keeping most specific domain)
4. Formats as `name=value; name2=value2` header string

**Rust equivalent:** Since `@rookie-rs/api` IS a Rust crate already, the Rust implementation can use the `rookie` crate directly (or skip browser extraction and just read the file).

---

### 2.3 `auth/credential-guard.ts`

**Exports:** `class CredentialGuard`, `class AuthExpiredDuringPipelineError`, `type ApiProbe`

**Types:**
- `ApiProbe = (credentials: JacpCredentials) => Promise<void>` (DI for testability)

**Class: `CredentialGuard`**
- Constructor takes `probe: ApiProbe` and optional `userRoot`
- Method: `async ensureValid(credentials): Promise<JacpCredentials>`
  1. Call `probe(credentials)` — if succeeds, return credentials
  2. If fails with auth-like error (401/unauthorized/expired/cookie/login in message):
     - Try `readBrowserCookies()` to auto-refresh
     - `parseUsernameFromCookie()` to extract ERP
     - `saveCredentials()` to persist
     - `probe(refreshed)` to verify
  3. If refresh also fails → throw `AuthExpiredDuringPipelineError`

**`isAuthExpired(error)`** — checks error message for: 401, unauthorized, auth, cookie, expired, login

---

## 3. Config Module

### 3.1 `config/index.ts`

**Exports:** `loadConfig(overrides?): PoriaConfig`, `interface PoriaConfig`

**PoriaConfig structure:**
```ts
{
  gates: { crScoreThreshold: string, testCoverageThreshold: number, diffSizeThreshold: number },
  timeouts: { gitShort, gitMedium, gitLong, build, agent, agentIdle: number },
  retry: { maxStageRetries, queuePollIntervalMs, mrPollIntervalMs, mrTimeoutMs: number },
  paths: { dbPath, archiveDir, backupDir, logDir: string }
}
```

**Default values:**
- gates: `"B+"`, 80, 500
- timeouts: 30k, 120k, 300k, 600k, 1800k, 300k ms
- retry: 3, 5000, 60000, 86400000 ms
- paths: `workspace/db/poria.db`, `workspace/archive`, `workspace/db/backup`, `workspace/logs`

**Logic:**
1. Try loading `poria.config.json` from CWD
2. Read env vars: `PORIA_CR_SCORE_THRESHOLD`, `PORIA_TEST_COVERAGE_THRESHOLD`, `PORIA_DIFF_SIZE_THRESHOLD`
3. Deep merge: defaults → file → env → programmatic overrides

**Dependencies:** `fs`, `path`, `process.env`

---

## 4. Logger Module

### 4.1 `logger/index.ts`

**Exports:** `createLogger(options?): ILogger`, `class JsonLogger`, `interface ILogger`, `type LogLevel`, `interface LogContext`

**Types:**
- `LogLevel = "debug" | "info" | "warn" | "error"`
- `LogContext = { pipelineId?, stageName?, [key]: unknown }`
- `ILogger` — `debug/info/warn/error(msg, ctx?)` + `child(defaultCtx): ILogger`

**JsonLogger:**
- Outputs one JSON line per log entry to `process.stderr` (default writer)
- Format: `{ timestamp, level, message, ...defaultContext, ...context }`
- Respects min level filtering
- `child()` creates new logger with merged default context

**Rust equivalent:** `tracing` crate with JSON subscriber, or custom implementation.

---

## 5. Metrics Module

### 5.1 `metrics/collector.ts`

**Exports:** `class InMemoryMetricsCollector`, interfaces `IMetricsCollector`, `MetricEntry`, `MetricsSnapshot`

**IMetricsCollector interface (26 methods):**
- Generic: `incrementCounter(name, labels?)`, `recordHistogram(name, value, labels?)`
- Pipeline: `recordPipelineCreated/Completed/Failed`
- Stage: `recordStageComplete/Retry/Failure`
- Agent: `recordAgentExecution`, `recordOutputGuardViolation`
- LLM: `recordLlmTokens`, `recordLlmCost`
- Human: `recordHumanLoop/Response/Escalation`
- Gate: `recordGatePass/Fail`
- Snapshot: `snapshot(): MetricsSnapshot`

**Internal state:**
- `counters: Map<string, number>` — key is `name{label="val",...}`
- `histograms: Map<string, number[]>` — same key format, stores all observations

**Key pattern:** Label encoding in key string: `pipeline_total` or `stage_duration_seconds{stage="dev"}`

**Note:** `recordLlmTokens` and `recordLlmCost` have a quirk: they call `incrementCounter` (+1) then add `tokens - 1` to compensate. Net effect: counter holds the cumulative token/cost value, not a count.

### 5.2 `metrics/reporters/stdout.ts`

**Exports:** `class StdoutReporter`, `interface IMetricsReporter`

Formats snapshot as text lines to `process.stdout`.

### 5.3 `metrics/reporters/file.ts`

**Exports:** `class FileReporter`

Appends JSON snapshot line to file. Creates directory if needed.

---

## 6. Plugin Loader

### 6.1 `plugin-loader/index.ts`

**Exports:** `class PluginLoader`, `interface IPluginLoader`

**Simple in-memory registry (Map<string, unknown>):**
- `register(id, instance)` — store by ID like `"channel:xingyun"`
- `load<T>(id)` → retrieve or throw
- `has(id)` → boolean

No filesystem scanning, no dynamic imports. Pure MVP manual registration.

**Rust equivalent:** `HashMap<String, Box<dyn Any>>` or typed registries per capability.

---

## Summary: Dependencies for Rust Migration

| Node.js API | Rust Equivalent |
|---|---|
| `better-sqlite3` | `rusqlite` (already in workspace) |
| `fs.readFileSync/writeFileSync` | `std::fs::read_to_string/write` |
| `fs.mkdirSync(recursive)` | `std::fs::create_dir_all` |
| `fs.renameSync` | `std::fs::rename` |
| `fs.chmodSync` | `std::os::unix::fs::PermissionsExt` |
| `fs.unlinkSync` | `std::fs::remove_file` |
| `os.homedir()` | `dirs::home_dir()` |
| `path.join/resolve/dirname/basename` | `std::path::PathBuf` methods |
| `process.pid` | `std::process::id()` |
| `process.kill(pid, 0)` | `libc::kill(pid, 0)` / `nix::sys::signal::kill` |
| `process.env` | `std::env::var()` |
| `process.stderr.write` | `eprintln!` / `tracing` |
| `readline.createInterface` | `std::io::BufRead::lines()` |
| `JSON.stringify/parse` | `serde_json::to_string/from_str` |
| `structuredClone` | `.clone()` on `#[derive(Clone)]` |
| `@rookie-rs/api` | `rookie` crate (same Rust library) |
| `better-sqlite3 db.backup()` | `rusqlite::backup::Backup` |

## Key State/Singleton Patterns

1. **SqlitePipelineStore** — all prepared statements cached in struct fields (constructor). Equivalent: `rusqlite` supports prepared statements via `conn.prepare_cached()`.
2. **PipelineQueue** — atomic dequeue uses better-sqlite3 transaction. Equivalent: `conn.transaction()` in rusqlite.
3. **WorkerLock** — PID file with liveness check. Same pattern in Rust.
4. **InMemoryMetricsCollector** — two `Map`s. Equivalent: `HashMap` behind `Mutex`.
5. **PluginLoader** — one `Map`. Equivalent: `HashMap`.
6. **JsonLogger** — writer callback, default context. Can use `tracing` subscriber instead.
