use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub struct PipelineQueue {
    conn: Mutex<Connection>,
}

impl PipelineQueue {
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Mutex::new(conn),
        }
    }

    pub fn enqueue(&self, pipeline_id: &str, priority: i32) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR IGNORE INTO queue (pipeline_id, priority, enqueued_at) VALUES (?1, ?2, datetime('now'))",
            params![pipeline_id, priority],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn dequeue(&self) -> Result<Option<String>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;

        let pipeline_id: Option<String> = tx
            .query_row(
                "SELECT pipeline_id FROM queue ORDER BY priority DESC, enqueued_at ASC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .ok();

        if let Some(ref id) = pipeline_id {
            tx.execute("DELETE FROM queue WHERE pipeline_id = ?1", params![id])
                .map_err(|e| e.to_string())?;
        }

        tx.commit().map_err(|e| e.to_string())?;
        Ok(pipeline_id)
    }

    pub fn size(&self) -> Result<i64, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.query_row("SELECT COUNT(*) FROM queue", [], |row| row.get(0))
            .map_err(|e| e.to_string())
    }

    pub fn contains(&self, pipeline_id: &str) -> Result<bool, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM queue WHERE pipeline_id = ?1",
                params![pipeline_id],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        Ok(count > 0)
    }

    pub fn remove(&self, pipeline_id: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "DELETE FROM queue WHERE pipeline_id = ?1",
            params![pipeline_id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }
}

pub struct WorkerLock {
    lock_path: PathBuf,
}

impl WorkerLock {
    pub fn new(lock_dir: &Path) -> Self {
        Self {
            lock_path: lock_dir.join("worker.lock"),
        }
    }

    pub fn acquire(&self) -> Result<bool, String> {
        if self.lock_path.exists() {
            let content = std::fs::read_to_string(&self.lock_path).unwrap_or_default();
            if let Ok(pid) = content.trim().parse::<u32>() {
                if is_process_alive(pid) {
                    return Ok(false);
                }
            }
        }

        if let Some(parent) = self.lock_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(&self.lock_path, std::process::id().to_string())
            .map_err(|e| e.to_string())?;
        Ok(true)
    }

    pub fn release(&self) -> Result<(), String> {
        if self.lock_path.exists() {
            std::fs::remove_file(&self.lock_path).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn is_locked(&self) -> bool {
        if !self.lock_path.exists() {
            return false;
        }
        let content = std::fs::read_to_string(&self.lock_path).unwrap_or_default();
        content
            .trim()
            .parse::<u32>()
            .map(is_process_alive)
            .unwrap_or(false)
    }
}

#[cfg(unix)]
fn is_process_alive(pid: u32) -> bool {
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

#[cfg(not(unix))]
fn is_process_alive(_pid: u32) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn test_queue() -> PipelineQueue {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE queue (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                pipeline_id TEXT NOT NULL UNIQUE,
                priority INTEGER DEFAULT 0,
                enqueued_at TEXT NOT NULL
            );",
        )
        .unwrap();
        PipelineQueue::new(conn)
    }

    #[test]
    fn enqueue_is_idempotent_and_dequeues_by_priority() {
        let queue = test_queue();
        queue.enqueue("low", 0).unwrap();
        queue.enqueue("high", 10).unwrap();
        queue.enqueue("low", 99).unwrap();
        assert!(queue.contains("low").unwrap());
        assert_eq!(queue.size().unwrap(), 2);
        assert_eq!(queue.dequeue().unwrap().as_deref(), Some("high"));
        assert_eq!(queue.dequeue().unwrap().as_deref(), Some("low"));
        assert_eq!(queue.dequeue().unwrap(), None);
    }

    #[test]
    fn remove_drops_queued_id() {
        let queue = test_queue();
        queue.enqueue("p1", 0).unwrap();
        queue.enqueue("p2", 0).unwrap();
        queue.remove("p1").unwrap();
        assert!(!queue.contains("p1").unwrap());
        assert_eq!(queue.dequeue().unwrap().as_deref(), Some("p2"));
    }

    #[test]
    fn worker_lock_acquire_and_release() {
        let dir = tempfile::tempdir().unwrap();
        let lock = WorkerLock::new(dir.path());
        assert!(lock.acquire().unwrap());
        assert!(lock.is_locked());
        lock.release().unwrap();
        assert!(!lock.is_locked());
        assert!(lock.acquire().unwrap());
        lock.release().unwrap();
    }
}
