use poria_channels::xingyun::{
    list_demands as query_xingyun_demands, preview_demand_prd as query_preview_demand_prd,
    DemandListQuery, DemandPage, DemandPrdPreview,
};
use poria_infrastructure::auth;

fn map_list_demands_error(err: impl ToString) -> String {
    let message = err.to_string();
    let lower = message.to_lowercase();
    if message.contains("AuthRequired")
        || lower.contains("auth expired")
        || lower.contains("re-login")
    {
        "登录已过期，请重新登录".into()
    } else {
        message
    }
}

fn require_jacp_credentials() -> Result<poria_channels::xingyun::JacpCredentials, String> {
    let creds = auth::get_credentials(None).ok_or_else(|| "请先登录".to_string())?;
    if creds.cookie.trim().is_empty() || creds.username.trim().is_empty() {
        return Err("请先登录".into());
    }
    Ok(poria_channels::xingyun::JacpCredentials {
        cookie: creds.cookie,
        username: creds.username,
    })
}

/// List Xingyun demands assigned to the logged-in ERP.
#[tauri::command]
pub async fn list_demands(
    keyword: Option<String>,
    current: Option<i64>,
    page_size: Option<i64>,
) -> Result<DemandPage, String> {
    let credentials = require_jacp_credentials()?;

    query_xingyun_demands(
        &credentials,
        DemandListQuery {
            keyword,
            current: current.unwrap_or(0),
            page_size: page_size.unwrap_or(0),
        },
    )
    .await
    .map_err(map_list_demands_error)
}

/// Best-effort JoySpace PRD URL for the start wizard.
#[tauri::command]
pub async fn preview_demand_prd(demand_id: i64) -> Result<DemandPrdPreview, String> {
    let credentials = require_jacp_credentials()?;
    query_preview_demand_prd(&credentials, demand_id)
        .await
        .map_err(map_list_demands_error)
}
