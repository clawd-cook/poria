use poria_channels::xingyun::{
    format_demand_status, get_demand_by_id, is_visible_demand_status,
    list_demands as query_xingyun_demands, parse_xingyun_demand_url,
    preview_demand_prd as query_preview_demand_prd, DemandListItem, DemandListQuery, DemandPage,
    DemandPrdPreview,
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

/// List Xingyun demands related to the logged-in ERP.
///
/// `accepted_by_me` (frontend `acceptedByMe`) adds `receiver = ERP`. Default false.
#[tauri::command]
pub async fn list_demands(
    keyword: Option<String>,
    current: Option<i64>,
    page_size: Option<i64>,
    accepted_by_me: Option<bool>,
) -> Result<DemandPage, String> {
    let credentials = require_jacp_credentials()?;

    query_xingyun_demands(
        &credentials,
        DemandListQuery {
            accepted_by_me: accepted_by_me.unwrap_or(false),
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

fn reject_closed_demand(status: Option<i32>) -> Result<(), String> {
    if is_visible_demand_status(status) {
        return Ok(());
    }
    let label = format_demand_status(status);
    if label.is_empty() {
        return Err("需求无效或已关闭，无法启动流水线".into());
    }
    Err(format!("需求已关闭或不可启动（{label}）"))
}

/// Parse a Xingyun demand URL, fetch the card, and reject closed/unstartable demands.
#[tauri::command]
pub async fn resolve_demand_link(url: String) -> Result<DemandListItem, String> {
    let credentials = require_jacp_credentials()?;
    let parsed = parse_xingyun_demand_url(&url)?;
    let detail = get_demand_by_id(&credentials, parsed.demand_id)
        .await
        .map_err(map_list_demands_error)?;
    reject_closed_demand(detail.status)?;
    Ok(DemandListItem {
        demand_code: if detail.demand_code.trim().is_empty() {
            parsed.demand_code.unwrap_or_default()
        } else {
            detail.demand_code
        },
        id: detail.id,
        name: detail.name,
        receiver_erp: detail.receiver.as_ref().and_then(|user| user.erp.clone()),
        receiver_name: detail.receiver.as_ref().and_then(|user| user.name.clone()),
        status: detail.status,
        status_label: format_demand_status(detail.status),
    })
}

#[cfg(test)]
mod tests {
    use super::reject_closed_demand;

    #[test]
    fn visible_demand_can_start() {
        reject_closed_demand(Some(3)).unwrap();
        reject_closed_demand(Some(13)).unwrap();
    }

    #[test]
    fn completed_or_cancelled_demand_cannot_start() {
        assert!(reject_closed_demand(Some(20)).unwrap_err().contains("完成"));
        assert!(reject_closed_demand(Some(21))
            .unwrap_err()
            .contains("已取消"));
        assert!(reject_closed_demand(None).unwrap_err().contains("关闭"));
    }
}
