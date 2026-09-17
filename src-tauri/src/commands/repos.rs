use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;
use poria_channels::coding::{normalize_git_url, query_branches, repo_scope_and_name_from_git_url};
use poria_infrastructure::auth::{
    assert_path_under_repos_root, get_credentials, get_registered_repo_path,
};
use poria_infrastructure::store::{CloneStatus, RegisteredRepo, RegisteredRepoStore};
use poria_resources::{git_clone, git_fetch, git_list_branches};
use rand::Rng;
use tauri::Emitter;
use tauri::State;

use crate::AppState;

fn create_repo_id() -> String {
    let bytes: [u8; 8] = rand::thread_rng().gen();
    format!("repo-{}", hex::encode(bytes))
}

fn prepare_clone_dest(path: &Path) -> Result<(), String> {
    if path.exists() {
        std::fs::remove_dir_all(path).map_err(|e| format!("无法清理旧克隆目录: {e}"))?;
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("无法创建仓库目录: {e}"))?;
    }
    Ok(())
}

fn spawn_clone(
    app: tauri::AppHandle,
    repo_store: Arc<RegisteredRepoStore>,
    repo_id: String,
    git_url: String,
    local_path: PathBuf,
) {
    tauri::async_runtime::spawn(async move {
        if let Err(e) = assert_path_under_repos_root(None, &local_path) {
            tracing::error!(repo_id = %repo_id, error = %e, "clone dest escaped repos root");
            if let Ok(updated) =
                repo_store.update_clone_status(&repo_id, CloneStatus::Failed, Some(e))
            {
                let _ = app.emit("repo:updated", &updated);
            }
            return;
        }

        if let Err(e) = prepare_clone_dest(&local_path) {
            tracing::error!(repo_id = %repo_id, error = %e, "failed to prepare clone dest");
            if let Ok(updated) =
                repo_store.update_clone_status(&repo_id, CloneStatus::Failed, Some(e))
            {
                let _ = app.emit("repo:updated", &updated);
            }
            return;
        }

        let result = git_clone(&git_url, &local_path).await;
        let (status, error) = match result {
            Ok(()) => (CloneStatus::Ready, None),
            Err(e) => (CloneStatus::Failed, Some(e.to_string())),
        };

        match repo_store.update_clone_status(&repo_id, status, error) {
            Ok(updated) => {
                let _ = app.emit("repo:updated", &updated);
            }
            Err(e) => {
                tracing::error!(repo_id = %repo_id, error = %e, "failed to persist clone status");
            }
        }
    });
}

/// Register a git URL, persist it, and clone into ~/.poria/repos/{scope}/{name}.
#[tauri::command]
pub async fn register_repo(
    git_url: String,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<RegisteredRepo, String> {
    let trimmed = git_url.trim();
    if trimmed.is_empty() {
        return Err("请输入 git URL".into());
    }

    let (scope, name) = repo_scope_and_name_from_git_url(trimmed).ok_or_else(|| {
        "无法从 git URL 解析仓库路径，请使用 git@host:scope/name.git 格式".to_string()
    })?;

    let local_path = get_registered_repo_path(None, &scope, &name)?;
    let now = Utc::now().to_rfc3339();
    let repo = RegisteredRepo {
        id: create_repo_id(),
        git_url: trimmed.to_string(),
        normalized_url: normalize_git_url(trimmed),
        scope,
        name,
        local_path: local_path.to_string_lossy().into_owned(),
        clone_status: CloneStatus::Cloning,
        error: None,
        created_at: now.clone(),
        updated_at: now,
    };

    state.repo_store.insert(&repo)?;
    app.emit("repo:updated", &repo)
        .map_err(|e| format!("Failed to emit event: {e}"))?;

    spawn_clone(
        app.clone(),
        state.repo_store.clone(),
        repo.id.clone(),
        repo.git_url.clone(),
        local_path,
    );

    Ok(repo)
}

#[tauri::command]
pub async fn list_repos(state: State<'_, AppState>) -> Result<Vec<RegisteredRepo>, String> {
    state.repo_store.list_all()
}

/// Retry a failed clone.
#[tauri::command]
pub async fn retry_clone(
    id: String,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<RegisteredRepo, String> {
    let existing = state
        .repo_store
        .load(&id)?
        .ok_or_else(|| format!("仓库不存在: {id}"))?;

    if existing.clone_status != CloneStatus::Failed {
        return Err("只能重试失败的仓库".into());
    }

    let updated = state
        .repo_store
        .update_clone_status(&id, CloneStatus::Cloning, None)?;
    app.emit("repo:updated", &updated)
        .map_err(|e| format!("Failed to emit event: {e}"))?;

    spawn_clone(
        app.clone(),
        state.repo_store.clone(),
        updated.id.clone(),
        updated.git_url.clone(),
        PathBuf::from(&updated.local_path),
    );

    Ok(updated)
}

/// List backend branches from the managed clone (fetch first); EasyCI is fallback.
#[tauri::command]
pub async fn list_repo_branches(
    id: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let repo = state
        .repo_store
        .load(&id)?
        .ok_or_else(|| format!("仓库不存在: {id}"))?;
    if repo.clone_status != CloneStatus::Ready {
        return Err("仓库尚未克隆完成".into());
    }

    let local_path = PathBuf::from(&repo.local_path);
    assert_path_under_repos_root(None, &local_path)?;

    let _ = git_fetch(&local_path).await;
    let mut branches = git_list_branches(&local_path)
        .await
        .map_err(|e| e.to_string())?;

    if branches.is_empty() {
        if let Some(creds) = get_credentials(None) {
            if !creds.cookie.trim().is_empty() {
                let coding_creds = poria_channels::coding::JacpCredentials {
                    cookie: creds.cookie,
                    username: creds.username,
                };
                if let Ok(remote) = query_branches(&coding_creds, &repo.git_url, "").await {
                    branches = remote
                        .into_iter()
                        .map(|branch| branch.name.trim().to_string())
                        .filter(|name| !name.is_empty() && name != "HEAD")
                        .collect();
                    branches.sort();
                    branches.dedup();
                }
            }
        }
    }

    Ok(branches)
}
