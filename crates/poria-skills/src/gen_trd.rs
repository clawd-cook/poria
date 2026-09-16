use async_trait::async_trait;
use serde_json::json;

use poria_core::contracts::{CapabilityMetadata, Skill, SkillContext};
use poria_core::types::{SkillInput, SkillOutput};

use crate::error::SkillError;
use crate::fixture::is_fixture_mode;

pub struct GenTrdSkill {
    metadata: CapabilityMetadata,
}

impl GenTrdSkill {
    pub fn new() -> Self {
        Self {
            metadata: CapabilityMetadata {
                id: "skill:gen-trd".into(),
                name: "GenTRD".into(),
                description: "Generate TRD.md and extract allowed modification scope".into(),
                version: "0.1.0".into(),
            },
        }
    }
}

impl Default for GenTrdSkill {
    fn default() -> Self {
        Self::new()
    }
}

fn fixture_output() -> SkillOutput {
    SkillOutput {
        output: json!({
            "trdPath": "/tmp/poria-fixture/project/source/TRD.md",
            "trdScope": ["src/**/*.ts", "package.json"]
        }),
        gates_pass: None,
    }
}

#[async_trait]
impl Skill for GenTrdSkill {
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
        Err(Box::new(SkillError::NotImplemented("GenTrdSkill".into())))
    }
}
