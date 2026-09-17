use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;

use poria_core::contracts::{CapabilityMetadata, Skill, SkillContext};
use poria_core::feature_context::{FeatureContext, ARTIFACT_PRD, ARTIFACT_PRD_REVIEW};
use poria_core::types::{AgentTaskInput, SkillInput, SkillOutput};
use poria_resources::ClaudeAgentPool;

use crate::error::SkillError;
use crate::fixture::is_fixture_mode;
use crate::prompt_templates::{render_prompt, PRD_REVIEW_PROMPT};

pub struct ReviewPrdSkill {
    metadata: CapabilityMetadata,
    agent_pool: Option<Arc<ClaudeAgentPool>>,
}

impl ReviewPrdSkill {
    pub fn new() -> Self {
        Self {
            metadata: CapabilityMetadata {
                id: "skill:review-prd".into(),
                name: "ReviewPRD".into(),
                description: "Analyze PRD and produce PRD_REVIEW.md with P0/P1/P2 questions".into(),
                version: "0.1.0".into(),
            },
            agent_pool: None,
        }
    }

    pub fn with_agent_pool(mut self, pool: Arc<ClaudeAgentPool>) -> Self {
        self.agent_pool = Some(pool);
        self
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
        input: SkillInput,
        ctx: SkillContext,
    ) -> Result<SkillOutput, Box<dyn std::error::Error + Send + Sync>> {
        if is_fixture_mode() {
            return Ok(fixture_output());
        }

        let agent_pool = self
            .agent_pool
            .as_ref()
            .ok_or_else(|| SkillError::NotImplemented("ReviewPrdSkill: no agent pool".into()))?;

        let feature_dir = input
            .extra
            .get("feature_dir")
            .and_then(|v| v.as_str())
            .ok_or("missing feature_dir in skill input")?;

        let feature_ctx = FeatureContext::from_root(std::path::Path::new(feature_dir))
            .ok_or("feature context not found")?;

        let prd_content = feature_ctx
            .read_artifact(ARTIFACT_PRD)?
            .ok_or("PRD.md not found in feature context")?;

        let mut vars = HashMap::new();
        vars.insert("feature_dir".into(), feature_dir.to_string());
        vars.insert("project_root".into(), ctx.workdir.clone());
        vars.insert("prd_content".into(), prd_content);
        vars.insert("prd_source".into(), "file".into());

        let system_prompt = render_prompt(PRD_REVIEW_PROMPT, &vars);

        let agent_input = AgentTaskInput {
            prompt: "分析 PRD，从前端视角进行需求澄清，生成 PRD_REVIEW.md".into(),
            worktree_path: ctx.workdir.clone(),
            system_prompt: Some(system_prompt),
            model: None,
            max_budget_usd: Some(1.0),
            max_turns: Some(10),
            timeout_ms: Some(5 * 60_000),
            extra_tools: Some(vec!["Read".into(), "Write".into(), "Grep".into()]),
        };

        let result = agent_pool.dispatch(agent_input).await;

        if !result.success {
            return Err(format!(
                "Agent dispatch failed: {}",
                result.error.unwrap_or_default()
            )
            .into());
        }

        let review_exists = feature_ctx.has_artifact(ARTIFACT_PRD_REVIEW);

        Ok(SkillOutput {
            output: json!({
                "reviewPath": feature_ctx.artifact_path(ARTIFACT_PRD_REVIEW),
                "reviewExists": review_exists,
                "agentSessionId": result.session_id,
                "costUsd": result.cost_usd,
            }),
            gates_pass: Some(review_exists),
        })
    }
}
