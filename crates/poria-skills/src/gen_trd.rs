use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;

use poria_core::contracts::{CapabilityMetadata, Skill, SkillContext};
use poria_core::feature_context::{FeatureContext, ARTIFACT_PRD, ARTIFACT_TRD};
use poria_core::pipeline::parse_trd_scope;
use poria_core::types::{AgentTaskInput, SkillInput, SkillOutput};
use poria_resources::ClaudeAgentPool;

use crate::artifacts::adopt_and_remove_from_worktree;
use crate::claude_prompt::{
    advance_note_from_input, backend_dir, backend_trd_url, build_claude_skill_prompt_with_note,
    extra_nonempty, frontend_base_branch,
    resolve_feature_dir, resolve_workspace_cwd, SKILL_GEN_TRD,
};
use crate::error::SkillError;
use crate::fixture::is_fixture_mode;

pub struct GenTrdSkill {
    metadata: CapabilityMetadata,
    agent_pool: Option<Arc<ClaudeAgentPool>>,
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
            agent_pool: None,
        }
    }

    pub fn with_agent_pool(mut self, pool: Arc<ClaudeAgentPool>) -> Self {
        self.agent_pool = Some(pool);
        self
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
        input: SkillInput,
        ctx: SkillContext,
    ) -> Result<SkillOutput, Box<dyn std::error::Error + Send + Sync>> {
        if is_fixture_mode() {
            return Ok(fixture_output());
        }

        let agent_pool = self
            .agent_pool
            .as_ref()
            .ok_or_else(|| SkillError::NotImplemented("GenTrdSkill: no agent pool".into()))?;

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
            return Err("PRD.md not found".into());
        }

        let prompt = build_claude_skill_prompt_with_note(
            SKILL_GEN_TRD,
            &input.pipeline.demand_code,
            &workspace_path,
            &frontend_worktree,
            &backend,
            backend_trd_url(&input),
            &frontend_base_branch(&input),
            advance_note_from_input(&input),
        );

        let agent_input = AgentTaskInput {
            prompt,
            worktree_path: workspace_path,
            system_prompt: None,
            model: None,
            max_budget_usd: Some(3.0),
            max_turns: Some(20),
            timeout_ms: Some(12 * 60_000),
            extra_tools: Some(vec![
                "Read".into(),
                "Write".into(),
                "Edit".into(),
                "Grep".into(),
            ]),
        };

        let result = agent_pool.dispatch(agent_input).await;
        adopt_and_remove_from_worktree(&feature_ctx, &frontend_worktree, ARTIFACT_TRD);
        let trd_exists = feature_ctx.has_artifact(ARTIFACT_TRD);
        if !result.success && !trd_exists {
            return Err(format!(
                "Agent dispatch failed: {}",
                result.error.unwrap_or_default()
            )
            .into());
        }
        if !trd_exists {
            return Err("TRD.md was not written".into());
        }

        let trd_scope = feature_ctx
            .read_artifact(ARTIFACT_TRD)?
            .map(|content| parse_trd_scope(&content))
            .unwrap_or_default();

        Ok(SkillOutput {
            output: json!({
                "trdPath": feature_ctx.artifact_path(ARTIFACT_TRD),
                "trdExists": true,
                "trdScope": trd_scope,
                "agentSessionId": result.session_id,
                "costUsd": result.cost_usd,
            }),
            gates_pass: Some(true),
        })
    }
}
