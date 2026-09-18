use rusqlite::{params, Connection};

use poria_core::types::STAGE_ORDER;

/// Local (SQLite) pipeline observability — not a central service.
#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub struct ObservabilitySummary {
    pub pipeline_total: i64,
    pub pipeline_completed: i64,
    pub pipeline_failed: i64,
    pub pipeline_cancelled: i64,
    pub pipeline_blocked: i64,
    pub pipeline_waiting_merge: i64,
    pub pipeline_running: i64,
    pub success_rate: Option<f64>,
    pub hitl_count: i64,
    pub hitl_pipelines: i64,
    pub hitl_rate: Option<f64>,
    pub cost_usd_total: f64,
    pub stages: Vec<StageObservability>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub struct StageObservability {
    pub stage: String,
    pub completed: i64,
    pub failed: i64,
    pub blocked: i64,
    pub success_rate: Option<f64>,
    pub duration_p50_ms: Option<i64>,
    pub duration_p95_ms: Option<i64>,
    pub cost_usd: f64,
}

pub fn summarize(conn: &Connection) -> Result<ObservabilitySummary, String> {
    let pipeline_total = count_pipelines(conn, None)?;
    let pipeline_completed = count_pipelines(conn, Some("completed"))?;
    let pipeline_failed = count_pipelines(conn, Some("failed"))?;
    let pipeline_cancelled = count_pipelines(conn, Some("cancelled"))?;
    let pipeline_blocked = count_pipelines(conn, Some("blocked"))?;
    let pipeline_waiting_merge = count_pipelines(conn, Some("waiting_merge"))?;
    let pipeline_running = count_pipelines(conn, Some("running"))?;

    let finished = pipeline_completed + pipeline_failed + pipeline_cancelled;
    let success_rate = if finished > 0 {
        Some(pipeline_completed as f64 / finished as f64)
    } else {
        None
    };

    let (hitl_count, hitl_pipelines) = hitl_counts(conn)?;
    let started = pipeline_total.saturating_sub(count_pipelines(conn, Some("created"))?);
    let hitl_rate = if started > 0 {
        Some(hitl_pipelines as f64 / started as f64)
    } else {
        None
    };

    let event_cost = agent_event_cost(conn)?;
    let output_cost = stage_output_cost(conn, None)?;
    let cost_usd_total = if event_cost > 0.0 {
        event_cost
    } else {
        output_cost
    };

    let mut stages = Vec::new();
    for stage in STAGE_ORDER {
        let name = serde_json::to_string(stage)
            .unwrap_or_default()
            .trim_matches('"')
            .to_string();
        stages.push(stage_summary(conn, &name)?);
    }

    Ok(ObservabilitySummary {
        pipeline_total,
        pipeline_completed,
        pipeline_failed,
        pipeline_cancelled,
        pipeline_blocked,
        pipeline_waiting_merge,
        pipeline_running,
        success_rate,
        hitl_count,
        hitl_pipelines,
        hitl_rate,
        cost_usd_total,
        stages,
    })
}

fn count_pipelines(conn: &Connection, status: Option<&str>) -> Result<i64, String> {
    if let Some(status) = status {
        conn.query_row(
            "SELECT COUNT(*) FROM pipelines WHERE status = ?1",
            params![status],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())
    } else {
        conn.query_row("SELECT COUNT(*) FROM pipelines", [], |row| row.get(0))
            .map_err(|e| e.to_string())
    }
}

fn hitl_counts(conn: &Connection) -> Result<(i64, i64), String> {
    let event_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM events WHERE kind IN ('stage_blocked', 'human_assist_requested')",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);
    let event_pipelines: i64 = conn
        .query_row(
            "SELECT COUNT(DISTINCT pipeline_id) FROM events WHERE kind IN ('stage_blocked', 'human_assist_requested')",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if event_count > 0 {
        return Ok((event_count, event_pipelines));
    }
    let blocked: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM stages WHERE status = 'blocked'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    let blocked_pipelines: i64 = conn
        .query_row(
            "SELECT COUNT(DISTINCT pipeline_id) FROM stages WHERE status = 'blocked'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    Ok((blocked, blocked_pipelines))
}

fn agent_event_cost(conn: &Connection) -> Result<f64, String> {
    let mut stmt = conn
        .prepare("SELECT payload FROM events WHERE kind = 'agent_completed'")
        .map_err(|e| e.to_string())?;
    let payloads = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    let mut total = 0.0;
    for payload in payloads.flatten() {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&payload) {
            if let Some(cost) = value.get("cost_usd").and_then(|v| v.as_f64()) {
                total += cost;
            }
        }
    }
    Ok(total)
}

fn stage_output_cost(conn: &Connection, stage: Option<&str>) -> Result<f64, String> {
    let mut outputs: Vec<String> = Vec::new();
    if let Some(stage) = stage {
        let mut stmt = conn
            .prepare("SELECT output FROM stages WHERE output IS NOT NULL AND name = ?1")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![stage], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        outputs.extend(rows.flatten());
    } else {
        let mut stmt = conn
            .prepare("SELECT output FROM stages WHERE output IS NOT NULL")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        outputs.extend(rows.flatten());
    }
    let mut total = 0.0;
    for output in outputs {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&output) {
            if let Some(cost) = value.get("costUsd").and_then(|v| v.as_f64()) {
                total += cost;
            }
        }
    }
    Ok(total)
}

fn stage_summary(conn: &Connection, name: &str) -> Result<StageObservability, String> {
    let completed = count_stages(conn, name, "completed")?;
    let failed = count_stages(conn, name, "failed")?;
    let blocked = count_stages(conn, name, "blocked")?;
    let decided = completed + failed + blocked;
    let success_rate = if decided > 0 {
        Some(completed as f64 / decided as f64)
    } else {
        None
    };
    let mut durations = stage_durations_ms(conn, name)?;
    durations.sort_unstable();
    let event_cost = agent_event_cost_for_stage(conn, name)?;
    let output_cost = stage_output_cost(conn, Some(name))?;
    Ok(StageObservability {
        stage: name.to_string(),
        completed,
        failed,
        blocked,
        success_rate,
        duration_p50_ms: percentile_ms(&durations, 0.50),
        duration_p95_ms: percentile_ms(&durations, 0.95),
        cost_usd: if event_cost > 0.0 {
            event_cost
        } else {
            output_cost
        },
    })
}

fn count_stages(conn: &Connection, name: &str, status: &str) -> Result<i64, String> {
    conn.query_row(
        "SELECT COUNT(*) FROM stages WHERE name = ?1 AND status = ?2",
        params![name, status],
        |row| row.get(0),
    )
    .map_err(|e| e.to_string())
}

fn stage_durations_ms(conn: &Connection, name: &str) -> Result<Vec<i64>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT started_at, completed_at FROM stages WHERE name = ?1 AND started_at IS NOT NULL AND completed_at IS NOT NULL",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![name], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for row in rows.flatten() {
        if let (Ok(start), Ok(end)) = (
            chrono::DateTime::parse_from_rfc3339(&row.0),
            chrono::DateTime::parse_from_rfc3339(&row.1),
        ) {
            let ms = (end - start).num_milliseconds();
            if ms >= 0 {
                out.push(ms);
            }
        }
    }
    Ok(out)
}

fn agent_event_cost_for_stage(conn: &Connection, stage: &str) -> Result<f64, String> {
    let mut stmt = conn
        .prepare("SELECT payload FROM events WHERE kind = 'agent_completed'")
        .map_err(|e| e.to_string())?;
    let payloads = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    let mut total = 0.0;
    for payload in payloads.flatten() {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&payload) {
            let matches = value
                .get("stage")
                .and_then(|v| v.as_str())
                .is_some_and(|s| s == stage);
            if matches {
                if let Some(cost) = value.get("cost_usd").and_then(|v| v.as_f64()) {
                    total += cost;
                }
            }
        }
    }
    Ok(total)
}

pub(crate) fn percentile_ms(sorted: &[i64], p: f64) -> Option<i64> {
    if sorted.is_empty() {
        return None;
    }
    let idx = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    sorted.get(idx.min(sorted.len() - 1)).copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE pipelines (id TEXT PRIMARY KEY, demand_id INTEGER NOT NULL, demand_code TEXT NOT NULL, demand_name TEXT, status TEXT NOT NULL, raw_link TEXT NOT NULL, operator TEXT NOT NULL, has_regressed INTEGER DEFAULT 0, config TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL);
            CREATE TABLE stages (id INTEGER PRIMARY KEY AUTOINCREMENT, pipeline_id TEXT NOT NULL, name TEXT NOT NULL, status TEXT NOT NULL, skill_id TEXT, retry_count INTEGER DEFAULT 0, max_retries INTEGER DEFAULT 3, input TEXT, output TEXT, gate_results TEXT, issue TEXT, rollback TEXT, agent_session_id TEXT, started_at TEXT, completed_at TEXT);
            CREATE TABLE events (seq INTEGER PRIMARY KEY AUTOINCREMENT, pipeline_id TEXT NOT NULL, kind TEXT NOT NULL, payload TEXT NOT NULL, created_at TEXT NOT NULL);
            ",
        )
        .unwrap();
        conn
    }

    #[test]
    fn percentile_picks_p50_and_p95() {
        let values = [10, 20, 30, 40, 100];
        assert_eq!(percentile_ms(&values, 0.50), Some(30));
        assert_eq!(percentile_ms(&values, 0.95), Some(100));
        assert_eq!(percentile_ms(&[], 0.50), None);
    }

    #[test]
    fn summarize_reads_status_duration_cost_and_hitl() {
        let conn = test_conn();
        conn.execute(
            "INSERT INTO pipelines (id, demand_id, demand_code, demand_name, status, raw_link, operator, has_regressed, config, created_at, updated_at) VALUES
             ('p1', 1, 'A', 'a', 'completed', 'u', 'op', 0, '{}', '2026-01-01T00:00:00Z', '2026-01-01T01:00:00Z'),
             ('p2', 2, 'B', 'b', 'failed', 'u', 'op', 0, '{}', '2026-01-01T00:00:00Z', '2026-01-01T01:00:00Z'),
             ('p3', 3, 'C', 'c', 'blocked', 'u', 'op', 0, '{}', '2026-01-01T00:00:00Z', '2026-01-01T01:00:00Z')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO stages (pipeline_id, name, status, output, started_at, completed_at) VALUES
             ('p1', 'dev', 'completed', '{\"costUsd\": 1.5}', '2026-01-01T00:00:00Z', '2026-01-01T00:10:00Z'),
             ('p2', 'dev', 'failed', '{\"costUsd\": 0.5}', '2026-01-01T00:00:00Z', '2026-01-01T00:02:00Z'),
             ('p3', 'design', 'blocked', NULL, '2026-01-01T00:00:00Z', NULL)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO events (pipeline_id, kind, payload, created_at) VALUES
             ('p3', 'stage_blocked', '{\"kind\":\"stage_blocked\"}', '2026-01-01T00:05:00Z')",
            [],
        )
        .unwrap();

        let summary = summarize(&conn).unwrap();
        assert_eq!(summary.pipeline_total, 3);
        assert_eq!(summary.pipeline_completed, 1);
        assert_eq!(summary.pipeline_failed, 1);
        assert_eq!(summary.success_rate, Some(0.5));
        assert_eq!(summary.hitl_count, 1);
        assert_eq!(summary.hitl_pipelines, 1);
        assert!((summary.cost_usd_total - 2.0).abs() < f64::EPSILON);

        let dev = summary.stages.iter().find(|s| s.stage == "dev").unwrap();
        assert_eq!(dev.completed, 1);
        assert_eq!(dev.failed, 1);
        assert_eq!(dev.duration_p50_ms, Some(600_000));
        assert!((dev.cost_usd - 2.0).abs() < f64::EPSILON);
    }
}
