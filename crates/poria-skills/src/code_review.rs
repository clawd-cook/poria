use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;

use poria_core::contracts::{CapabilityMetadata, Skill, SkillContext};
use poria_core::feature_context::{FeatureContext, ARTIFACT_CR};
use poria_core::types::{AgentTaskInput, SkillInput, SkillOutput};
use poria_resources::ClaudeAgentPool;

use crate::error::SkillError;
use crate::fixture::is_fixture_mode;
use crate::prompt_templates::{render_prompt, CR_WEB_PROMPT};

pub struct CodeReviewSkill {
    metadata: CapabilityMetadata,
    agent_pool: Option<Arc<ClaudeAgentPool>>,
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
            agent_pool: None,
        }
    }

    pub fn with_agent_pool(mut self, pool: Arc<ClaudeAgentPool>) -> Self {
        self.agent_pool = Some(pool);
        self
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
        input: SkillInput,
        ctx: SkillContext,
    ) -> Result<SkillOutput, Box<dyn std::error::Error + Send + Sync>> {
        if is_fixture_mode() {
            return Ok(fixture_output());
        }

        let agent_pool = self
            .agent_pool
            .as_ref()
            .ok_or_else(|| SkillError::NotImplemented("CodeReviewSkill: no agent pool".into()))?;

        let feature_dir = input
            .extra
            .get("feature_dir")
            .and_then(|v| v.as_str())
            .ok_or("missing feature_dir in skill input")?;

        let feature_ctx = FeatureContext::from_root(std::path::Path::new(feature_dir))
            .ok_or("feature context not found")?;

        let git_diff = input
            .extra
            .get("git_diff")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let base_branch = input
            .extra
            .get("base_branch")
            .and_then(|v| v.as_str())
            .unwrap_or("main");

        let target_branch = input
            .extra
            .get("target_branch")
            .and_then(|v| v.as_str())
            .unwrap_or("feature");

        let mut vars = HashMap::new();
        vars.insert("feature_dir".into(), feature_dir.to_string());
        vars.insert("project_root".into(), ctx.workdir.clone());
        vars.insert("git_diff".into(), git_diff.to_string());
        vars.insert("base_branch".into(), base_branch.to_string());
        vars.insert("target_branch".into(), target_branch.to_string());

        let system_prompt = render_prompt(CR_WEB_PROMPT, &vars);

        let agent_input = AgentTaskInput {
            prompt: "对代码变更进行 architecture-first 代码审查，生成 CR 报告".into(),
            worktree_path: ctx.workdir.clone(),
            system_prompt: Some(system_prompt),
            model: None,
            max_budget_usd: Some(5.0),
            max_turns: Some(30),
            timeout_ms: Some(15 * 60_000),
            extra_tools: Some(vec!["Read".into(), "Bash".into(), "Grep".into()]),
        };

        let result = agent_pool.dispatch(agent_input).await;

        if !result.success {
            return Err(format!(
                "Agent dispatch failed: {}",
                result.error.unwrap_or_default()
            )
            .into());
        }

        let cr_exists = feature_ctx.has_artifact(ARTIFACT_CR);

        // Try to extract CR score from agent output
        let cr_score = result
            .result
            .as_deref()
            .and_then(|r| {
                // Look for score pattern in the result text
                for grade in ["S", "A", "B+", "B", "C", "D"] {
                    if r.contains(&format!("评分: {}", grade))
                        || r.contains(&format!("Score: {}", grade))
                        || r.contains(&format!("建议: {}", grade))
                    {
                        return Some(grade.to_string());
                    }
                }
                None
            })
            .unwrap_or_else(|| "B+".into());

        Ok(SkillOutput {
            output: json!({
                "crReportPath": feature_ctx.artifact_path(ARTIFACT_CR),
                "crExists": cr_exists,
                "crScore": cr_score,
                "agentSessionId": result.session_id,
                "costUsd": result.cost_usd,
            }),
            gates_pass: Some(cr_exists),
        })
    }
}
