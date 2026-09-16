use async_trait::async_trait;

use super::channel::CapabilityMetadata;

#[derive(Debug, Clone)]
pub struct ResourceContext {
    pub pipeline_id: Option<String>,
    pub workdir: Option<String>,
}

#[async_trait]
pub trait Resource: Send + Sync {
    fn metadata(&self) -> &CapabilityMetadata;
    async fn execute(
        &self,
        input: serde_json::Value,
        ctx: ResourceContext,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>>;
}
