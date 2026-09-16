use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct AuthStatus {
    pub logged_in: bool,
    pub username: Option<String>,
    pub cookie_valid: bool,
}

/// Check the current authentication status by reading ~/.poria/auth.json.
#[tauri::command]
pub async fn get_auth_status() -> Result<AuthStatus, String> {
    let auth_path = dirs::home_dir()
        .unwrap_or_default()
        .join(".poria/auth.json");

    if !auth_path.exists() {
        return Ok(AuthStatus {
            logged_in: false,
            username: None,
            cookie_valid: false,
        });
    }

    let content = std::fs::read_to_string(&auth_path).map_err(|e| e.to_string())?;
    let auth: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| e.to_string())?;

    Ok(AuthStatus {
        logged_in: true,
        username: auth
            .get("erp")
            .and_then(|v| v.as_str())
            .map(String::from),
        // MVP: assume cookie is valid if the file exists and parses
        cookie_valid: true,
    })
}
