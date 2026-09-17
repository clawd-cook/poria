use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CloneStatus {
    Cloning,
    Ready,
    Failed,
}

impl CloneStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cloning => "cloning",
            Self::Ready => "ready",
            Self::Failed => "failed",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value {
            "ready" => Self::Ready,
            "failed" => Self::Failed,
            _ => Self::Cloning,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredRepo {
    pub id: String,
    pub git_url: String,
    pub normalized_url: String,
    pub scope: String,
    pub name: String,
    pub local_path: String,
    pub clone_status: CloneStatus,
    pub error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub struct RegisteredRepoStore {
    conn: Mutex<Connection>,
}

impl RegisteredRepoStore {
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Mutex::new(conn),
        }
    }

    pub fn insert(&self, repo: &RegisteredRepo) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        if self.find_by_normalized_url_locked(&conn, &repo.normalized_url)? {
            return Err("该 git URL 已登记".into());
        }
        if self
            .find_by_scope_name_locked(&conn, &repo.scope, &repo.name)?
            .is_some()
        {
            return Err(format!("已存在仓库 {}/{}", repo.scope, repo.name));
        }

        conn.execute(
            "INSERT INTO registered_repos (id, git_url, normalized_url, scope, name, local_path, clone_status, error, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![
                repo.id,
                repo.git_url,
                repo.normalized_url,
                repo.scope,
                repo.name,
                repo.local_path,
                repo.clone_status.as_str(),
                repo.error,
                repo.created_at,
                repo.updated_at,
            ],
        )
        .map_err(|e| map_constraint(e, repo))?;
        Ok(())
    }

    pub fn list_all(&self) -> Result<Vec<RegisteredRepo>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT id, git_url, normalized_url, scope, name, local_path, clone_status, error, created_at, updated_at FROM registered_repos ORDER BY scope ASC, name ASC",
            )
            .map_err(|e| e.to_string())?;

        let repos = stmt
            .query_map([], row_to_repo)
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?;
        Ok(repos)
    }

    pub fn load(&self, id: &str) -> Result<Option<RegisteredRepo>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        match conn.query_row(
            "SELECT id, git_url, normalized_url, scope, name, local_path, clone_status, error, created_at, updated_at FROM registered_repos WHERE id = ?1",
            params![id],
            row_to_repo,
        ) {
            Ok(repo) => Ok(Some(repo)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn update_clone_status(
        &self,
        id: &str,
        status: CloneStatus,
        error: Option<String>,
    ) -> Result<RegisteredRepo, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let updated_at = Utc::now().to_rfc3339();
        let changed = conn
            .execute(
                "UPDATE registered_repos SET clone_status = ?1, error = ?2, updated_at = ?3 WHERE id = ?4",
                params![status.as_str(), error, updated_at, id],
            )
            .map_err(|e| e.to_string())?;
        if changed == 0 {
            return Err(format!("仓库不存在: {id}"));
        }
        conn.query_row(
            "SELECT id, git_url, normalized_url, scope, name, local_path, clone_status, error, created_at, updated_at FROM registered_repos WHERE id = ?1",
            params![id],
            row_to_repo,
        )
        .map_err(|e| e.to_string())
    }

    /// Mark in-progress clones as failed so a restarted app can retry.
    pub fn fail_interrupted_clones(&self) -> Result<usize, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let updated_at = Utc::now().to_rfc3339();
        let changed = conn
            .execute(
                "UPDATE registered_repos SET clone_status = ?1, error = ?2, updated_at = ?3 WHERE clone_status = ?4",
                params![
                    CloneStatus::Failed.as_str(),
                    "克隆中断，请重试",
                    updated_at,
                    CloneStatus::Cloning.as_str(),
                ],
            )
            .map_err(|e| e.to_string())?;
        Ok(changed)
    }

    fn find_by_normalized_url_locked(
        &self,
        conn: &Connection,
        normalized_url: &str,
    ) -> Result<bool, String> {
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM registered_repos WHERE normalized_url = ?1",
                params![normalized_url],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        Ok(count > 0)
    }

    fn find_by_scope_name_locked(
        &self,
        conn: &Connection,
        scope: &str,
        name: &str,
    ) -> Result<Option<String>, String> {
        match conn.query_row(
            "SELECT id FROM registered_repos WHERE scope = ?1 AND name = ?2",
            params![scope, name],
            |row| row.get::<_, String>(0),
        ) {
            Ok(id) => Ok(Some(id)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }
}

fn row_to_repo(row: &rusqlite::Row<'_>) -> rusqlite::Result<RegisteredRepo> {
    let status: String = row.get(6)?;
    Ok(RegisteredRepo {
        id: row.get(0)?,
        git_url: row.get(1)?,
        normalized_url: row.get(2)?,
        scope: row.get(3)?,
        name: row.get(4)?,
        local_path: row.get(5)?,
        clone_status: CloneStatus::parse(&status),
        error: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

fn map_constraint(err: rusqlite::Error, repo: &RegisteredRepo) -> String {
    let text = err.to_string();
    if text.contains("normalized_url") {
        "该 git URL 已登记".into()
    } else if text.contains("scope") || text.contains("UNIQUE") {
        format!("已存在仓库 {}/{}", repo.scope, repo.name)
    } else {
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::init_database;

    struct TestCtx {
        store: RegisteredRepoStore,
        _dir: tempfile::TempDir,
    }

    fn test_store() -> TestCtx {
        let dir = tempfile::tempdir().unwrap();
        let conn = init_database(&dir.path().join("repos.db")).unwrap();
        TestCtx {
            store: RegisteredRepoStore::new(conn),
            _dir: dir,
        }
    }

    fn sample_repo(
        id: &str,
        git_url: &str,
        normalized: &str,
        scope: &str,
        name: &str,
    ) -> RegisteredRepo {
        let now = Utc::now().to_rfc3339();
        RegisteredRepo {
            id: id.into(),
            git_url: git_url.into(),
            normalized_url: normalized.into(),
            scope: scope.into(),
            name: name.into(),
            local_path: format!("/tmp/.poria/repos/{scope}/{name}"),
            clone_status: CloneStatus::Cloning,
            error: None,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    #[test]
    fn insert_and_list_registered_repo() {
        let ctx = test_store();
        ctx.store
            .insert(&sample_repo(
                "repo-1",
                "git@coding.jd.com:ls/ls-entrance.git",
                "https://coding.jd.com/ls/ls-entrance",
                "ls",
                "ls-entrance",
            ))
            .unwrap();

        let listed = ctx.store.list_all().unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].scope, "ls");
        assert_eq!(listed[0].name, "ls-entrance");
        assert_eq!(listed[0].clone_status, CloneStatus::Cloning);
    }

    #[test]
    fn insert_rejects_duplicate_normalized_url() {
        let ctx = test_store();
        ctx.store
            .insert(&sample_repo(
                "repo-1",
                "git@coding.jd.com:ls/ls-entrance.git",
                "https://coding.jd.com/ls/ls-entrance",
                "ls",
                "ls-entrance",
            ))
            .unwrap();

        let err = ctx
            .store
            .insert(&sample_repo(
                "repo-2",
                "https://coding.jd.com/ls/ls-entrance.git",
                "https://coding.jd.com/ls/ls-entrance",
                "other",
                "other-name",
            ))
            .unwrap_err();
        assert!(err.contains("已登记"), "unexpected error: {err}");
    }

    #[test]
    fn insert_rejects_duplicate_scope_name() {
        let ctx = test_store();
        ctx.store
            .insert(&sample_repo(
                "repo-1",
                "git@coding.jd.com:ls/ls-entrance.git",
                "https://coding.jd.com/ls/ls-entrance",
                "ls",
                "ls-entrance",
            ))
            .unwrap();

        let err = ctx
            .store
            .insert(&sample_repo(
                "repo-2",
                "git@coding.jd.com:ls/ls-entrance-fork.git",
                "https://coding.jd.com/ls/ls-entrance-fork",
                "ls",
                "ls-entrance",
            ))
            .unwrap_err();
        assert!(err.contains("ls/ls-entrance"), "unexpected error: {err}");
    }

    #[test]
    fn update_clone_status_ready_and_failed() {
        let ctx = test_store();
        ctx.store
            .insert(&sample_repo(
                "repo-1",
                "git@coding.jd.com:ls/ls-entrance.git",
                "https://coding.jd.com/ls/ls-entrance",
                "ls",
                "ls-entrance",
            ))
            .unwrap();

        let ready = ctx
            .store
            .update_clone_status("repo-1", CloneStatus::Ready, None)
            .unwrap();
        assert_eq!(ready.clone_status, CloneStatus::Ready);
        assert!(ready.error.is_none());

        let failed = ctx
            .store
            .update_clone_status("repo-1", CloneStatus::Failed, Some("clone failed".into()))
            .unwrap();
        assert_eq!(failed.clone_status, CloneStatus::Failed);
        assert_eq!(failed.error.as_deref(), Some("clone failed"));
    }

    #[test]
    fn fail_interrupted_clones_leaves_ready_untouched() {
        let ctx = test_store();
        ctx.store
            .insert(&sample_repo(
                "repo-1",
                "git@coding.jd.com:ls/ls-entrance.git",
                "https://coding.jd.com/ls/ls-entrance",
                "ls",
                "ls-entrance",
            ))
            .unwrap();
        ctx.store
            .insert(&sample_repo(
                "repo-2",
                "git@coding.jd.com:ls/other.git",
                "https://coding.jd.com/ls/other",
                "ls",
                "other",
            ))
            .unwrap();
        ctx.store
            .update_clone_status("repo-2", CloneStatus::Ready, None)
            .unwrap();

        let changed = ctx.store.fail_interrupted_clones().unwrap();
        assert_eq!(changed, 1);

        let listed = ctx.store.list_all().unwrap();
        let by_id: std::collections::HashMap<_, _> =
            listed.into_iter().map(|r| (r.id.clone(), r)).collect();
        assert_eq!(by_id["repo-1"].clone_status, CloneStatus::Failed);
        assert_eq!(by_id["repo-1"].error.as_deref(), Some("克隆中断，请重试"));
        assert_eq!(by_id["repo-2"].clone_status, CloneStatus::Ready);
    }
}
