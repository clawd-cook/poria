use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;

use poria_core::contracts::{CapabilityMetadata, Skill, SkillContext};
use poria_core::feature_context::{FeatureContext, ARTIFACT_PRD, ARTIFACT_PRD_REVIEW};
use poria_core::types::{AgentTaskInput, SkillInput, SkillOutput};
use poria_resources::ClaudeAgentPool;

use crate::artifacts::adopt_and_remove_from_worktree;
use crate::backend_aid::insert_backend_coding_aid_vars;
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
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .or_else(|| input.pipeline.config.project_dir.clone())
            .ok_or("missing feature_dir in skill input")?;

        let feature_ctx = FeatureContext::from_root(std::path::Path::new(&feature_dir))
            .ok_or("feature context not found")?;

        let prd_content = feature_ctx
            .read_artifact(ARTIFACT_PRD)?
            .ok_or("PRD.md not found in feature context")?;

        let prd_source = input
            .pipeline
            .config
            .prd_url
            .clone()
            .filter(|url| !url.trim().is_empty())
            .unwrap_or_else(|| "file".into());
        let title = input
            .pipeline
            .demand_name
            .clone()
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| input.pipeline.demand_code.clone());
        let reviewed_at = input
            .extra
            .get("reviewed_at")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let mut vars = HashMap::new();
        vars.insert("feature_dir".into(), feature_dir.clone());
        vars.insert("project_root".into(), ctx.workdir.clone());
        vars.insert("prd_content".into(), prd_content.clone());
        vars.insert("prd_source".into(), prd_source);
        vars.insert("title".into(), title);
        vars.insert("reviewed_at".into(), reviewed_at);
        insert_backend_coding_aid_vars(&mut vars, &input.pipeline.config, &feature_ctx);

        let system_prompt = format!(
            "{system_prompt}\n\n## 桌面端非交互覆盖（优先于上文任何等待指令）\n\
             没有用户可以回复。禁止提问、禁止等待范围确认。默认全部模块纳入本期（PRD 明确写二期/不做的除外）。\n\
             必须用 Write 工具把完整 `PRD_REVIEW.md` 写到 `{feature_dir}/PRD_REVIEW.md`（绝对路径）。\n\
             禁止把 PRD_REVIEW.md 写到 git 工作区根目录。可以阅读前端代码和后端只读 worktree。`An` 行留空。",
            system_prompt = render_prompt(PRD_REVIEW_PROMPT, &vars),
            feature_dir = feature_dir
        );

        let prompt = format!(
            "这是桌面端非交互执行。默认全部模块纳入本期（PRD 明确写二期/不做的除外），不要停下来等用户确认范围。\
             根据下面的 PRD 从前端视角生成 PRD_REVIEW.md：An 行留空给产品回填。读完即可 Write 到 `{feature_dir}/PRD_REVIEW.md`，不要追问。\n\n\
             # PRD.md\n\n{prd_content}"
        );

        let agent_input = AgentTaskInput {
            prompt,
            worktree_path: ctx.workdir.clone(),
            system_prompt: Some(system_prompt),
            model: None,
            max_budget_usd: Some(1.0),
            max_turns: Some(10),
            timeout_ms: Some(12 * 60_000),
            extra_tools: Some(vec!["Read".into(), "Write".into(), "Grep".into(), "Glob".into()]),
        };

        let review_already_exists = feature_ctx.has_artifact(ARTIFACT_PRD_REVIEW);
        let (session_id, cost_usd) = if review_already_exists {
            (None, None)
        } else {
            let result = agent_pool.dispatch(agent_input).await;
            adopt_and_remove_from_worktree(&feature_ctx, &ctx.workdir, ARTIFACT_PRD_REVIEW);
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

        adopt_and_remove_from_worktree(&feature_ctx, &ctx.workdir, ARTIFACT_PRD_REVIEW);

        let review_exists = feature_ctx.has_artifact(ARTIFACT_PRD_REVIEW);
        if !review_exists {
            return Err("PRD_REVIEW.md was not written".into());
        }

        Ok(SkillOutput {
            output: json!({
                "reviewPath": feature_ctx.artifact_path(ARTIFACT_PRD_REVIEW),
                "reviewExists": review_exists,
                "agentSessionId": session_id,
                "costUsd": cost_usd,
            }),
            gates_pass: Some(true),
        })
    }
}
