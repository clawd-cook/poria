use async_trait::async_trait;
use serde_json::json;

use poria_core::contracts::{CapabilityMetadata, Skill, SkillContext};
use poria_core::types::{SkillInput, SkillOutput};

use crate::error::SkillError;
use crate::fixture::is_fixture_mode;

pub struct GenCodeSkill {
    metadata: CapabilityMetadata,
}

impl GenCodeSkill {
    pub fn new() -> Self {
        Self {
            metadata: CapabilityMetadata {
                id: "skill:gen-code".into(),
                name: "GenCode".into(),
                description: "Agent-driven code generation with OutputGuard validation".into(),
                version: "0.1.0".into(),
            },
        }
    }
}

impl Default for GenCodeSkill {
    fn default() -> Self {
        Self::new()
    }
}

fn fixture_output() -> SkillOutput {
    SkillOutput {
        output: json!({
            "changedFiles": ["src/index.ts"],
            "totalDiffLines": 50,
            "agentSessionId": "session-fixture-001"
        }),
        gates_pass: None,
    }
}

#[async_trait]
impl Skill for GenCodeSkill {
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
        Err(Box::new(SkillError::NotImplemented("GenCodeSkill".into())))
    }
}
