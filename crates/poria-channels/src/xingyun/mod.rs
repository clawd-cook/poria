mod demand_url;
mod git_url;
mod types;

pub use demand_url::{feature_branch_name, feature_slug, parse_xingyun_demand_url};
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
                let demand_id = input
                    .demand_id
                    .ok_or("getDemand requires demandId")?;
                let demand = get_demand_by_id(&credentials, demand_id).await?;
                XingyunChannelOutput::GetDemand { demand: Box::new(demand) }
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
                let demand_id = input
                    .demand_id
                    .ok_or("resolvePrdLink requires demandId")?;
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
                let demand_id = input
                    .demand_id
                    .ok_or("communicate requires demandId")?;
                let result = communicate_demand(&credentials, demand_id).await?;
                XingyunChannelOutput::Communicate {
                    demand_id,
                    demand_status_code: result.demand_status_code,
                }
            }
            XingyunAction::Accept => {
                let demand_id = input
                    .demand_id
                    .ok_or("accept requires demandId")?;
                let result = accept_demand(&credentials, demand_id).await?;
                XingyunChannelOutput::Accept {
                    demand_id,
                    demand_status_code: result.demand_status_code,
                }
            }
            XingyunAction::BindBranch => {
                // BindBranch requires git operations and EasyCI integration.
                // Full implementation deferred -- returns an error for now since
                // it depends on local git context.
                return Err("bindBranch action requires local git context (not yet supported in Rust)".into());
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
            if trimmed.is_empty() { None } else { Some(trimmed) }
        })
        .unwrap_or_else(|| DEFAULT_JACP_BASE_URL.to_string())
}

fn jacp_app_id() -> String {
    std::env::var("PORIA_JACP_APP_ID")
        .ok()
        .and_then(|s| {
            let trimmed = s.trim().to_string();
            if trimmed.is_empty() { None } else { Some(trimmed) }
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

async fn jacp_fetch(
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

    let response = req.send().await.map_err(|e| {
        format!("{}: {}", error_label, e)
    })?;

    if response.status().as_u16() == 401 {
        return Err("Auth expired, please re-login (poria auth login)".into());
    }
    if !response.status().is_success() {
        return Err(format!("{}: HTTP {}", error_label, response.status()).into());
    }

    let envelope: JacpEnvelope = response.json().await.map_err(|e| {
        format!("{}: failed to parse response: {}", error_label, e)
    })?;

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

async fn get_demand_by_id(
    credentials: &JacpCredentials,
    demand_id: i64,
) -> Result<DemandDetail, Box<dyn std::error::Error + Send + Sync>> {
    let path = format!("/openapi/v3/demands/{}", demand_id);
    let data = jacp_fetch(credentials, &path, reqwest::Method::GET, None, "Demand query failed").await?;
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
    let path = format!(
        "/openapi/v3/cards/code/{}",
        urlencoding::encode(code)
    );
    let data = jacp_fetch(credentials, &path, reqwest::Method::GET, None, "Card query failed").await?;

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
            let host = parsed.host_str().unwrap_or("");
            host.ends_with("joyspace.jd.com")
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
mod urlencoding {
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
