//! Defect channel: create a Xingyun-linked defect on quality-gate failure.

use async_trait::async_trait;
use poria_core::contracts::{CapabilityMetadata, Channel, ChannelContext};
use serde::{Deserialize, Serialize};

use crate::xingyun::{jacp_fetch, JacpCredentials};

/// Placeholder constant kept for compatibility; create is implemented.
pub const DEFECT_CHANNEL_PLACEHOLDER: bool = false;

/// Action discriminator for DefectChannelInput.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DefectAction {
    ListDefects,
    GetDefect,
    CreateDefect,
}

/// Input for the Defect channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefectChannelInput {
    pub action: DefectAction,
    #[serde(rename = "defectId", skip_serializing_if = "Option::is_none")]
    pub defect_id: Option<i64>,
    #[serde(rename = "demandId", skip_serializing_if = "Option::is_none")]
    pub demand_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Output from the Defect channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefectChannelOutput {
    pub action: String,
    pub message: String,
    #[serde(rename = "defectId", skip_serializing_if = "Option::is_none")]
    pub defect_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CreateDefectInput {
    pub demand_id: i64,
    pub title: String,
    pub detail: String,
}

#[derive(Debug, Clone)]
pub struct CreateDefectResult {
    pub defect_id: String,
}

/// Creates a new Defect channel instance.
pub fn create_defect_channel() -> DefectChannel {
    DefectChannel {
        metadata: CapabilityMetadata {
            id: "channel:defect".into(),
            name: "Defect".into(),
            description: "Create defects linked to a Xingyun demand".into(),
            version: "0.2.0".into(),
        },
    }
}

pub struct DefectChannel {
    metadata: CapabilityMetadata,
}

fn is_defect_fixture() -> bool {
    std::env::var("PORIA_DEFECT_FIXTURE")
        .ok()
        .filter(|v| v == "1")
        .is_some()
        || std::env::var("PORIA_PIPELINE_FIXTURE")
            .ok()
            .filter(|v| v == "1")
            .is_some()
}

pub async fn create_defect(
    credentials: &JacpCredentials,
    input: &CreateDefectInput,
) -> Result<CreateDefectResult, Box<dyn std::error::Error + Send + Sync>> {
    if is_defect_fixture() {
        return Ok(CreateDefectResult {
            defect_id: format!("fixture-defect-{}", input.demand_id),
        });
    }
    if input.demand_id <= 0 {
        return Err("create defect requires demandId".into());
    }
    let data = jacp_fetch(
        credentials,
        "/openapi/v3/defects",
        reqwest::Method::POST,
        Some(serde_json::json!({
            "demandId": input.demand_id,
            "title": input.title,
            "description": input.detail,
        })),
        "Create defect failed",
    )
    .await?;
    let defect_id = data
        .get("id")
        .or_else(|| data.get("defectId"))
        .map(|value| match value {
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::String(s) => s.clone(),
            _ => String::new(),
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "created".into());
    Ok(CreateDefectResult { defect_id })
}

fn require_credentials(
    ctx: &ChannelContext,
) -> Result<JacpCredentials, Box<dyn std::error::Error + Send + Sync>> {
    let cookie = ctx
        .credentials
        .get("cookie")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    if cookie.trim().is_empty() {
        return Err(poria_core::types::AUTH_EXPIRED_USER_MESSAGE.into());
    }
    Ok(JacpCredentials {
        cookie,
        username: ctx
            .credentials
            .get("username")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
    })
}

#[async_trait]
impl Channel for DefectChannel {
    fn metadata(&self) -> &CapabilityMetadata {
        &self.metadata
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        ctx: ChannelContext,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let input: DefectChannelInput = serde_json::from_value(input)?;
        match input.action {
            DefectAction::CreateDefect => {
                let creds = require_credentials(&ctx)?;
                let created = create_defect(
                    &creds,
                    &CreateDefectInput {
                        demand_id: input.demand_id.unwrap_or(0),
                        title: input.title.unwrap_or_else(|| "Poria quality gate".into()),
                        detail: input.detail.unwrap_or_default(),
                    },
                )
                .await?;
                Ok(serde_json::to_value(DefectChannelOutput {
                    action: "createDefect".into(),
                    message: "created".into(),
                    defect_id: Some(created.defect_id),
                })?)
            }
            DefectAction::ListDefects | DefectAction::GetDefect => {
                Err("Defect list/get is not implemented".into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_channel_has_correct_metadata() {
        let channel = create_defect_channel();
        let meta = channel.metadata();
        assert_eq!(meta.id, "channel:defect");
        assert_eq!(meta.name, "Defect");
    }

    #[test]
    fn action_serialization() {
        let input = DefectChannelInput {
            action: DefectAction::ListDefects,
            defect_id: None,
            demand_id: None,
            title: None,
            detail: None,
        };
        let json = serde_json::to_value(&input).unwrap();
        assert_eq!(json["action"], "listDefects");
    }

    #[tokio::test]
    async fn fixture_create_defect_returns_id() {
        std::env::set_var("PORIA_DEFECT_FIXTURE", "1");
        let result = create_defect(
            &JacpCredentials {
                cookie: "c".into(),
                username: "u".into(),
            },
            &CreateDefectInput {
                demand_id: 12,
                title: "coverage".into(),
                detail: "missing".into(),
            },
        )
        .await
        .unwrap();
        std::env::remove_var("PORIA_DEFECT_FIXTURE");
        assert_eq!(result.defect_id, "fixture-defect-12");
    }
}
