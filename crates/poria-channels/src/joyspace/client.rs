use serde_json::{json, Value};

pub const JOYSPACE_API_BASE: &str = "https://apijoyspace.jd.com";
pub const DEFAULT_TEAM_ID: &str = "00046419";

#[derive(Debug, Clone)]
pub struct JoySpaceAuth {
    pub cookie: String,
    pub team_id: String,
}

fn encode_query_component(value: &str) -> String {
    url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

impl JoySpaceAuth {
    pub fn new(cookie: impl Into<String>, team_id: Option<&str>) -> Self {
        let team = team_id
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(DEFAULT_TEAM_ID);
        Self {
            cookie: cookie.into(),
            team_id: team.to_string(),
        }
    }
}

pub fn extract_page_id_from_url(url: &str) -> Result<String, String> {
    let parsed =
        url::Url::parse(url.trim()).map_err(|_| format!("Invalid JoySpace URL: {}", url))?;

    if let Some(page_id) = parsed
        .query_pairs()
        .find(|(k, _)| k == "pageId" || k == "page_id")
    {
        let id = page_id.1.trim().to_string();
        if !id.is_empty() {
            return Ok(id);
        }
    }

    let segments: Vec<&str> = parsed.path().split('/').filter(|s| !s.is_empty()).collect();
    for (i, seg) in segments.iter().enumerate() {
        if matches!(
            *seg,
            "pages"
                | "page"
                | "doc"
                | "table"
                | "ppt"
                | "board"
                | "mind"
                | "meeting"
                | "p_view"
                | "sheets"
                | "sheet"
        ) && i + 1 < segments.len()
        {
            let candidate = segments[i + 1].trim();
            if !candidate.is_empty() && candidate.len() >= 4 {
                return Ok(candidate.to_string());
            }
        }
    }

    for seg in segments.iter().rev() {
        if seg.len() >= 6 {
            return Ok(seg.to_string());
        }
    }

    Err(format!("Cannot extract page ID from JoySpace URL: {}", url))
}

async fn request_json(
    auth: &JoySpaceAuth,
    method: reqwest::Method,
    path: &str,
    body: Option<Value>,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let url = format!("{}{}", JOYSPACE_API_BASE, path);
    let client = reqwest::Client::new();
    let mut req = client
        .request(method, &url)
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .header("Cookie", &auth.cookie)
        .header("x-team-id", &auth.team_id)
        .header("Origin", "https://joyspace.jd.com")
        .header("Referer", "https://joyspace.jd.com/");

    if let Some(json_body) = body {
        req = req.json(&json_body);
    }

    let response = req.send().await?;
    if response.status().as_u16() == 401 {
        return Err(poria_core::types::AUTH_EXPIRED_USER_MESSAGE.into());
    }
    if !response.status().is_success() {
        return Err(format!("{path} HTTP {}", response.status()).into());
    }

    let json: Value = response.json().await?;
    let failed = json.get("status").and_then(|v| v.as_str()) == Some("failed")
        || json.get("errCode").is_some()
        || json
            .get("errorCode")
            .and_then(|v| v.as_str())
            .is_some_and(|code| code != "0")
        || json
            .get("code")
            .and_then(|v| {
                v.as_i64()
                    .map(|n| n != 0)
                    .or_else(|| v.as_str().map(|s| s != "0"))
            })
            .unwrap_or(false);

    if failed {
        let err_code = json
            .get("errCode")
            .or_else(|| json.get("errorCode"))
            .or_else(|| json.get("code"))
            .map(|v| v.to_string())
            .unwrap_or_else(|| "unknown".into());
        let err_msg = json
            .get("errMsg")
            .or_else(|| json.get("errorMsg"))
            .or_else(|| json.get("msg"))
            .or_else(|| json.get("message"))
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown API error");
        return Err(format!("JoySpace API error {err_code}: {err_msg} ({path})").into());
    }

    if let Some(data) = json.get("data") {
        return Ok(data.clone());
    }
    Ok(json)
}

pub async fn fetch_page_basic(
    auth: &JoySpaceAuth,
    page_id: &str,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    request_json(
        auth,
        reqwest::Method::GET,
        &format!("/v3/pages/{page_id}/basic?sendRecent=0"),
        None,
    )
    .await
}

pub async fn fetch_page_content(
    auth: &JoySpaceAuth,
    page_id: &str,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    request_json(
        auth,
        reqwest::Method::POST,
        "/v1/pages/content",
        Some(json!({ "pageId": page_id })),
    )
    .await
}

pub async fn fetch_diagram_detail(
    auth: &JoySpaceAuth,
    diagram_id: &str,
    page_id: &str,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let path = format!(
        "/v1/diagram/{}/detail?pageId={}",
        encode_query_component(diagram_id),
        encode_query_component(page_id)
    );
    request_json(auth, reqwest::Method::GET, &path, None).await
}
