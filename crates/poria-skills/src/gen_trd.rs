use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;

use poria_core::contracts::{CapabilityMetadata, Skill, SkillContext};
use poria_core::feature_context::{
    FeatureContext, ARTIFACT_PRD, ARTIFACT_PRD_REVIEW, ARTIFACT_TRD,
};
use poria_core::types::{AgentTaskInput, SkillInput, SkillOutput};
use poria_resources::ClaudeAgentPool;

use crate::backend_aid::insert_backend_coding_aid_vars;
use crate::error::SkillError;
use crate::fixture::is_fixture_mode;
use crate::prompt_templates::{render_prompt, TRD_GEN_PROMPT};

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

        let feature_dir = input
            .extra
            .get("feature_dir")
            .and_then(|v| v.as_str())
            .ok_or("missing feature_dir in skill input")?;

        let feature_ctx = FeatureContext::from_root(std::path::Path::new(feature_dir))
            .ok_or("feature context not found")?;

        let prd_content = feature_ctx
            .read_artifact(ARTIFACT_PRD)?
            .ok_or("PRD.md not found")?;

        let prd_review_content = feature_ctx
            .read_artifact(ARTIFACT_PRD_REVIEW)?
            .unwrap_or_default();

        let mut vars = HashMap::new();
        vars.insert("feature_dir".into(), feature_dir.to_string());
        vars.insert("project_root".into(), ctx.workdir.clone());
        vars.insert("prd_content".into(), prd_content);
        vars.insert("prd_review_content".into(), prd_review_content);
        insert_backend_coding_aid_vars(&mut vars, &input.pipeline.config, &feature_ctx);

        let system_prompt = render_prompt(TRD_GEN_PROMPT, &vars);

        let agent_input = AgentTaskInput {
            prompt: "基于 PRD 和 PRD_REVIEW 生成前端技术设计文档 TRD.md。后端 TRD 与后端仓仅作只读参考，禁止改后端仓，不得覆盖前端 TRD.md。".into(),
            worktree_path: ctx.workdir.clone(),
            system_prompt: Some(system_prompt),
            model: None,
            max_budget_usd: Some(3.0),
            max_turns: Some(20),
            timeout_ms: Some(10 * 60_000),
            extra_tools: Some(vec![
                "Read".into(),
                "Write".into(),
                "Edit".into(),
                "Grep".into(),
            ]),
        };

        let result = agent_pool.dispatch(agent_input).await;

        if !result.success {
            return Err(format!(
                "Agent dispatch failed: {}",
                result.error.unwrap_or_default()
            )
            .into());
        }

        let trd_exists = feature_ctx.has_artifact(ARTIFACT_TRD);

        Ok(SkillOutput {
            output: json!({
                "trdPath": feature_ctx.artifact_path(ARTIFACT_TRD),
                "trdExists": trd_exists,
                "agentSessionId": result.session_id,
                "costUsd": result.cost_usd,
            }),
            gates_pass: Some(trd_exists),
        })
    }
}
