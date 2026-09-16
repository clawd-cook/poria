use rusqlite::{params, Connection, OpenFlags, Result as SqlResult};
use serde::Serialize;
use std::path::Path;

/// Read-only database wrapper for UI queries.
/// The Node sidecar owns writes via better-sqlite3; Rust reads in WAL mode.
pub struct Database {
    conn: Connection,
}

#[derive(Debug, Serialize, Clone)]
pub struct PipelineSummary {
    pub id: String,
    pub demand_name: String,
    pub demand_code: String,
    pub status: String,
    pub current_stage: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct StageDetail {
    pub name: String,
    pub status: String,
    pub retry_count: i32,
    pub output_summary: Option<String>,
    pub gate_results: Option<String>,
    pub issue: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct PipelineDetail {
    pub id: String,
    pub demand_id: i64,
    pub demand_code: String,
    pub demand_name: String,
    pub status: String,
    pub raw_link: String,
    pub operator: String,
    pub has_regressed: bool,
    pub stages: Vec<StageDetail>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct PipelineEvent {
    pub seq: i64,
    pub kind: String,
    pub payload: String,
    pub created_at: String,
}

impl Database {
    /// Open an existing database in read-only WAL mode.
    /// If the file does not exist, create it with an empty schema so queries
    /// return empty results instead of errors.
    pub fn open(path: &Path) -> SqlResult<Self> {
        if path.exists() {
            let conn = Connection::open_with_flags(
                path,
                OpenFlags::SQLITE_OPEN_READ_ONLY
                    | OpenFlags::SQLITE_OPEN_NO_MUTEX
                    | OpenFlags::SQLITE_OPEN_URI,
            )?;
            conn.pragma_update(None, "journal_mode", "WAL")?;
            Ok(Database { conn })
        } else {
            // Database does not exist yet — create with schema so queries
            // against missing tables do not crash.
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            let conn = Connection::open(path)?;
            conn.pragma_update(None, "journal_mode", "WAL")?;
            conn.pragma_update(None, "foreign_keys", "ON")?;
            conn.execute_batch(INIT_SCHEMA)?;
            Ok(Database { conn })
        }
    }

    /// List all pipelines, ordered by creation time descending.
    /// Computes `current_stage` as the first stage that is not completed/skipped.
    pub fn list_pipelines(&self) -> SqlResult<Vec<PipelineSummary>> {
        if !self.table_exists("pipelines")? {
            return Ok(Vec::new());
        }

        let mut stmt = self.conn.prepare(
            "SELECT p.id, COALESCE(p.demand_name, ''), p.demand_code, p.status,
                    p.created_at, p.updated_at,
                    (SELECT s.name FROM stages s
                     WHERE s.pipeline_id = p.id
                       AND s.status NOT IN ('completed', 'skipped')
                     ORDER BY s.id ASC
                     LIMIT 1) AS current_stage
             FROM pipelines p
             ORDER BY p.created_at DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(PipelineSummary {
                id: row.get(0)?,
                demand_name: row.get(1)?,
                demand_code: row.get(2)?,
                status: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
                current_stage: row.get(6)?,
            })
        })?;

        let mut pipelines = Vec::new();
        for row in rows {
            pipelines.push(row?);
        }
        Ok(pipelines)
    }

    /// Get full pipeline detail including all stages.
    pub fn get_pipeline(&self, id: &str) -> SqlResult<PipelineDetail> {
        if !self.table_exists("pipelines")? {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }

        let pipeline = self.conn.query_row(
            "SELECT id, demand_id, demand_code, COALESCE(demand_name, ''),
                    status, raw_link, operator, has_regressed,
                    created_at, updated_at
             FROM pipelines WHERE id = ?1",
            params![id],
            |row| {
                let has_regressed: i32 = row.get(7)?;
                Ok(PipelineDetail {
                    id: row.get(0)?,
                    demand_id: row.get(1)?,
                    demand_code: row.get(2)?,
                    demand_name: row.get(3)?,
                    status: row.get(4)?,
                    raw_link: row.get(5)?,
                    operator: row.get(6)?,
                    has_regressed: has_regressed != 0,
                    stages: Vec::new(),
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            },
        )?;

        let stages = self.get_stages(id)?;
        Ok(PipelineDetail { stages, ..pipeline })
    }

    /// Get stages for a pipeline, ordered by id.
    fn get_stages(&self, pipeline_id: &str) -> SqlResult<Vec<StageDetail>> {
        if !self.table_exists("stages")? {
            return Ok(Vec::new());
        }

        let mut stmt = self.conn.prepare(
            "SELECT name, status, retry_count,
                    output, gate_results, issue,
                    started_at, completed_at
             FROM stages
             WHERE pipeline_id = ?1
             ORDER BY id ASC",
        )?;

        let rows = stmt.query_map(params![pipeline_id], |row| {
            let output: Option<String> = row.get(3)?;
            // Truncate output to a summary (first 500 chars)
            let output_summary = output.map(|o| {
                if o.len() > 500 {
                    format!("{}...", &o[..500])
                } else {
                    o
                }
            });

            Ok(StageDetail {
                name: row.get(0)?,
                status: row.get(1)?,
                retry_count: row.get(2)?,
                output_summary,
                gate_results: row.get(4)?,
                issue: row.get(5)?,
                started_at: row.get(6)?,
                completed_at: row.get(7)?,
            })
        })?;

        let mut stages = Vec::new();
        for row in rows {
            stages.push(row?);
        }
        Ok(stages)
    }

    /// Get recent events for a pipeline.
    pub fn get_pipeline_events(
        &self,
        pipeline_id: &str,
        limit: i64,
    ) -> SqlResult<Vec<PipelineEvent>> {
        if !self.table_exists("events")? {
            return Ok(Vec::new());
        }

        let mut stmt = self.conn.prepare(
            "SELECT seq, kind, payload, created_at
             FROM events
             WHERE pipeline_id = ?1
             ORDER BY seq DESC
             LIMIT ?2",
        )?;

        let rows = stmt.query_map(params![pipeline_id, limit], |row| {
            Ok(PipelineEvent {
                seq: row.get(0)?,
                kind: row.get(1)?,
                payload: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;

        let mut events = Vec::new();
        for row in rows {
            events.push(row?);
        }
        // Return in chronological order
        events.reverse();
        Ok(events)
    }

    /// Check if a table exists in the database.
    fn table_exists(&self, table_name: &str) -> SqlResult<bool> {
        let count: i32 = self.conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
            params![table_name],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }
}

/// Schema DDL used when creating a new database.
/// Mirrors the Node-side schema in packages/infrastructure/src/store/schema.ts
const INIT_SCHEMA: &str = "
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
        config TEXT,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );

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

    CREATE TABLE IF NOT EXISTS events (
        seq INTEGER PRIMARY KEY AUTOINCREMENT,
        pipeline_id TEXT NOT NULL REFERENCES pipelines(id),
        kind TEXT NOT NULL,
        payload TEXT NOT NULL,
        created_at TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS audit_log (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        pipeline_id TEXT NOT NULL,
        stage TEXT,
        action TEXT NOT NULL,
        operator TEXT NOT NULL,
        detail TEXT,
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
";
