use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityMetadata {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
}

#[derive(Debug, Clone)]
pub struct ChannelContext {
    pub credentials: serde_json::Value,
    pub pipeline_id: Option<String>,
}

#[async_trait]
pub trait Channel: Send + Sync {
    fn metadata(&self) -> &CapabilityMetadata;
    async fn execute(
        &self,
        input: serde_json::Value,
        ctx: ChannelContext,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>>;
}
