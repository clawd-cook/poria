use async_trait::async_trait;
use serde_json::json;

use poria_core::contracts::{CapabilityMetadata, Skill, SkillContext};
use poria_core::types::{SkillInput, SkillOutput};

use crate::error::SkillError;
use crate::fixture::is_fixture_mode;

pub struct DeploySkill {
    metadata: CapabilityMetadata,
}

impl DeploySkill {
    pub fn new() -> Self {
        Self {
            metadata: CapabilityMetadata {
                id: "skill:deploy".into(),
                name: "Deploy".into(),
                description: "Build, push, and create idempotent merge request".into(),
                version: "0.1.0".into(),
            },
        }
    }
}

impl Default for DeploySkill {
    fn default() -> Self {
        Self::new()
    }
}

fn fixture_output() -> SkillOutput {
    SkillOutput {
        output: json!({
            "mrUrl": "https://coding.jd.com/group/repo/-/merge_requests/1",
            "mrIid": 1,
            "repo": "main"
        }),
        gates_pass: None,
    }
}

#[async_trait]
impl Skill for DeploySkill {
    fn metadata(&self) -> &CapabilityMetadata {
        &self.metadata
    }

    async fn execute(
        &self,
        _input: SkillInput,
        _ctx: SkillContext,
    ) -> Result<SkillOutput, Box<dyn std::error::Error + Send + Sync>> {
        if is_fixture_mode() {
            return Ok(fixture_output());
        }
        Err(Box::new(SkillError::NotImplemented("DeploySkill".into())))
    }
}
