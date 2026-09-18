use rusqlite::{Connection, Result as SqlResult};
use std::path::Path;

const INIT_DDL: &str = "
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

pub fn init_database(path: &Path) -> SqlResult<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.execute_batch(INIT_DDL)?;
    apply_migrations(&conn)?;
    Ok(conn)
}

fn apply_migrations(conn: &Connection) -> SqlResult<()> {
    let current: Option<i32> = conn
        .query_row("SELECT MAX(version) FROM schema_version", [], |row| {
            row.get(0)
        })
        .unwrap_or(None);

    let version = current.unwrap_or(0);

    if version < 1 {
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
            rusqlite::params![1, chrono::Utc::now().to_rfc3339()],
        )?;
        tx.commit()?;
    }

    if version < 2 {
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS registered_repos (
              id TEXT PRIMARY KEY,
              git_url TEXT NOT NULL,
              normalized_url TEXT NOT NULL UNIQUE,
              scope TEXT NOT NULL,
              name TEXT NOT NULL,
              local_path TEXT NOT NULL,
              clone_status TEXT NOT NULL,
              error TEXT,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL,
              UNIQUE (scope, name)
            );
            ",
        )?;
        tx.execute(
            "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
            rusqlite::params![2, chrono::Utc::now().to_rfc3339()],
        )?;
        tx.commit()?;
    }

    if version < 3 {
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(
            "
            ALTER TABLE registered_repos ADD COLUMN default_branch TEXT NOT NULL DEFAULT 'master';
            ALTER TABLE registered_repos ADD COLUMN sync_status TEXT NOT NULL DEFAULT 'idle';
            ALTER TABLE registered_repos ADD COLUMN last_synced_at TEXT;
            ALTER TABLE registered_repos ADD COLUMN sync_error TEXT;
            ",
        )?;
        tx.execute(
            "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
            rusqlite::params![3, chrono::Utc::now().to_rfc3339()],
        )?;
        tx.commit()?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_in_memory() {
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "journal_mode", "WAL").unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        conn.execute_batch(INIT_DDL).unwrap();
        apply_migrations(&conn).unwrap();

        let count: i32 = conn
            .query_row("SELECT COUNT(*) FROM schema_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn test_init_database_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");
        let conn = init_database(&path).unwrap();

        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(tables.contains(&"pipelines".to_string()));
        assert!(tables.contains(&"stages".to_string()));
        assert!(tables.contains(&"events".to_string()));
        assert!(tables.contains(&"audit_log".to_string()));
        assert!(tables.contains(&"queue".to_string()));
        assert!(tables.contains(&"registered_repos".to_string()));

        let version: i32 = conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, 3);

        let columns: Vec<String> = conn
            .prepare("PRAGMA table_info(registered_repos)")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        assert!(columns.contains(&"default_branch".to_string()));
        assert!(columns.contains(&"sync_status".to_string()));
        assert!(columns.contains(&"last_synced_at".to_string()));
        assert!(columns.contains(&"sync_error".to_string()));
    }

    #[test]
    fn schema_v3_defaults_existing_rows_to_master() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(INIT_DDL).unwrap();
        conn.execute_batch(
            "
            CREATE TABLE registered_repos (
              id TEXT PRIMARY KEY,
              git_url TEXT NOT NULL,
              normalized_url TEXT NOT NULL UNIQUE,
              scope TEXT NOT NULL,
              name TEXT NOT NULL,
              local_path TEXT NOT NULL,
              clone_status TEXT NOT NULL,
              error TEXT,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL,
              UNIQUE (scope, name)
            );
            INSERT INTO schema_version (version, applied_at) VALUES (2, '2026-09-17T00:00:00Z');
            INSERT INTO registered_repos (
              id, git_url, normalized_url, scope, name, local_path, clone_status, error, created_at, updated_at
            ) VALUES (
              'repo-1',
              'git@coding.jd.com:ls/ls-entrance.git',
              'https://coding.jd.com/ls/ls-entrance',
              'ls',
              'ls-entrance',
              '/tmp/.poria/repos/ls/ls-entrance',
              'ready',
              NULL,
              '2026-09-17T00:00:00Z',
              '2026-09-17T00:00:00Z'
            );
            ",
        )
        .unwrap();

        apply_migrations(&conn).unwrap();
        let (branch, sync_status): (String, String) = conn
            .query_row(
                "SELECT default_branch, sync_status FROM registered_repos WHERE id = 'repo-1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(branch, "master");
        assert_eq!(sync_status, "idle");
        let version: i32 = conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, 3);
    }
}
