use async_trait::async_trait;

use super::channel::CapabilityMetadata;
use crate::types::{SkillInput, SkillOutput};

#[derive(Debug, Clone)]
pub struct SkillContext {
    pub pipeline_id: String,
    pub workdir: String,
    pub credentials: serde_json::Value,
}

#[async_trait]
pub trait Skill: Send + Sync {
    fn metadata(&self) -> &CapabilityMetadata;
    async fn execute(
        &self,
        input: SkillInput,
        ctx: SkillContext,
    ) -> Result<SkillOutput, Box<dyn std::error::Error + Send + Sync>>;
}
