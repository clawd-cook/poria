import Database from "better-sqlite3";

const CURRENT_SCHEMA_VERSION = 1;

/**
 * Initialize the SQLite database with all required tables.
 * Idempotent: safe to call multiple times (CREATE TABLE IF NOT EXISTS).
 */
export function initDatabase(dbPath: string): Database.Database {
  const db = new Database(dbPath);

  // Enable WAL mode for better concurrent read performance
  db.pragma("journal_mode = WAL");
  db.pragma("foreign_keys = ON");

  db.exec(`
    -- Schema version tracking for migrations
    CREATE TABLE IF NOT EXISTS schema_version (
      version INTEGER NOT NULL,
      applied_at TEXT NOT NULL
    );

    -- Pipeline main table
    CREATE TABLE IF NOT EXISTS pipelines (
      id TEXT PRIMARY KEY,
      demand_id INTEGER NOT NULL,
      demand_code TEXT NOT NULL,
      demand_name TEXT,
      status TEXT NOT NULL DEFAULT 'created',
      raw_link TEXT NOT NULL,
      operator TEXT NOT NULL,
      has_regressed INTEGER DEFAULT 0,
      config TEXT,
      created_at TEXT NOT NULL,
      updated_at TEXT NOT NULL
    );

    -- Stage details
    CREATE TABLE IF NOT EXISTS stages (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      pipeline_id TEXT NOT NULL REFERENCES pipelines(id),
      name TEXT NOT NULL,
      status TEXT NOT NULL DEFAULT 'pending',
      skill_id TEXT,
      retry_count INTEGER DEFAULT 0,
      max_retries INTEGER DEFAULT 3,
      input TEXT,
      output TEXT,
      gate_results TEXT,
      issue TEXT,
      rollback TEXT,
      agent_session_id TEXT,
      started_at TEXT,
      completed_at TEXT
    );

    -- Event log (append-only, used for replay)
    CREATE TABLE IF NOT EXISTS events (
      seq INTEGER PRIMARY KEY AUTOINCREMENT,
      pipeline_id TEXT NOT NULL REFERENCES pipelines(id),
      kind TEXT NOT NULL,
      payload TEXT NOT NULL,
      created_at TEXT NOT NULL
    );

    -- Audit log
    CREATE TABLE IF NOT EXISTS audit_log (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      pipeline_id TEXT NOT NULL,
      stage TEXT,
      action TEXT NOT NULL,
      operator TEXT NOT NULL,
      detail TEXT,
      created_at TEXT NOT NULL
    );

    -- Execution queue (serial model)
    CREATE TABLE IF NOT EXISTS queue (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      pipeline_id TEXT NOT NULL UNIQUE,
      priority INTEGER DEFAULT 0,
      enqueued_at TEXT NOT NULL
    );

    -- Indexes
    CREATE INDEX IF NOT EXISTS idx_events_pipeline ON events(pipeline_id, seq);
    CREATE INDEX IF NOT EXISTS idx_audit_pipeline ON audit_log(pipeline_id);
  `);

  // Apply migrations
  applyMigrations(db);

  return db;
}

function applyMigrations(db: Database.Database): void {
  const row = db.prepare(
    "SELECT MAX(version) as version FROM schema_version",
  ).get() as { version: number | null } | undefined;
  const currentVersion = row?.version ?? 0;

  if (currentVersion < CURRENT_SCHEMA_VERSION) {
    db.transaction(() => {
      // Migration 1: initial schema (tables created above via IF NOT EXISTS)
      if (currentVersion < 1) {
        db.prepare(
          "INSERT INTO schema_version (version, applied_at) VALUES (?, ?)",
        ).run(1, new Date().toISOString());
      }
      // Future migrations go here:
      // if (currentVersion < 2) { ... }
    })();
  }
}

export type { Database };
