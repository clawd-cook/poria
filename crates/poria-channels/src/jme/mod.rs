mod reply_parser;

pub use reply_parser::{parse_reply, ReplyAction};

use async_trait::async_trait;
use poria_core::contracts::{CapabilityMetadata, Channel, ChannelContext};
use serde::{Deserialize, Serialize};

/// Configuration for the JoyClaw bridge.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JoyClawBridgeConfig {
    #[serde(rename = "nodePath", skip_serializing_if = "Option::is_none")]
    pub node_path: Option<String>,
    #[serde(rename = "openclawPath", skip_serializing_if = "Option::is_none")]
    pub openclaw_path: Option<String>,
    #[serde(rename = "gatewayPort", skip_serializing_if = "Option::is_none")]
    pub gateway_port: Option<u16>,
}

/// Action discriminator for JmeChannelInput.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum JmeAction {
    Send,
    ReadReplies,
    EnsureGatewayAlive,
}

/// Input for the JME channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JmeChannelInput {
    pub action: JmeAction,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub send: Option<SendInput>,
    #[serde(rename = "readReplies", skip_serializing_if = "Option::is_none")]
    pub read_replies: Option<ReadRepliesInput>,
    #[serde(rename = "bridgeConfig", skip_serializing_if = "Option::is_none")]
    pub bridge_config: Option<JoyClawBridgeConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendInput {
    pub target: String,
    pub message: String,
    #[serde(rename = "timeoutSec", skip_serializing_if = "Option::is_none")]
    pub timeout_sec: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadRepliesInput {
    #[serde(rename = "chatName")]
    pub chat_name: String,
    pub since: String,
}

/// Output from the JME channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
pub enum JmeChannelOutput {
    #[serde(rename = "send")]
    Send { result: String },
    #[serde(rename = "readReplies")]
    ReadReplies { replies: Vec<String> },
    #[serde(rename = "ensureGatewayAlive")]
    EnsureGatewayAlive { alive: bool },
}

/// Creates a new JME channel instance.
pub fn create_jme_channel() -> JmeChannel {
    JmeChannel {
        metadata: CapabilityMetadata {
            id: "channel:jme".into(),
            name: "JME".into(),
            description: "JingME messaging via JoyClaw bridge".into(),
            version: "0.1.0".into(),
        },
    }
}

pub struct JmeChannel {
    metadata: CapabilityMetadata,
}

const DEFAULT_GATEWAY_PORT: u16 = 18810;

fn gateway_port(config: Option<&JoyClawBridgeConfig>) -> u16 {
    config
        .and_then(|c| c.gateway_port)
        .unwrap_or(DEFAULT_GATEWAY_PORT)
}

#[async_trait]
impl Channel for JmeChannel {
    fn metadata(&self) -> &CapabilityMetadata {
        &self.metadata
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _ctx: ChannelContext,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let input: JmeChannelInput = serde_json::from_value(input)?;

        let output = match input.action {
            JmeAction::Send => {
                let send = input.send.ok_or("send requires send input")?;
                // JME send requires local JoyClaw agent (child process spawn).
                // This is a placeholder that documents the expected behavior.
                let result = format!(
                    "[rust-channel] Would send to {}: {}",
                    send.target,
                    &send.message[..send.message.len().min(100)]
                );
                JmeChannelOutput::Send { result }
            }
            JmeAction::ReadReplies => {
                let read = input
                    .read_replies
                    .ok_or("readReplies requires readReplies input")?;
                let _ = &read.since; // Since timestamp
                let _ = &read.chat_name;
                // Placeholder -- reading replies requires JoyClaw agent
                JmeChannelOutput::ReadReplies {
                    replies: vec![],
                }
            }
            JmeAction::EnsureGatewayAlive => {
                let port = gateway_port(input.bridge_config.as_ref());
                let alive = check_gateway_alive(port).await;
                JmeChannelOutput::EnsureGatewayAlive { alive }
            }
        };

        Ok(serde_json::to_value(output)?)
    }
}

/// Check if the JoyClaw gateway is alive by probing its health endpoint.
async fn check_gateway_alive(port: u16) -> bool {
    let url = format!("http://127.0.0.1:{}/health", port);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build();
    match client {
        Ok(client) => match client.get(&url).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        },
        Err(_) => false,
    }
}
