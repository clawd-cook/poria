//! Defect channel -- placeholder for P1.
//! Defect management integration will be implemented in a future phase.

use async_trait::async_trait;
use poria_core::contracts::{CapabilityMetadata, Channel, ChannelContext};
use serde::{Deserialize, Serialize};

/// Placeholder constant indicating the defect channel exists but is not yet implemented.
pub const DEFECT_CHANNEL_PLACEHOLDER: bool = true;

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
}

/// Output from the Defect channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefectChannelOutput {
    pub action: String,
    pub message: String,
}

/// Creates a new Defect channel instance.
pub fn create_defect_channel() -> DefectChannel {
    DefectChannel {
        metadata: CapabilityMetadata {
            id: "channel:defect".into(),
            name: "Defect".into(),
            description: "Defect management (placeholder for P1)".into(),
            version: "0.1.0".into(),
        },
    }
}

pub struct DefectChannel {
    metadata: CapabilityMetadata,
}

#[async_trait]
impl Channel for DefectChannel {
    fn metadata(&self) -> &CapabilityMetadata {
        &self.metadata
    }

    async fn execute(
        &self,
        _input: serde_json::Value,
        _ctx: ChannelContext,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        Err("Defect channel is not yet implemented (planned for P1)".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_constant_is_true() {
        assert!(DEFECT_CHANNEL_PLACEHOLDER);
    }

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
        };
        let json = serde_json::to_value(&input).unwrap();
        assert_eq!(json["action"], "listDefects");
    }
}
