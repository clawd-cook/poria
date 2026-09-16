use async_trait::async_trait;

use super::channel::CapabilityMetadata;

#[derive(Debug, Clone)]
pub struct CommandContext {
    pub operator: String,
}

#[async_trait]
pub trait Command: Send + Sync {
    fn metadata(&self) -> &CapabilityMetadata;
    async fn execute(
        &self,
        args: serde_json::Value,
        ctx: CommandContext,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>>;
}
