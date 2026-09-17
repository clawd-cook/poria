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
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .or_else(|| {
                let workdir = ctx.workdir.trim();
                if workdir.is_empty() {
                    None
                } else {
                    Some(workdir.to_string())
                }
            })
            .or_else(|| input.pipeline.config.project_dir.clone())
            .ok_or("missing feature_dir in skill input")?;

        let feature_ctx = FeatureContext::from_root(std::path::Path::new(&feature_dir))
            .ok_or("feature context not found")?;

        let prd_content = feature_ctx
            .read_artifact(ARTIFACT_PRD)?
            .ok_or("PRD.md not found")?;

        let prd_review_content = feature_ctx
            .read_artifact(ARTIFACT_PRD_REVIEW)?
            .unwrap_or_default();

        let title = input
            .pipeline
            .demand_name
            .clone()
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| input.pipeline.demand_code.clone());
        let repo_name = input
            .pipeline
            .config
            .repos
            .first()
            .map(|repo| repo.name.clone())
            .unwrap_or_default();

        let mut vars = HashMap::new();
        vars.insert("feature_dir".into(), feature_dir.clone());
        vars.insert("project_root".into(), ctx.workdir.clone());
        vars.insert("prd_content".into(), prd_content);
        vars.insert("prd_review_content".into(), prd_review_content);
        vars.insert("title".into(), title);
        vars.insert("repo_name".into(), repo_name);
        insert_backend_coding_aid_vars(&mut vars, &input.pipeline.config, &feature_ctx);

        let system_prompt = format!(
            "{rendered}\n\n## 桌面端非交互覆盖（优先于上文任何等待指令）\n\
             没有用户可以回复。禁止提问。默认本期做完 PRD 前端可见项（明确二期/不做除外）。\n\
             无 API.md 则用「仅高保真 UI」。grilling 用推荐答案写入附录 B。\n\
             禁止读取 backend_repo_path 下任何源码，禁止派生子 agent / Explore / Bash。\n\
             第一个工具调用必须是 Write `TRD.md`。",
            rendered = render_prompt(TRD_GEN_PROMPT, &vars)
        );

        let agent_input = AgentTaskInput {
            prompt: "这是桌面端非交互执行。材料已在系统提示中。不要读后端仓、不要子 agent、不要 Bash。第一个工具调用必须 Write TRD.md。grilling 推荐答案写入附录 B。禁止改后端仓，不得覆盖或改名前端 TRD.md。".into(),
            worktree_path: ctx.workdir.clone(),
            system_prompt: Some(system_prompt),
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

        Ok(SkillOutput {
            output: json!({
                "trdPath": feature_ctx.artifact_path(ARTIFACT_TRD),
                "trdExists": true,
                "agentSessionId": result.session_id,
                "costUsd": result.cost_usd,
            }),
            gates_pass: Some(true),
        })
    }
}
