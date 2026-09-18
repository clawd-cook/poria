mod client;
mod export;
mod markdown;

use async_trait::async_trait;
use poria_core::contracts::{CapabilityMetadata, Channel, ChannelContext};
use serde::{Deserialize, Serialize};

pub use client::{
    extract_page_id_from_url, fetch_page_basic, fetch_page_content, JoySpaceAuth, DEFAULT_TEAM_ID,
    JOYSPACE_API_BASE,
};
pub use export::{export_page_markdown, markdown_from_content, ExportedDocument};
pub use markdown::{joyspace_content_to_markdown, ConversionResult, DiagramInfo};

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
            version: "0.2.0".into(),
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

                let credentials: Option<JacpCredentials> =
                    serde_json::from_value(ctx.credentials.clone()).ok();
                let cookie = credentials
                    .as_ref()
                    .map(|c| c.cookie.trim())
                    .filter(|c| !c.is_empty())
                    .ok_or(poria_core::types::AUTH_EXPIRED_USER_MESSAGE)?;
                let auth = JoySpaceAuth::new(cookie, None);
                let exported = export_page_markdown(&auth, &url).await?;

                let output_dir = input.output_dir.unwrap_or_else(|| ".".to_string());
                let output_name = input.output_name.unwrap_or_else(|| exported.title.clone());
                let output_path = format!(
                    "{}/{}.md",
                    output_dir.trim_end_matches('/'),
                    sanitize_filename(&output_name)
                );
                if let Some(parent) = std::path::Path::new(&output_path).parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&output_path, &exported.markdown)?;

                Ok(serde_json::to_value(JoySpaceChannelOutput {
                    action: "exportToMarkdown".to_string(),
                    result: ExportJoySpaceResult {
                        output_path,
                        title: exported.title,
                        cookie_source: Some("sso".to_string()),
                    },
                })?)
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
