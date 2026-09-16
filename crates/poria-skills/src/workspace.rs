use async_trait::async_trait;
use serde_json::json;

use poria_core::contracts::{CapabilityMetadata, Skill, SkillContext};
use poria_core::types::{SkillInput, SkillOutput};

use crate::error::SkillError;
use crate::fixture::is_fixture_mode;

pub struct WorkspaceSkill {
    metadata: CapabilityMetadata,
}

impl WorkspaceSkill {
    pub fn new() -> Self {
        Self {
            metadata: CapabilityMetadata {
                id: "skill:workspace".into(),
                name: "Workspace".into(),
                description: "Create git worktree and bind Xingyun branch".into(),
                version: "0.1.0".into(),
            },
        }
    }
}

impl Default for WorkspaceSkill {
    fn default() -> Self {
        Self::new()
    }
}

fn fixture_output() -> SkillOutput {
    SkillOutput {
        output: json!({
            "repos": [
                {
                    "name": "main",
                    "branch": "feature_TEST",
                    "baseBranch": "master",
                    "changeId": "chg-fixture-001",
                    "worktreePath": "/tmp/poria-fixture/worktree",
                    "gitlabProjectPath": "group/repo"
                }
            ]
        }),
        gates_pass: None,
    }
}

#[async_trait]
impl Skill for WorkspaceSkill {
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
            "WorkspaceSkill".into(),
        )))
    }
}
