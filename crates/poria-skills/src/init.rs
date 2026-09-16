use async_trait::async_trait;
use serde_json::json;

use poria_core::contracts::{CapabilityMetadata, Skill, SkillContext};
use poria_core::types::{SkillInput, SkillOutput};

use crate::error::SkillError;
use crate::fixture::is_fixture_mode;

pub struct InitSkill {
    metadata: CapabilityMetadata,
}

impl InitSkill {
    pub fn new() -> Self {
        Self {
            metadata: CapabilityMetadata {
                id: "skill:init".into(),
                name: "Init".into(),
                description: "Parse demand link and export PRD from JoySpace".into(),
                version: "0.1.0".into(),
            },
        }
    }
}

impl Default for InitSkill {
    fn default() -> Self {
        Self::new()
    }
}

fn fixture_output() -> SkillOutput {
    SkillOutput {
        output: json!({
            "projectDir": "/tmp/poria-fixture/project",
            "prdPath": "/tmp/poria-fixture/project/source/PRD.md",
            "prdTitle": "Fixture PRD",
            "demandMetadata": {
                "demandId": 1,
                "demandCode": "TEST",
                "name": "Fixture Demand",
                "prdUrl": "http://fixture",
                "attachments": [],
                "rawLink": "http://fixture"
            }
        }),
        gates_pass: None,
    }
}

#[async_trait]
impl Skill for InitSkill {
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
        Err(Box::new(SkillError::NotImplemented("InitSkill".into())))
    }
}
