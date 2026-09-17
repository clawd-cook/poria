use chrono::Utc;
use rusqlite::{params, Connection};
use std::sync::Mutex;

use poria_core::pipeline::PipelineEvent;
use poria_core::types::{
    Pipeline, PipelineConfig, PipelineStatus, Stage, StageEnum, StageStatus, STAGE_ORDER,
};

pub struct SqlitePipelineStore {
    conn: Mutex<Connection>,
}

impl SqlitePipelineStore {
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Mutex::new(conn),
        }
    }

    pub fn create(&self, pipeline: &Pipeline) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;

        let config_json = serde_json::to_string(&pipeline.config).unwrap_or_default();
        let created_at = pipeline.created_at.to_rfc3339();
        let updated_at = pipeline.updated_at.to_rfc3339();

        tx.execute(
            "INSERT INTO pipelines (id, demand_id, demand_code, demand_name, status, raw_link, operator, has_regressed, config, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            params![
                pipeline.id,
                pipeline.demand_id,
                pipeline.demand_code,
                pipeline.demand_name,
                serde_json::to_string(&pipeline.status).unwrap().trim_matches('"'),
                pipeline.raw_link,
                pipeline.operator,
                pipeline.has_regressed as i32,
                config_json,
                created_at,
                updated_at,
            ],
        ).map_err(|e| e.to_string())?;

        for stage_enum in STAGE_ORDER {
            let existing = pipeline.stages.iter().find(|s| s.name == *stage_enum);
            let status_str = existing.map(|s| s.status).unwrap_or(StageStatus::Pending);

            tx.execute(
                "INSERT INTO stages (pipeline_id, name, status, skill_id, retry_count, max_retries, input, output, gate_results, issue, rollback, agent_session_id, started_at, completed_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
                params![
                    pipeline.id,
                    serde_json::to_string(stage_enum).unwrap().trim_matches('"'),
                    serde_json::to_string(&status_str).unwrap().trim_matches('"'),
                    existing.and_then(|s| s.skill_id.as_deref()),
                    existing.map(|s| s.retry_count).unwrap_or(0),
                    existing.map(|s| s.max_retries).unwrap_or(3),
                    existing.and_then(|s| s.input.as_ref().map(|v| v.to_string())),
                    existing.and_then(|s| s.output.as_ref().map(|v| v.to_string())),
                    existing.and_then(|s| s.gate_results.as_ref().map(|v| v.to_string())),
                    existing.and_then(|s| s.issue.as_ref().map(|v| serde_json::to_string(v).unwrap())),
                    existing.and_then(|s| s.rollback.as_ref().map(|v| serde_json::to_string(v).unwrap())),
                    existing.and_then(|s| s.agent_session_id.as_deref()),
                    existing.and_then(|s| s.started_at.map(|d| d.to_rfc3339())),
                    existing.and_then(|s| s.completed_at.map(|d| d.to_rfc3339())),
                ],
            ).map_err(|e| e.to_string())?;
        }

        tx.commit().map_err(|e| e.to_string())
    }

    pub fn load(&self, pipeline_id: &str) -> Result<Option<Pipeline>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        let p_row = conn.query_row(
            "SELECT id, demand_id, demand_code, demand_name, status, raw_link, operator, has_regressed, config, created_at, updated_at FROM pipelines WHERE id = ?1",
            params![pipeline_id],
            |row| {
                Ok(PipelineRow {
                    id: row.get(0)?,
                    demand_id: row.get(1)?,
                    demand_code: row.get(2)?,
                    demand_name: row.get(3)?,
                    status: row.get(4)?,
                    raw_link: row.get(5)?,
                    operator: row.get(6)?,
                    has_regressed: row.get(7)?,
                    config: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            },
        );

        let p_row = match p_row {
            Ok(r) => r,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
            Err(e) => return Err(e.to_string()),
        };

        let stages = self.load_stages(&conn, pipeline_id)?;
        Ok(Some(row_to_pipeline(p_row, stages)))
    }

    pub fn find_by_status(&self, status: PipelineStatus) -> Result<Vec<Pipeline>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let status_str = serde_json::to_string(&status)
            .unwrap()
            .trim_matches('"')
            .to_string();

        let mut stmt = conn.prepare(
            "SELECT id, demand_id, demand_code, demand_name, status, raw_link, operator, has_regressed, config, created_at, updated_at FROM pipelines WHERE status = ?1"
        ).map_err(|e| e.to_string())?;

        let rows: Vec<PipelineRow> = stmt
            .query_map(params![status_str], |row| {
                Ok(PipelineRow {
                    id: row.get(0)?,
                    demand_id: row.get(1)?,
                    demand_code: row.get(2)?,
                    demand_name: row.get(3)?,
                    status: row.get(4)?,
                    raw_link: row.get(5)?,
                    operator: row.get(6)?,
                    has_regressed: row.get(7)?,
                    config: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        let mut pipelines = Vec::new();
        for row in rows {
            let stages = self.load_stages(&conn, &row.id)?;
            pipelines.push(row_to_pipeline(row, stages));
        }
        Ok(pipelines)
    }

    pub fn save_stage_tx(
        &self,
        stage: Option<&Stage>,
        pipeline: &Pipeline,
        events: &[PipelineEvent],
    ) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;

        if let Some(stage) = stage {
            if let Some(id) = stage.id {
                tx.execute(
                    "UPDATE stages SET status=?1, skill_id=?2, retry_count=?3, max_retries=?4, input=?5, output=?6, gate_results=?7, issue=?8, rollback=?9, agent_session_id=?10, started_at=?11, completed_at=?12 WHERE id=?13",
                    params![
                        serde_json::to_string(&stage.status).unwrap().trim_matches('"'),
                        stage.skill_id,
                        stage.retry_count,
                        stage.max_retries,
                        stage.input.as_ref().map(|v| v.to_string()),
                        stage.output.as_ref().map(|v| v.to_string()),
                        stage.gate_results.as_ref().map(|v| v.to_string()),
                        stage.issue.as_ref().map(|v| serde_json::to_string(v).unwrap()),
                        stage.rollback.as_ref().map(|v| serde_json::to_string(v).unwrap()),
                        stage.agent_session_id,
                        stage.started_at.map(|d| d.to_rfc3339()),
                        stage.completed_at.map(|d| d.to_rfc3339()),
                        id,
                    ],
                ).map_err(|e| e.to_string())?;
            }
        }

        let config_json = serde_json::to_string(&pipeline.config).unwrap_or_default();
        tx.execute(
            "UPDATE pipelines SET status=?1, has_regressed=?2, config=?3, demand_name=?4, updated_at=?5 WHERE id=?6",
            params![
                serde_json::to_string(&pipeline.status).unwrap().trim_matches('"'),
                pipeline.has_regressed as i32,
                config_json,
                pipeline.demand_name,
                Utc::now().to_rfc3339(),
                pipeline.id,
            ],
        ).map_err(|e| e.to_string())?;

        for event in events {
            let payload = serde_json::to_string(event).unwrap_or_default();
            let kind = extract_event_kind(&payload);
            tx.execute(
                "INSERT INTO events (pipeline_id, kind, payload, created_at) VALUES (?1, ?2, ?3, ?4)",
                params![pipeline.id, kind, payload, Utc::now().to_rfc3339()],
            ).map_err(|e| e.to_string())?;
        }

        tx.commit().map_err(|e| e.to_string())
    }

    pub fn list_all(&self) -> Result<Vec<Pipeline>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare(
            "SELECT id, demand_id, demand_code, demand_name, status, raw_link, operator, has_regressed, config, created_at, updated_at FROM pipelines ORDER BY created_at DESC"
        ).map_err(|e| e.to_string())?;

        let rows: Vec<PipelineRow> = stmt
            .query_map([], |row| {
                Ok(PipelineRow {
                    id: row.get(0)?,
                    demand_id: row.get(1)?,
                    demand_code: row.get(2)?,
                    demand_name: row.get(3)?,
                    status: row.get(4)?,
                    raw_link: row.get(5)?,
                    operator: row.get(6)?,
                    has_regressed: row.get(7)?,
                    config: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        let mut pipelines = Vec::new();
        for row in rows {
            let stages = self.load_stages(&conn, &row.id)?;
            pipelines.push(row_to_pipeline(row, stages));
        }
        Ok(pipelines)
    }

    fn load_stages(&self, conn: &Connection, pipeline_id: &str) -> Result<Vec<Stage>, String> {
        let mut stmt = conn.prepare(
            "SELECT id, pipeline_id, name, status, skill_id, retry_count, max_retries, input, output, gate_results, issue, rollback, agent_session_id, started_at, completed_at FROM stages WHERE pipeline_id = ?1 ORDER BY id ASC"
        ).map_err(|e| e.to_string())?;

        let stages = stmt
            .query_map(params![pipeline_id], |row| {
                let name_str: String = row.get(2)?;
                let status_str: String = row.get(3)?;
                let input_str: Option<String> = row.get(7)?;
                let output_str: Option<String> = row.get(8)?;
                let gate_str: Option<String> = row.get(9)?;
                let issue_str: Option<String> = row.get(10)?;
                let rollback_str: Option<String> = row.get(11)?;
                let started_str: Option<String> = row.get(13)?;
                let completed_str: Option<String> = row.get(14)?;

                Ok(Stage {
                    id: Some(row.get(0)?),
                    pipeline_id: row.get(1)?,
                    name: serde_json::from_str(&format!("\"{}\"", name_str))
                        .unwrap_or(StageEnum::Init),
                    status: serde_json::from_str(&format!("\"{}\"", status_str))
                        .unwrap_or(StageStatus::Pending),
                    skill_id: row.get(4)?,
                    retry_count: row.get(5)?,
                    max_retries: row.get(6)?,
                    input: input_str.and_then(|s| serde_json::from_str(&s).ok()),
                    output: output_str.and_then(|s| serde_json::from_str(&s).ok()),
                    gate_results: gate_str.and_then(|s| serde_json::from_str(&s).ok()),
                    issue: issue_str.and_then(|s| serde_json::from_str(&s).ok()),
                    rollback: rollback_str.and_then(|s| serde_json::from_str(&s).ok()),
                    agent_session_id: row.get(12)?,
                    started_at: started_str.and_then(|s| {
                        chrono::DateTime::parse_from_rfc3339(&s)
                            .ok()
                            .map(|d| d.with_timezone(&chrono::Utc))
                    }),
                    completed_at: completed_str.and_then(|s| {
                        chrono::DateTime::parse_from_rfc3339(&s)
                            .ok()
                            .map(|d| d.with_timezone(&chrono::Utc))
                    }),
                })
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        Ok(stages)
    }
}

struct PipelineRow {
    id: String,
    demand_id: i64,
    demand_code: String,
    demand_name: Option<String>,
    status: String,
    raw_link: String,
    operator: String,
    has_regressed: i32,
    config: Option<String>,
    created_at: String,
    updated_at: String,
}

fn row_to_pipeline(row: PipelineRow, stages: Vec<Stage>) -> Pipeline {
    let config: PipelineConfig = row
        .config
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    let status: PipelineStatus =
        serde_json::from_str(&format!("\"{}\"", row.status)).unwrap_or(PipelineStatus::Created);

    let repos = config.repos.clone();

    Pipeline {
        id: row.id,
        demand_id: row.demand_id,
        demand_code: row.demand_code,
        demand_name: row.demand_name,
        status,
        raw_link: row.raw_link,
        operator: row.operator,
        has_regressed: row.has_regressed != 0,
        config,
        stages,
        repos,
        created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
            .map(|d| d.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now()),
        updated_at: chrono::DateTime::parse_from_rfc3339(&row.updated_at)
            .map(|d| d.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now()),
    }
}

fn extract_event_kind(payload_json: &str) -> String {
    serde_json::from_str::<serde_json::Value>(payload_json)
        .ok()
        .and_then(|v| v.get("kind").and_then(|k| k.as_str()).map(String::from))
        .unwrap_or_else(|| "unknown".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "journal_mode", "WAL").unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        conn.execute_batch(&std::fs::read_to_string("/dev/null").unwrap_or_default())
            .ok();
        // Use init DDL directly
        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS schema_version (version INTEGER NOT NULL, applied_at TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS pipelines (id TEXT PRIMARY KEY, demand_id INTEGER NOT NULL, demand_code TEXT NOT NULL, demand_name TEXT, status TEXT NOT NULL DEFAULT 'created', raw_link TEXT NOT NULL, operator TEXT NOT NULL, has_regressed INTEGER DEFAULT 0, config TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS stages (id INTEGER PRIMARY KEY AUTOINCREMENT, pipeline_id TEXT NOT NULL REFERENCES pipelines(id), name TEXT NOT NULL, status TEXT NOT NULL DEFAULT 'pending', skill_id TEXT, retry_count INTEGER DEFAULT 0, max_retries INTEGER DEFAULT 3, input TEXT, output TEXT, gate_results TEXT, issue TEXT, rollback TEXT, agent_session_id TEXT, started_at TEXT, completed_at TEXT);
            CREATE TABLE IF NOT EXISTS events (seq INTEGER PRIMARY KEY AUTOINCREMENT, pipeline_id TEXT NOT NULL REFERENCES pipelines(id), kind TEXT NOT NULL, payload TEXT NOT NULL, created_at TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS audit_log (id INTEGER PRIMARY KEY AUTOINCREMENT, pipeline_id TEXT NOT NULL, stage TEXT, action TEXT NOT NULL, operator TEXT NOT NULL, detail TEXT, created_at TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS queue (id INTEGER PRIMARY KEY AUTOINCREMENT, pipeline_id TEXT NOT NULL UNIQUE, priority INTEGER DEFAULT 0, enqueued_at TEXT NOT NULL);
        ").unwrap();
        conn
    }

    fn test_pipeline() -> Pipeline {
        Pipeline {
            id: "pl-test-001".into(),
            demand_id: 42,
            demand_code: "REQ-001".into(),
            demand_name: Some("Test Demand".into()),
            status: PipelineStatus::Created,
            raw_link: "https://xingyun.jd.com/demand/42".into(),
            operator: "test_user".into(),
            has_regressed: false,
            config: PipelineConfig::default(),
            stages: vec![],
            repos: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_create_and_load() {
        let conn = test_conn();
        let store = SqlitePipelineStore::new(conn);
        let pipeline = test_pipeline();

        store.create(&pipeline).unwrap();
        let loaded = store.load("pl-test-001").unwrap().unwrap();

        assert_eq!(loaded.id, "pl-test-001");
        assert_eq!(loaded.demand_code, "REQ-001");
        assert_eq!(loaded.stages.len(), 7);
        assert_eq!(loaded.stages[0].name, StageEnum::Init);
    }

    #[test]
    fn test_load_nonexistent() {
        let conn = test_conn();
        let store = SqlitePipelineStore::new(conn);
        let result = store.load("nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_find_by_status() {
        let conn = test_conn();
        let store = SqlitePipelineStore::new(conn);
        store.create(&test_pipeline()).unwrap();

        let found = store.find_by_status(PipelineStatus::Created).unwrap();
        assert_eq!(found.len(), 1);

        let not_found = store.find_by_status(PipelineStatus::Running).unwrap();
        assert!(not_found.is_empty());
    }
}
