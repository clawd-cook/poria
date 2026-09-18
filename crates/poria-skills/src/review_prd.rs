use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;

use poria_core::contracts::{CapabilityMetadata, Skill, SkillContext};
use poria_core::feature_context::{FeatureContext, ARTIFACT_PRD, ARTIFACT_PRD_REVIEW};
use poria_core::pipeline::parse_prd_review;
use poria_core::types::{AgentTaskInput, SkillInput, SkillOutput};
use poria_resources::ClaudeAgentPool;

use crate::artifacts::adopt_and_remove_from_worktree;
use crate::claude_prompt::{
    backend_dir, backend_trd_url, build_claude_skill_prompt, extra_nonempty, frontend_base_branch,
    resolve_feature_dir, resolve_workspace_cwd, SKILL_REVIEW_PRD,
};
use crate::error::SkillError;
use crate::fixture::is_fixture_mode;

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

        let feature_dir = resolve_feature_dir(&input)?;
        let workspace_path = resolve_workspace_cwd(&input, &ctx.workdir)?;
        let frontend_worktree = extra_nonempty(&input, "worktree_path")
            .map(str::to_string)
            .unwrap_or_else(|| workspace_path.clone());
        let backend = backend_dir(&input);

        let feature_ctx = FeatureContext::from_root(std::path::Path::new(&feature_dir))
            .ok_or("feature context not found")?;

        if feature_ctx
            .read_artifact(ARTIFACT_PRD)?
            .filter(|content| !content.trim().is_empty())
            .is_none()
        {
            return Err("PRD.md not found in feature context".into());
        }

        let prompt = build_claude_skill_prompt(
            SKILL_REVIEW_PRD,
            &input.pipeline.demand_code,
            &workspace_path,
            &frontend_worktree,
            &backend,
            backend_trd_url(&input),
            &frontend_base_branch(&input),
        );

        let agent_input = AgentTaskInput {
            prompt,
            worktree_path: workspace_path,
            system_prompt: None,
            model: None,
            max_budget_usd: Some(1.0),
            max_turns: Some(10),
            timeout_ms: Some(12 * 60_000),
            extra_tools: Some(vec![
                "Read".into(),
                "Write".into(),
                "Grep".into(),
                "Glob".into(),
            ]),
        };

        let review_already_exists = feature_ctx.has_artifact(ARTIFACT_PRD_REVIEW);
        let (session_id, cost_usd) = if review_already_exists {
            (None, None)
        } else {
            let result = agent_pool.dispatch(agent_input).await;
            adopt_and_remove_from_worktree(&feature_ctx, &frontend_worktree, ARTIFACT_PRD_REVIEW);
            let review_exists = feature_ctx.has_artifact(ARTIFACT_PRD_REVIEW);
            if !result.success && !review_exists {
                return Err(format!(
                    "Agent dispatch failed: {}",
                    result.error.unwrap_or_default()
                )
                .into());
            }
            (result.session_id, result.cost_usd)
        };

        adopt_and_remove_from_worktree(&feature_ctx, &frontend_worktree, ARTIFACT_PRD_REVIEW);

        let review_exists = feature_ctx.has_artifact(ARTIFACT_PRD_REVIEW);
        if !review_exists {
            return Err("PRD_REVIEW.md was not written".into());
        }

        let review_status = feature_ctx
            .read_artifact(ARTIFACT_PRD_REVIEW)?
            .map(|content| parse_prd_review(&content))
            .unwrap_or_default();

        Ok(SkillOutput {
            output: json!({
                "reviewPath": feature_ctx.artifact_path(ARTIFACT_PRD_REVIEW),
                "reviewExists": review_exists,
                "p0Answered": review_status.p0_done,
                "p1Answered": review_status.p1_done,
                "p2Answered": review_status.p2_done,
                "p0Unanswered": review_status.p0_unanswered,
                "p1Unanswered": review_status.p1_unanswered,
                "p2Unanswered": review_status.p2_unanswered,
                "agentSessionId": session_id,
                "costUsd": cost_usd,
            }),
            gates_pass: Some(true),
        })
    }
}
