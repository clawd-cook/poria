use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;

use poria_core::contracts::{CapabilityMetadata, Skill, SkillContext};
use poria_core::feature_context::{FeatureContext, ARTIFACT_PRD, ARTIFACT_TASK, ARTIFACT_TRD};
use poria_core::types::{AgentTaskInput, SkillInput, SkillOutput};
use poria_resources::ClaudeAgentPool;

use crate::backend_aid::insert_backend_coding_aid_vars;
use crate::error::SkillError;
use crate::fixture::is_fixture_mode;
use crate::prompt_templates::{render_prompt, CODE_IMPL_PROMPT};

pub struct GenCodeSkill {
    metadata: CapabilityMetadata,
    agent_pool: Option<Arc<ClaudeAgentPool>>,
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
            agent_pool: None,
        }
    }

    pub fn with_agent_pool(mut self, pool: Arc<ClaudeAgentPool>) -> Self {
        self.agent_pool = Some(pool);
        self
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
        input: SkillInput,
        ctx: SkillContext,
    ) -> Result<SkillOutput, Box<dyn std::error::Error + Send + Sync>> {
        if is_fixture_mode() {
            return Ok(fixture_output());
        }

        let agent_pool = self
            .agent_pool
            .as_ref()
            .ok_or_else(|| SkillError::NotImplemented("GenCodeSkill: no agent pool".into()))?;

        let feature_dir = input
            .extra
            .get("feature_dir")
            .and_then(|v| v.as_str())
            .ok_or("missing feature_dir in skill input")?;

        let feature_ctx = FeatureContext::from_root(std::path::Path::new(feature_dir))
            .ok_or("feature context not found")?;

        let trd_content = feature_ctx
            .read_artifact(ARTIFACT_TRD)?
            .ok_or("TRD.md not found")?;

        let prd_content = feature_ctx.read_artifact(ARTIFACT_PRD)?.unwrap_or_default();

        let mut vars = HashMap::new();
        vars.insert("feature_dir".into(), feature_dir.to_string());
        vars.insert("project_root".into(), ctx.workdir.clone());
        vars.insert("trd_content".into(), trd_content);
        vars.insert("prd_content".into(), prd_content);
        insert_backend_coding_aid_vars(&mut vars, &input.pipeline.config, &feature_ctx);

        let system_prompt = render_prompt(CODE_IMPL_PROMPT, &vars);

        let agent_input = AgentTaskInput {
            prompt: "基于前端 TRD 生成执行计划 TASK.md，然后按计划实现代码。后端 TRD 与后端仓仅作只读参考，禁止改后端仓。".into(),
            worktree_path: ctx.workdir.clone(),
            system_prompt: Some(system_prompt),
            model: None,
            max_budget_usd: Some(10.0),
            max_turns: Some(100),
            timeout_ms: Some(30 * 60_000),
            extra_tools: Some(vec![
                "Read".into(),
                "Write".into(),
                "Edit".into(),
                "Bash".into(),
                "Glob".into(),
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

        let task_exists = feature_ctx.has_artifact(ARTIFACT_TASK);

        Ok(SkillOutput {
            output: json!({
                "taskPath": feature_ctx.artifact_path(ARTIFACT_TASK),
                "taskExists": task_exists,
                "agentSessionId": result.session_id,
                "costUsd": result.cost_usd,
            }),
            gates_pass: Some(result.success),
        })
    }
}
