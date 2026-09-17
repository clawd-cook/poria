use async_trait::async_trait;
use poria_core::contracts::{CapabilityMetadata, Channel, ChannelContext};
use serde::{Deserialize, Serialize};

/// Credentials for JoySpace API access (cookie-based SSO).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JacpCredentials {
    pub cookie: String,
    pub username: String,
}

/// Action discriminator (only exportToMarkdown for now).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum JoySpaceAction {
    ExportToMarkdown,
}

/// Input for the JoySpace channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoySpaceChannelInput {
    pub action: JoySpaceAction,
    pub url: String,
    #[serde(rename = "outputDir", skip_serializing_if = "Option::is_none")]
    pub output_dir: Option<String>,
    #[serde(rename = "outputName", skip_serializing_if = "Option::is_none")]
    pub output_name: Option<String>,
    #[serde(rename = "tenantCode", skip_serializing_if = "Option::is_none")]
    pub tenant_code: Option<String>,
}

/// Result of exporting a JoySpace document to Markdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportJoySpaceResult {
    #[serde(rename = "outputPath")]
    pub output_path: String,
    pub title: String,
    #[serde(rename = "cookieSource", skip_serializing_if = "Option::is_none")]
    pub cookie_source: Option<String>,
}

/// Output from the JoySpace channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoySpaceChannelOutput {
    pub action: String,
    pub result: ExportJoySpaceResult,
}

/// Creates a new JoySpace channel instance.
pub fn create_joyspace_channel() -> JoySpaceChannel {
    JoySpaceChannel {
        metadata: CapabilityMetadata {
            id: "channel:joyspace".into(),
            name: "JoySpace".into(),
            description: "JoySpace document export to Markdown".into(),
            version: "0.1.0".into(),
        },
    }
}

pub struct JoySpaceChannel {
    metadata: CapabilityMetadata,
}

/// Check if JoySpace fixture mode is active.
pub fn is_joyspace_fixture_mode() -> bool {
    std::env::var("PORIA_JOYSPACE_FIXTURE")
        .ok()
        .map(|v| v == "1")
        .unwrap_or(false)
}

/// Extract a page ID from a JoySpace URL.
///
/// Looks for patterns like `/pages/<pageId>` or `pageId=<id>` in the URL.
pub fn extract_page_id_from_url(url: &str) -> Result<String, String> {
    let parsed =
        url::Url::parse(url.trim()).map_err(|_| format!("Invalid JoySpace URL: {}", url))?;

    // Try query param first
    if let Some(page_id) = parsed
        .query_pairs()
        .find(|(k, _)| k == "pageId" || k == "page_id")
    {
        let id = page_id.1.trim().to_string();
        if !id.is_empty() {
            return Ok(id);
        }
    }

    // Try path segments
    let segments: Vec<&str> = parsed.path().split('/').filter(|s| !s.is_empty()).collect();
    for (i, seg) in segments.iter().enumerate() {
        if (*seg == "pages" || *seg == "page" || *seg == "p_view") && i + 1 < segments.len() {
            let candidate = segments[i + 1].trim();
            if !candidate.is_empty() && candidate.len() >= 4 {
                return Ok(candidate.to_string());
            }
        }
    }

    // Last resort: use the last long segment
    for seg in segments.iter().rev() {
        if seg.len() >= 6 {
            return Ok(seg.to_string());
        }
    }

    Err(format!("Cannot extract page ID from JoySpace URL: {}", url))
}

#[async_trait]
impl Channel for JoySpaceChannel {
    fn metadata(&self) -> &CapabilityMetadata {
        &self.metadata
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        ctx: ChannelContext,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let input: JoySpaceChannelInput = serde_json::from_value(input)?;

        match input.action {
            JoySpaceAction::ExportToMarkdown => {
                let url = input.url.trim().to_string();
                if url.is_empty() {
                    return Err("JoySpace URL is required".into());
                }

                if is_joyspace_fixture_mode() {
                    let output_dir = input.output_dir.unwrap_or_else(|| ".".to_string());
                    let title = "JoySpace Fixture Page".to_string();
                    let output_name = input.output_name.unwrap_or_else(|| title.clone());
                    let output_path =
                        format!("{}/{}.md", output_dir, sanitize_filename(&output_name));

                    let result = ExportJoySpaceResult {
                        output_path,
                        title,
                        cookie_source: Some("fixture".to_string()),
                    };

                    return Ok(serde_json::to_value(JoySpaceChannelOutput {
                        action: "exportToMarkdown".to_string(),
                        result,
                    })?);
                }

                // Live mode: requires JoySpace API integration with vendor modules.
                // The Rust port provides the type structure and URL parsing;
                // the actual API calls and markdown conversion require the vendored
                // joyspace-api-client and joyspace-content-to-markdown modules
                // which are JavaScript-specific. This will be bridged via Tauri commands.
                let _page_id = extract_page_id_from_url(&url)?;
                let _credentials: Option<JacpCredentials> =
                    serde_json::from_value(ctx.credentials.clone()).ok();

                Err(
                    "JoySpace live export not yet implemented in Rust (requires vendor JS bridge)"
                        .into(),
                )
            }
        }
    }
}

fn sanitize_filename(value: &str) -> String {
    let cleaned: String = value
        .chars()
        .map(|c| if "/\\:*?\"<>|".contains(c) { '_' } else { c })
        .collect();
    let trimmed = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    if trimmed.is_empty() {
        "Doc".to_string()
    } else {
        trimmed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_mode_returns_false_by_default() {
        // Clear the env var to ensure default behavior
        std::env::remove_var("PORIA_JOYSPACE_FIXTURE");
        assert!(!is_joyspace_fixture_mode());
    }

    #[test]
    fn extract_page_id_from_pages_path() {
        let id = extract_page_id_from_url("https://joyspace.jd.com/pages/abc123def456").unwrap();
        assert_eq!(id, "abc123def456");
    }

    #[test]
    fn extract_page_id_from_query_param() {
        let id = extract_page_id_from_url("https://joyspace.jd.com/view?pageId=mypage123").unwrap();
        assert_eq!(id, "mypage123");
    }

    #[test]
    fn sanitize_strips_illegal_chars() {
        assert_eq!(sanitize_filename("test/file:name"), "test_file_name");
    }

    #[test]
    fn sanitize_empty_returns_doc() {
        assert_eq!(sanitize_filename(""), "Doc");
    }
}
