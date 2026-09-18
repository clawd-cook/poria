use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

const USER_DIR_NAME: &str = ".poria";
const AUTH_FILE_NAME: &str = "auth.json";
const CONFIG_FILE_NAME: &str = "config.json";
const REPOS_DIR_NAME: &str = "repos";
const PROJECTS_DIR_NAME: &str = "projects";
const ERP_COOKIE_NAME: &str = "erp_erp";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JacpCredentials {
    pub username: String,
    pub cookie: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthStatus {
    pub logged_in: bool,
    pub username: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredAuth {
    #[serde(flatten)]
    credentials: JacpCredentials,
    updated_at: String,
}

pub fn get_user_root(home: Option<&Path>) -> PathBuf {
    let base = home
        .map(PathBuf::from)
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")));
    base.join(USER_DIR_NAME)
}

pub fn get_auth_file_path(user_root: Option<&Path>) -> PathBuf {
    let root = user_root
        .map(PathBuf::from)
        .unwrap_or_else(|| get_user_root(None));
    root.join(AUTH_FILE_NAME)
}

pub fn get_config_file_path(user_root: Option<&Path>) -> PathBuf {
    let root = user_root
        .map(PathBuf::from)
        .unwrap_or_else(|| get_user_root(None));
    root.join(CONFIG_FILE_NAME)
}

pub fn get_repos_root(home: Option<&Path>) -> PathBuf {
    get_user_root(home).join(REPOS_DIR_NAME)
}

pub fn get_projects_root(home: Option<&Path>) -> PathBuf {
    get_user_root(home).join(PROJECTS_DIR_NAME)
}

/// `~/.poria/projects/<demand_code>` for extracted PRD / TRD markdown.
pub fn get_demand_project_dir(home: Option<&Path>, demand_code: &str) -> Result<PathBuf, String> {
    validate_path_segment(demand_code, "需求编码")?;
    Ok(get_projects_root(home).join(demand_code))
}

pub fn assert_path_under_projects_root(home: Option<&Path>, path: &Path) -> Result<(), String> {
    let root = get_projects_root(home);
    if path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err("项目路径不合法".into());
    }
    if !path.starts_with(&root) {
        return Err("项目路径不在 ~/.poria/projects 下".into());
    }
    Ok(())
}

pub fn demand_project_folder_name(demand_code: &str, demand_id: i64) -> String {
    let trimmed = demand_code.trim();
    if trimmed.is_empty() {
        format!("demand-{demand_id}")
    } else {
        trimmed.to_string()
    }
}

/// Build `~/.poria/repos/<scope>/<name>` from a parsed git URL path.
///
/// Rejects `.` / `..` / empty segments so clone + `remove_dir_all` cannot escape
/// the managed repos root (`git@host:ls/../../.ssh` must not succeed).
pub fn get_registered_repo_path(
    home: Option<&Path>,
    scope: &str,
    name: &str,
) -> Result<PathBuf, String> {
    validate_path_segment(scope, "scope")?;
    let mut path = get_repos_root(home).join(scope);
    let mut pushed_name = false;
    for segment in name.split(['/', '\\']) {
        validate_path_segment(segment, "name")?;
        path.push(segment);
        pushed_name = true;
    }
    if !pushed_name {
        return Err("无法从 git URL 解析仓库名".into());
    }
    Ok(path)
}

pub fn assert_path_under_repos_root(home: Option<&Path>, path: &Path) -> Result<(), String> {
    let root = get_repos_root(home);
    if path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err("克隆路径不合法".into());
    }
    if !path.starts_with(&root) {
        return Err("克隆路径不在 ~/.poria/repos 下".into());
    }
    Ok(())
}

fn validate_path_segment(segment: &str, field: &str) -> Result<(), String> {
    if segment.is_empty()
        || segment == "."
        || segment == ".."
        || segment.contains('\0')
        || segment.contains('/')
        || segment.contains('\\')
    {
        return Err(format!("非法{field}"));
    }
    Ok(())
}

pub fn redact_cookie(cookie: &str) -> String {
    let len = cookie.len();
    if len <= 8 {
        return format!("xxxx ({} chars)", len);
    }
    let start = &cookie[..4];
    let end = &cookie[len - 4..];
    format!("{}...{} ({} chars)", start, end, len)
}

pub fn auth_headers(creds: &JacpCredentials) -> Vec<(String, String)> {
    vec![("Cookie".into(), creds.cookie.clone())]
}

pub fn parse_username_from_cookie(cookie: &str) -> Option<String> {
    for part in cookie.split(';') {
        let trimmed = part.trim();
        if let Some(value) = trimmed.strip_prefix(&format!("{}=", ERP_COOKIE_NAME)) {
            let decoded = urlencoding_decode(value);
            if !decoded.is_empty() {
                return Some(decoded);
            }
        }
    }
    None
}

fn urlencoding_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                result.push(byte as char);
            }
        } else if c == '+' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }
    result
}

pub fn get_credentials(user_root: Option<&Path>) -> Option<JacpCredentials> {
    let path = get_auth_file_path(user_root);
    let content = std::fs::read_to_string(&path).ok()?;
    let stored: StoredAuth = serde_json::from_str(&content).ok()?;
    Some(stored.credentials)
}

pub fn get_status(user_root: Option<&Path>) -> AuthStatus {
    match get_credentials(user_root) {
        Some(creds) => AuthStatus {
            logged_in: true,
            username: Some(creds.username),
        },
        None => AuthStatus {
            logged_in: false,
            username: None,
        },
    }
}

pub fn save_credentials(creds: &JacpCredentials, user_root: Option<&Path>) -> Result<(), String> {
    let root = user_root
        .map(PathBuf::from)
        .unwrap_or_else(|| get_user_root(None));
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;

    let path = root.join(AUTH_FILE_NAME);
    let tmp_path = root.join(format!(".auth.json.{}.tmp", std::process::id()));

    let stored = StoredAuth {
        credentials: creds.clone(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    let json = serde_json::to_string_pretty(&stored).map_err(|e| e.to_string())?;

    std::fs::write(&tmp_path, &json).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp_path, &path).map_err(|e| e.to_string())?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).ok();
    }

    Ok(())
}

pub fn logout(user_root: Option<&Path>) -> Result<(), String> {
    let path = get_auth_file_path(user_root);
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_repos_root_under_poria() {
        let dir = tempfile::tempdir().unwrap();
        let path = get_repos_root(Some(dir.path()));
        assert_eq!(path, dir.path().join(".poria").join("repos"));
    }

    #[test]
    fn test_get_demand_project_dir_under_poria() {
        let dir = tempfile::tempdir().unwrap();
        let path = get_demand_project_dir(Some(dir.path()), "R2026082156824").unwrap();
        assert_eq!(
            path,
            dir.path()
                .join(".poria")
                .join("projects")
                .join("R2026082156824")
        );
        assert_path_under_projects_root(Some(dir.path()), &path).unwrap();
    }

    #[test]
    fn demand_project_dir_rejects_parent_segments() {
        let dir = tempfile::tempdir().unwrap();
        assert!(get_demand_project_dir(Some(dir.path()), "..").is_err());
        assert!(get_demand_project_dir(Some(dir.path()), "a/b").is_err());
    }

    #[test]
    fn registered_repo_path_uses_scope_and_name() {
        let dir = tempfile::tempdir().unwrap();
        let path = get_registered_repo_path(Some(dir.path()), "ls", "ls-entrance").unwrap();
        assert_eq!(
            path,
            dir.path()
                .join(".poria")
                .join("repos")
                .join("ls")
                .join("ls-entrance")
        );
        assert_path_under_repos_root(Some(dir.path()), &path).unwrap();
    }

    #[test]
    fn registered_repo_path_nests_name_segments() {
        let dir = tempfile::tempdir().unwrap();
        let path = get_registered_repo_path(Some(dir.path()), "group", "sub/repo").unwrap();
        assert_eq!(
            path,
            dir.path()
                .join(".poria")
                .join("repos")
                .join("group")
                .join("sub")
                .join("repo")
        );
    }

    #[test]
    fn registered_repo_path_rejects_parent_segments() {
        let dir = tempfile::tempdir().unwrap();
        assert!(get_registered_repo_path(Some(dir.path()), "..", "ls-entrance").is_err());
        assert!(get_registered_repo_path(Some(dir.path()), "ls", "../evil").is_err());
        assert!(get_registered_repo_path(Some(dir.path()), "ls", "foo/../bar").is_err());
    }

    #[test]
    fn test_parse_username_from_cookie() {
        assert_eq!(
            parse_username_from_cookie("other=abc; erp_erp=zhangsan; path=/"),
            Some("zhangsan".into())
        );
    }

    #[test]
    fn test_parse_username_missing() {
        assert_eq!(parse_username_from_cookie("other=abc"), None);
    }

    #[test]
    fn test_redact_cookie() {
        let result = redact_cookie("abcdefghijklmnop");
        assert!(result.starts_with("abcd"));
        assert!(result.contains("mnop"));
        assert!(result.contains("16 chars"));
    }

    #[test]
    fn test_redact_short_cookie() {
        let result = redact_cookie("ab");
        assert!(result.contains("2 chars"));
    }

    #[test]
    fn test_save_and_load_credentials() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        let creds = JacpCredentials {
            username: "testuser".into(),
            cookie: "erp_erp=testuser; path=/".into(),
        };

        save_credentials(&creds, Some(root)).unwrap();
        let loaded = get_credentials(Some(root)).unwrap();
        assert_eq!(loaded.username, "testuser");
    }

    #[test]
    fn test_get_status_no_file() {
        let dir = tempfile::tempdir().unwrap();
        let status = get_status(Some(dir.path()));
        assert!(!status.logged_in);
    }

    #[test]
    fn test_logout() {
        let dir = tempfile::tempdir().unwrap();
        let creds = JacpCredentials {
            username: "u".into(),
            cookie: "c".into(),
        };
        save_credentials(&creds, Some(dir.path())).unwrap();
        logout(Some(dir.path())).unwrap();
        assert!(get_credentials(Some(dir.path())).is_none());
    }
}
