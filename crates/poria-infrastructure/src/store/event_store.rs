use chrono::Utc;
use rusqlite::{params, Connection};
use std::sync::Mutex;

use poria_core::pipeline::PipelineEvent;

pub struct EventStore {
    conn: Mutex<Connection>,
}

impl EventStore {
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Mutex::new(conn),
        }
    }

    pub fn query_by_pipeline(&self, pipeline_id: &str) -> Result<Vec<PipelineEvent>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT payload FROM events WHERE pipeline_id = ?1 ORDER BY seq ASC")
            .map_err(|e| e.to_string())?;

        let events: Vec<PipelineEvent> = stmt
            .query_map(params![pipeline_id], |row| {
                let payload: String = row.get(0)?;
                Ok(payload)
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .filter_map(|payload| serde_json::from_str(&payload).ok())
            .collect();

        Ok(events)
    }

    pub fn query_by_kind(
        &self,
        pipeline_id: &str,
        kind: &str,
    ) -> Result<Vec<PipelineEvent>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT payload FROM events WHERE pipeline_id = ?1 AND kind = ?2 ORDER BY seq ASC",
            )
            .map_err(|e| e.to_string())?;

        let events: Vec<PipelineEvent> = stmt
            .query_map(params![pipeline_id, kind], |row| {
                let payload: String = row.get(0)?;
                Ok(payload)
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .filter_map(|payload| serde_json::from_str(&payload).ok())
            .collect();

        Ok(events)
    }

    pub fn append(&self, pipeline_id: &str, event: &PipelineEvent) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let payload = serde_json::to_string(event).map_err(|e| e.to_string())?;
        let kind = serde_json::from_str::<serde_json::Value>(&payload)
            .ok()
            .and_then(|v| v.get("kind").and_then(|k| k.as_str()).map(String::from))
            .unwrap_or_else(|| "unknown".into());

        conn.execute(
            "INSERT INTO events (pipeline_id, kind, payload, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![pipeline_id, kind, payload, Utc::now().to_rfc3339()],
        )
        .map_err(|e| e.to_string())?;

        Ok(())
    }
}
