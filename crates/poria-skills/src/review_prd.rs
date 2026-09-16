use async_trait::async_trait;
use serde_json::json;

use poria_core::contracts::{CapabilityMetadata, Skill, SkillContext};
use poria_core::types::{SkillInput, SkillOutput};

use crate::error::SkillError;
use crate::fixture::is_fixture_mode;

pub struct ReviewPrdSkill {
    metadata: CapabilityMetadata,
}

impl ReviewPrdSkill {
    pub fn new() -> Self {
        Self {
            metadata: CapabilityMetadata {
                id: "skill:review-prd".into(),
                name: "ReviewPRD".into(),
                description: "Analyze PRD and produce PRD_REVIEW.md with P0/P1/P2 questions"
                    .into(),
                version: "0.1.0".into(),
            },
        }
    }
}

impl Default for ReviewPrdSkill {
    fn default() -> Self {
        Self::new()
    }
}

fn fixture_output() -> SkillOutput {
    SkillOutput {
        output: json!({
            "reviewPath": "/tmp/poria-fixture/project/source/PRD_REVIEW.md",
            "p0Answered": true,
            "p1Answered": false,
            "p2Answered": false
        }),
        gates_pass: None,
    }
}

#[async_trait]
impl Skill for ReviewPrdSkill {
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
        Err(Box::new(SkillError::NotImplemented(
            "ReviewPrdSkill".into(),
        )))
    }
}
