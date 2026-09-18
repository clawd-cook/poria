mod agent_output;
mod reply_parser;

pub use agent_output::parse_agent_output;
pub use reply_parser::{parse_reply, ReplyAction};

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use async_trait::async_trait;
use poria_core::contracts::{CapabilityMetadata, Channel, ChannelContext};
use serde::{Deserialize, Serialize};
use tokio::process::Command;

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
            version: "0.2.0".into(),
        },
    }
}

pub struct JmeChannel {
    metadata: CapabilityMetadata,
}

const DEFAULT_GATEWAY_PORT: u16 = 18810;
const DEFAULT_AGENT_TIMEOUT_SEC: u64 = 120;

pub fn is_jme_fixture() -> bool {
    std::env::var("PORIA_JME_FIXTURE")
        .ok()
        .filter(|v| v == "1")
        .is_some()
        || std::env::var("PORIA_PIPELINE_FIXTURE")
            .ok()
            .filter(|v| v == "1")
            .is_some()
}

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
        let config = input.bridge_config.clone();

        let output = match input.action {
            JmeAction::Send => {
                let send = input.send.ok_or("send requires send input")?;
                let result = send_message(
                    &send.target,
                    &send.message,
                    send.timeout_sec,
                    config.as_ref(),
                )
                .await?;
                JmeChannelOutput::Send { result }
            }
            JmeAction::ReadReplies => {
                let read = input
                    .read_replies
                    .ok_or("readReplies requires readReplies input")?;
                let replies = read_replies(
                    &read.chat_name,
                    &read.since,
                    Some(DEFAULT_AGENT_TIMEOUT_SEC),
                    config.as_ref(),
                )
                .await?;
                JmeChannelOutput::ReadReplies { replies }
            }
            JmeAction::EnsureGatewayAlive => {
                let port = gateway_port(config.as_ref());
                let alive = check_gateway_alive(port).await;
                JmeChannelOutput::EnsureGatewayAlive { alive }
            }
        };

        Ok(serde_json::to_value(output)?)
    }
}

/// Send a JingME message through JoyClaw (or fixture).
pub async fn send_message(
    target: &str,
    message: &str,
    timeout_sec: Option<u64>,
    config: Option<&JoyClawBridgeConfig>,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    if is_jme_fixture() {
        let preview = &message[..message.len().min(80)];
        return Ok(format!("fixture sent to {target}: {preview}"));
    }
    let port = gateway_port(config);
    if !check_gateway_alive(port).await {
        return Err(format!("JoyClaw gateway not alive on :{port}").into());
    }
    let prompt = format!("向 {target} 发送京ME消息: {message}");
    let texts = run_joyclaw_agent(
        &prompt,
        timeout_sec.unwrap_or(DEFAULT_AGENT_TIMEOUT_SEC),
        config,
    )
    .await?;
    Ok(texts.first().cloned().unwrap_or_else(|| "sent".into()))
}

/// Poll JingME replies through JoyClaw (or fixture).
pub async fn read_replies(
    chat_name: &str,
    since: &str,
    timeout_sec: Option<u64>,
    config: Option<&JoyClawBridgeConfig>,
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    if is_jme_fixture() {
        let injected = std::env::var("PORIA_JME_FIXTURE_REPLIES").unwrap_or_default();
        if injected.trim().is_empty() {
            return Ok(Vec::new());
        }
        return Ok(injected
            .split('|')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned)
            .collect());
    }
    let port = gateway_port(config);
    if !check_gateway_alive(port).await {
        return Err(format!("JoyClaw gateway not alive on :{port}").into());
    }
    let prompt = format!("查看 {chat_name} 聊天中 {since} 之后的消息");
    run_joyclaw_agent(
        &prompt,
        timeout_sec.unwrap_or(DEFAULT_AGENT_TIMEOUT_SEC),
        config,
    )
    .await
}

async fn run_joyclaw_agent(
    message: &str,
    timeout_sec: u64,
    config: Option<&JoyClawBridgeConfig>,
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    let (node, openclaw) = resolve_joyclaw_paths(config)?;
    let timeout = Duration::from_secs(timeout_sec.max(1));
    let child = Command::new(&node)
        .args([
            openclaw.as_str(),
            "agent",
            "--agent",
            "main",
            "--timeout",
            &timeout_sec.to_string(),
            "--json",
            "--message",
            message,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("spawn JoyClaw failed ({node}): {e}"))?;
    let output = tokio::time::timeout(timeout, child.wait_with_output())
        .await
        .map_err(|_| "JoyClaw agent timed out".to_string())?
        .map_err(|e| format!("JoyClaw agent failed: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("JoyClaw agent exited {}: {}", output.status, stderr.trim()).into());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_agent_output(&stdout))
}

fn resolve_joyclaw_paths(
    config: Option<&JoyClawBridgeConfig>,
) -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>> {
    let home = std::env::var("HOME").unwrap_or_default();
    let node = config
        .and_then(|c| c.node_path.as_deref())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| std::env::var("JOYCLAW_NODE").ok().filter(|s| !s.is_empty()))
        .unwrap_or_else(|| default_node_path(&home));
    let openclaw = config
        .and_then(|c| c.openclaw_path.as_deref())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            std::env::var("JOYCLAW_OPENCLAW")
                .ok()
                .filter(|s| !s.is_empty())
        })
        .unwrap_or_else(|| default_openclaw_path(&home));
    if !Path::new(&openclaw).is_file() {
        return Err(format!("JoyClaw openclaw.mjs not found: {openclaw}").into());
    }
    Ok((node, openclaw))
}

fn default_node_path(home: &str) -> String {
    let pinned = PathBuf::from(home).join(".joyclaw/node/node-v22.16.0-darwin-arm64/bin/node");
    if pinned.is_file() {
        return pinned.to_string_lossy().into_owned();
    }
    "node".into()
}

fn default_openclaw_path(home: &str) -> String {
    let pinned = PathBuf::from(home).join(".joyclaw/apps/v2.5.6/openclaw.mjs");
    if pinned.is_file() {
        return pinned.to_string_lossy().into_owned();
    }
    let apps = PathBuf::from(home).join(".joyclaw/apps");
    if let Ok(entries) = std::fs::read_dir(&apps) {
        let mut versions: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path().join("openclaw.mjs"))
            .filter(|p| p.is_file())
            .collect();
        versions.sort();
        if let Some(path) = versions.last() {
            return path.to_string_lossy().into_owned();
        }
    }
    pinned.to_string_lossy().into_owned()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fixture_send_is_not_placeholder() {
        std::env::set_var("PORIA_JME_FIXTURE", "1");
        let result = send_message("product", "P0 unanswered", None, None)
            .await
            .unwrap();
        std::env::remove_var("PORIA_JME_FIXTURE");
        assert!(result.starts_with("fixture sent to product"));
        assert!(!result.contains("Would send"));
    }

    #[tokio::test]
    async fn fixture_read_uses_injected_replies() {
        std::env::set_var("PORIA_JME_FIXTURE", "1");
        std::env::set_var("PORIA_JME_FIXTURE_REPLIES", "已修复|ok");
        let replies = read_replies("erp.li", "2026-01-01T00:00:00Z", None, None)
            .await
            .unwrap();
        std::env::remove_var("PORIA_JME_FIXTURE_REPLIES");
        std::env::remove_var("PORIA_JME_FIXTURE");
        assert_eq!(replies, vec!["已修复", "ok"]);
    }
}
