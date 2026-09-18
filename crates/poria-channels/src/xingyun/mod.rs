mod bind_branch;
mod demand_list;
mod demand_status;
mod demand_url;
mod git_url;
mod types;

pub use bind_branch::{bind_branch, BindBranchInput, BindBranchResult};
pub use demand_status::{format_demand_status, is_visible_demand_status};
pub use demand_url::{
    feature_branch_name, feature_slug, parse_xingyun_demand_url, xingyun_demand_view_url,
};
pub use git_url::{normalize_git_url, repo_search_path_from_git_url, same_git_url};
pub use types::*;

use async_trait::async_trait;
use poria_core::contracts::{CapabilityMetadata, Channel, ChannelContext};

/// Creates a new Xingyun channel instance.
pub fn create_xingyun_channel() -> XingyunChannel {
    XingyunChannel {
        metadata: CapabilityMetadata {
            id: "channel:xingyun".into(),
            name: "Xingyun".into(),
            description: "Xingyun demand, card, PRD, and branch binding operations".into(),
            version: "0.1.0".into(),
        },
    }
}

pub struct XingyunChannel {
    metadata: CapabilityMetadata,
}

#[async_trait]
impl Channel for XingyunChannel {
    fn metadata(&self) -> &CapabilityMetadata {
        &self.metadata
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        ctx: ChannelContext,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let input: XingyunChannelInput = serde_json::from_value(input)?;
        let credentials = require_credentials(&ctx)?;

        let output = match input.action {
            XingyunAction::GetDemand => {
                let demand_id = input.demand_id.ok_or("getDemand requires demandId")?;
                let demand = get_demand_by_id(&credentials, demand_id).await?;
                XingyunChannelOutput::GetDemand {
                    demand: Box::new(demand),
                }
            }
            XingyunAction::ListDemands => {
                let page = list_demands(
                    &credentials,
                    DemandListQuery {
                        accepted_by_me: input.accepted_by_me.unwrap_or(false),
                        keyword: input.keyword,
                        current: input.current.unwrap_or(0),
                        page_size: input.page_size.unwrap_or(0),
                    },
                )
                .await?;
                XingyunChannelOutput::ListDemands {
                    records: page.records,
                    total: page.total,
                    current: page.current,
                    page_size: page.page_size,
                }
            }
            XingyunAction::ListCardAttachments => {
                let demand_code = match input.demand_code {
                    Some(code) if !code.trim().is_empty() => code.trim().to_string(),
                    _ => {
                        let demand_id = input
                            .demand_id
                            .ok_or("listCardAttachments requires demandId or demandCode")?;
                        let demand = get_demand_by_id(&credentials, demand_id).await?;
                        demand.demand_code.clone()
                    }
                };
                let attachments = get_card_attachments(&credentials, &demand_code).await?;
                XingyunChannelOutput::ListCardAttachments { attachments }
            }
            XingyunAction::ResolvePrdLink => {
                let demand_id = input.demand_id.ok_or("resolvePrdLink requires demandId")?;
                let demand = get_demand_by_id(&credentials, demand_id).await?;
                let code = demand.demand_code.trim().to_string();
                let attachments = if code.is_empty() {
                    vec![]
                } else {
                    get_card_attachments(&credentials, &code)
                        .await
                        .unwrap_or_default()
                };
                let result = resolve_prd_from_attachments(&attachments)?;
                XingyunChannelOutput::ResolvePrdLink {
                    url: result.url,
                    attachment: result.attachment,
                }
            }
            XingyunAction::Communicate => {
                let demand_id = input.demand_id.ok_or("communicate requires demandId")?;
                let result = communicate_demand(&credentials, demand_id).await?;
                XingyunChannelOutput::Communicate {
                    demand_id,
                    demand_status_code: result.demand_status_code,
                }
            }
            XingyunAction::Accept => {
                let demand_id = input.demand_id.ok_or("accept requires demandId")?;
                let result = accept_demand(&credentials, demand_id).await?;
                XingyunChannelOutput::Accept {
                    demand_id,
                    demand_status_code: result.demand_status_code,
                }
            }
            XingyunAction::BindBranch => {
                let git_url = input
                    .git_url
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .ok_or("bindBranch requires gitUrl")?
                    .to_string();
                let branch = input
                    .branch
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .ok_or("bindBranch requires branch")?
                    .to_string();
                let result = bind_branch(
                    &credentials,
                    BindBranchInput {
                        demand_id: input.demand_id,
                        demand_code: input.demand_code,
                        git_url,
                        branch,
                        base_branch: input.base_branch,
                    },
                )
                .await?;
                XingyunChannelOutput::BindBranch {
                    branch: result.branch,
                    change_id: result.change_id,
                    base_branch: result.base_branch,
                }
            }
        };

        Ok(serde_json::to_value(output)?)
    }
}

fn require_credentials(
    ctx: &ChannelContext,
) -> Result<JacpCredentials, Box<dyn std::error::Error + Send + Sync>> {
    let creds: JacpCredentials = serde_json::from_value(ctx.credentials.clone())?;
    if creds.cookie.is_empty() {
        return Err("AuthRequired: please run poria auth login".into());
    }
    Ok(creds)
}

// ---------------------------------------------------------------------------
// JACP HTTP helpers
// ---------------------------------------------------------------------------

const DEFAULT_JACP_BASE_URL: &str = "http://api-gateway.jd.com/jacp";
const DEFAULT_JACP_APP_ID: &str = "joycode";

fn jacp_base_url() -> String {
    std::env::var("PORIA_JACP_BASE_URL")
        .ok()
        .and_then(|s| {
            let trimmed = s.trim().trim_end_matches('/').to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        })
        .unwrap_or_else(|| DEFAULT_JACP_BASE_URL.to_string())
}

fn jacp_app_id() -> String {
    std::env::var("PORIA_JACP_APP_ID")
        .ok()
        .and_then(|s| {
            let trimmed = s.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        })
        .unwrap_or_else(|| DEFAULT_JACP_APP_ID.to_string())
}

#[derive(serde::Deserialize)]
struct JacpEnvelope {
    code: Option<i32>,
    message: Option<String>,
    msg: Option<String>,
    data: Option<serde_json::Value>,
}

pub(crate) async fn jacp_fetch(
    credentials: &JacpCredentials,
    path: &str,
    method: reqwest::Method,
    body: Option<serde_json::Value>,
    error_label: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    let base = jacp_base_url();
    let app_id = jacp_app_id();
    let url = format!("{}{}", base, path);

    let client = reqwest::Client::new();
    let mut req = client
        .request(method, &url)
        .header("Cookie", &credentials.cookie)
        .header("optErp", &credentials.username)
        .header("appId", &app_id);

    if let Some(json_body) = body {
        req = req
            .header("Content-Type", "application/json")
            .json(&json_body);
    }

    let response = req
        .send()
        .await
        .map_err(|e| format!("{}: {}", error_label, e))?;

    if response.status().as_u16() == 401 {
        return Err("Auth expired, please re-login (poria auth login)".into());
    }
    if !response.status().is_success() {
        return Err(format!("{}: HTTP {}", error_label, response.status()).into());
    }

    let envelope: JacpEnvelope = response
        .json()
        .await
        .map_err(|e| format!("{}: failed to parse response: {}", error_label, e))?;

    if envelope.code != Some(200) {
        let msg = envelope
            .message
            .or(envelope.msg)
            .unwrap_or_else(|| format!("code {:?}", envelope.code));
        return Err(format!("{}: {}", error_label, msg).into());
    }

    envelope
        .data
        .ok_or_else(|| format!("{}: empty response", error_label).into())
}

// ---------------------------------------------------------------------------
// Demand API
// ---------------------------------------------------------------------------

/// List demands related to the current ERP.
///
/// Default omits `receiver` (related-to-me via cookie / `optErp`). When
/// `query.accepted_by_me` is true, the body includes `receiver = ERP`.
pub async fn list_demands(
    credentials: &JacpCredentials,
    query: DemandListQuery,
) -> Result<DemandPage, Box<dyn std::error::Error + Send + Sync>> {
    if credentials.cookie.trim().is_empty() {
        return Err("AuthRequired: please run poria auth login".into());
    }
    let receiver = credentials.username.trim();
    if receiver.is_empty() {
        return Err("AuthRequired: missing ERP username".into());
    }

    let query = query.normalized();
    let body = demand_list::demand_list_query_body(&query, receiver);
    let data = jacp_fetch(
        credentials,
        "/openapi/v3/demands/query",
        reqwest::Method::POST,
        Some(body),
        "Demand list query failed",
    )
    .await?;
    let fallback_receiver = query.accepted_by_me.then_some(receiver);
    Ok(demand_list::parse_demand_page(
        data,
        query.current,
        query.page_size,
        fallback_receiver,
    ))
}

/// Fetch a demand by numeric id.
pub async fn get_demand_by_id(
    credentials: &JacpCredentials,
    demand_id: i64,
) -> Result<DemandDetail, Box<dyn std::error::Error + Send + Sync>> {
    let path = format!("/openapi/v3/demands/{}", demand_id);
    let data = jacp_fetch(
        credentials,
        &path,
        reqwest::Method::GET,
        None,
        "Demand query failed",
    )
    .await?;
    let detail: DemandDetail = serde_json::from_value(data)?;
    Ok(detail)
}

async fn communicate_demand(
    credentials: &JacpCredentials,
    demand_id: i64,
) -> Result<DemandActionResult, Box<dyn std::error::Error + Send + Sync>> {
    let body = serde_json::json!({ "demandId": demand_id });
    let data = jacp_fetch(
        credentials,
        "/openapi/v3/demands/actions/communicate",
        reqwest::Method::POST,
        Some(body),
        "Communicate demand failed",
    )
    .await;
    match data {
        Ok(v) => Ok(serde_json::from_value(v).unwrap_or(DemandActionResult {
            demand_id: Some(demand_id),
            demand_status_code: None,
            task_owner: None,
            task_id: None,
            task_name: None,
        })),
        Err(e) => Err(e),
    }
}

async fn accept_demand(
    credentials: &JacpCredentials,
    demand_id: i64,
) -> Result<DemandActionResult, Box<dyn std::error::Error + Send + Sync>> {
    let body = serde_json::json!({ "demandId": demand_id });
    let data = jacp_fetch(
        credentials,
        "/openapi/v3/demands/actions/accept",
        reqwest::Method::POST,
        Some(body),
        "Accept demand failed",
    )
    .await;
    match data {
        Ok(v) => Ok(serde_json::from_value(v).unwrap_or(DemandActionResult {
            demand_id: Some(demand_id),
            demand_status_code: None,
            task_owner: None,
            task_id: None,
            task_name: None,
        })),
        Err(e) => Err(e),
    }
}

// ---------------------------------------------------------------------------
// Card / attachments API
// ---------------------------------------------------------------------------

async fn get_card_attachments(
    credentials: &JacpCredentials,
    card_code: &str,
) -> Result<Vec<CardAttachment>, Box<dyn std::error::Error + Send + Sync>> {
    let code = card_code.trim();
    if code.is_empty() {
        return Err("Missing card code".into());
    }
    let path = format!("/openapi/v3/cards/code/{}", urlencoding::encode(code));
    let data = jacp_fetch(
        credentials,
        &path,
        reqwest::Method::GET,
        None,
        "Card query failed",
    )
    .await?;

    // Extract attachments array from the card response
    let attachments_val = data
        .as_object()
        .and_then(|obj| obj.get("attachments"))
        .cloned()
        .unwrap_or(serde_json::Value::Array(vec![]));

    let raw: Vec<serde_json::Value> = serde_json::from_value(attachments_val).unwrap_or_default();
    let attachments: Vec<CardAttachment> = raw
        .into_iter()
        .filter_map(|v| {
            let obj = v.as_object()?;
            let name = obj.get("name")?.as_str()?.trim().to_string();
            let url = obj.get("url")?.as_str()?.trim().to_string();
            if name.is_empty() || url.is_empty() {
                return None;
            }
            Some(CardAttachment {
                id: obj.get("id").and_then(|v| v.as_i64()),
                tag_id: obj.get("tagId").and_then(|v| v.as_i64()),
                tag_name: obj
                    .get("tagName")
                    .and_then(|v| v.as_str())
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|| "attachment".to_string()),
                name,
                url,
            })
        })
        .collect();
    Ok(attachments)
}

// ---------------------------------------------------------------------------
// PRD resolution
// ---------------------------------------------------------------------------

/// Check if a URL is a JoySpace PRD link.
pub fn is_joyspace_prd_link(raw: &str) -> bool {
    match url::Url::parse(raw.trim()) {
        Ok(parsed) => {
            let host = parsed.host_str().unwrap_or("").to_ascii_lowercase();
            host == "joyspace.jd.com" || host.ends_with(".joyspace.jd.com")
        }
        Err(_) => false,
    }
}

pub struct PrdResolveResult {
    pub url: String,
    pub attachment: CardAttachment,
}

#[derive(Debug, thiserror::Error)]
pub enum PrdResolveError {
    #[error("No JoySpace PRD attachment found on the demand card")]
    NoPrd,
    #[error("Multiple JoySpace PRD candidates; resolve manually")]
    AmbiguousPrd,
}

/// Best-effort PRD URL for the start wizard (`getDemand` + `resolvePrdLink`).
pub async fn preview_demand_prd(
    credentials: &JacpCredentials,
    demand_id: i64,
) -> Result<DemandPrdPreview, Box<dyn std::error::Error + Send + Sync>> {
    if demand_id <= 0 {
        return Err("demandId 无效".into());
    }
    let demand = get_demand_by_id(credentials, demand_id).await?;
    let demand_code = demand.demand_code.trim().to_string();
    let attachments = if demand_code.is_empty() {
        Vec::new()
    } else {
        get_card_attachments(credentials, &demand_code)
            .await
            .unwrap_or_default()
    };
    let mut url = resolve_prd_from_attachments(&attachments)
        .ok()
        .map(|result| result.url);
    if url.is_none() {
        if let Some(link) = demand
            .demand_desc_link
            .as_deref()
            .map(str::trim)
            .filter(|link| !link.is_empty() && is_joyspace_prd_link(link))
        {
            url = Some(link.to_string());
        }
    }
    Ok(DemandPrdPreview {
        demand_id: demand.id,
        demand_code,
        demand_name: demand.name,
        url,
    })
}

/// Resolve the single best PRD attachment from a list.
pub fn resolve_prd_from_attachments(
    attachments: &[CardAttachment],
) -> Result<PrdResolveResult, PrdResolveError> {
    let joyspace: Vec<&CardAttachment> = attachments
        .iter()
        .filter(|a| is_joyspace_prd_link(&a.url))
        .collect();

    if joyspace.is_empty() {
        return Err(PrdResolveError::NoPrd);
    }
    if joyspace.len() == 1 {
        return Ok(PrdResolveResult {
            url: joyspace[0].url.clone(),
            attachment: joyspace[0].clone(),
        });
    }

    // Prefer attachments whose name/tag mentions PRD
    let prd_re = regex::RegexBuilder::new(r"prd")
        .case_insensitive(true)
        .build()
        .unwrap();

    let preferred: Vec<&CardAttachment> = joyspace
        .iter()
        .filter(|a| prd_re.is_match(&a.tag_name) || prd_re.is_match(&a.name))
        .copied()
        .collect();

    let candidates = if !preferred.is_empty() {
        preferred
    } else {
        joyspace
    };

    if candidates.len() == 1 {
        return Ok(PrdResolveResult {
            url: candidates[0].url.clone(),
            attachment: candidates[0].clone(),
        });
    }

    Err(PrdResolveError::AmbiguousPrd)
}

/// Returns true if the demand status is considered active.
/// MVP implementation always returns true (no status filtering).
pub fn is_demand_active(_status: Option<i32>) -> bool {
    true
}

// Need urlencoding for path encoding
pub(crate) mod urlencoding {
    pub fn encode(input: &str) -> String {
        let mut result = String::new();
        for byte in input.bytes() {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    result.push(byte as char);
                }
                _ => {
                    result.push_str(&format!("%{:02X}", byte));
                }
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn list_demand_rejects_missing_cookie() {
        let creds = JacpCredentials {
            cookie: String::new(),
            username: "heyongqi10".into(),
        };
        let err = list_demands(&creds, DemandListQuery::default())
            .await
            .unwrap_err();
        assert!(err.to_string().contains("AuthRequired"));
    }

    #[tokio::test]
    async fn list_demand_rejects_missing_erp() {
        let creds = JacpCredentials {
            cookie: "erp_erp=x".into(),
            username: "  ".into(),
        };
        let err = list_demands(&creds, DemandListQuery::default())
            .await
            .unwrap_err();
        assert!(err.to_string().contains("AuthRequired"));
    }

    #[tokio::test]
    async fn preview_demand_prd_rejects_invalid_id() {
        let creds = JacpCredentials {
            cookie: "erp_erp=x".into(),
            username: "heyongqi10".into(),
        };
        let err = preview_demand_prd(&creds, 0).await.unwrap_err();
        assert!(err.to_string().contains("demandId"));
    }

    #[test]
    fn joyspace_prd_link_accepts_apex_and_subdomain() {
        assert!(is_joyspace_prd_link("https://joyspace.jd.com/pages/abc"));
        assert!(is_joyspace_prd_link(
            "https://doc.joyspace.jd.com/pages/abc"
        ));
    }

    #[test]
    fn joyspace_prd_link_rejects_lookalike_hosts() {
        assert!(!is_joyspace_prd_link(
            "https://eviljoyspace.jd.com/pages/abc"
        ));
        assert!(!is_joyspace_prd_link("https://example.com/pages/abc"));
        assert!(!is_joyspace_prd_link("not-a-url"));
    }
}
