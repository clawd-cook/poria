use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    GitCommit,
    GitPush,
    MrCreate,
    MrMerge,
    BranchDelete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub pipeline_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
    pub action: AuditAction,
    pub operator: String,
    pub detail: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    pub id: i64,
    #[serde(flatten)]
    pub entry: AuditEntry,
    pub created_at: String,
}

pub struct AuditStore {
    conn: Mutex<Connection>,
}

impl AuditStore {
    pub fn new(conn: Connection) -> Self {
        Self { conn: Mutex::new(conn) }
    }

    pub fn record(&self, entry: &AuditEntry) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let action_str = serde_json::to_string(&entry.action).unwrap().trim_matches('"').to_string();
        let detail_str = serde_json::to_string(&entry.detail).unwrap_or_default();

        conn.execute(
            "INSERT INTO audit_log (pipeline_id, stage, action, operator, detail, created_at) VALUES (?1,?2,?3,?4,?5,?6)",
            params![entry.pipeline_id, entry.stage, action_str, entry.operator, detail_str, Utc::now().to_rfc3339()],
        ).map_err(|e| e.to_string())?;

        Ok(())
    }

    pub fn query_by_pipeline(&self, pipeline_id: &str) -> Result<Vec<AuditRecord>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare(
            "SELECT id, pipeline_id, stage, action, operator, detail, created_at FROM audit_log WHERE pipeline_id = ?1 ORDER BY created_at ASC"
        ).map_err(|e| e.to_string())?;

        let records = stmt.query_map(params![pipeline_id], |row| {
            let action_str: String = row.get(3)?;
            let detail_str: Option<String> = row.get(5)?;
            Ok(AuditRecord {
                id: row.get(0)?,
                entry: AuditEntry {
                    pipeline_id: row.get(1)?,
                    stage: row.get(2)?,
                    action: serde_json::from_str(&format!("\"{}\"", action_str)).unwrap_or(AuditAction::GitCommit),
                    operator: row.get(4)?,
                    detail: detail_str
                        .and_then(|s| serde_json::from_str(&s).ok())
                        .unwrap_or(serde_json::Value::Null),
                },
                created_at: row.get(6)?,
            })
        }).map_err(|e| e.to_string())?
          .filter_map(|r| r.ok())
          .collect();

        Ok(records)
    }
}
