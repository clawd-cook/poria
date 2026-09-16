use async_trait::async_trait;
use serde_json::json;

use poria_core::contracts::{CapabilityMetadata, Skill, SkillContext};
use poria_core::types::{SkillInput, SkillOutput};

use crate::error::SkillError;
use crate::fixture::is_fixture_mode;

pub struct CodeReviewSkill {
    metadata: CapabilityMetadata,
}

impl CodeReviewSkill {
    pub fn new() -> Self {
        Self {
            metadata: CapabilityMetadata {
                id: "skill:code-review".into(),
                name: "CodeReview".into(),
                description: "Agent-driven code review with security scan".into(),
                version: "0.1.0".into(),
            },
        }
    }
}

impl Default for CodeReviewSkill {
    fn default() -> Self {
        Self::new()
    }
}

fn fixture_output() -> SkillOutput {
    SkillOutput {
        output: json!({
            "crReportPath": "/tmp/poria-fixture/project/CR_REPORT.md",
            "crScore": "A",
            "findings": [],
            "securityScanPassed": true
        }),
        gates_pass: None,
    }
}

#[async_trait]
impl Skill for CodeReviewSkill {
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
            "CodeReviewSkill".into(),
        )))
    }
}
