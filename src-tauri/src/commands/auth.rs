use poria_infrastructure::auth;
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct AuthStatusView {
    pub logged_in: bool,
    pub username: Option<String>,
    pub cookie_valid: bool,
}

/// Check the current authentication status using poria-infrastructure.
#[tauri::command]
pub async fn get_auth_status() -> Result<AuthStatusView, String> {
    let status = auth::get_status(None);

    Ok(AuthStatusView {
        logged_in: status.logged_in,
        username: status.username,
        // Cookie is valid if we have credentials on file
        cookie_valid: status.logged_in,
    })
}
