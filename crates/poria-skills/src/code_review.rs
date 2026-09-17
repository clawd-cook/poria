use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;

use poria_core::contracts::{CapabilityMetadata, Skill, SkillContext};
use poria_core::feature_context::{FeatureContext, ARTIFACT_CR};
use poria_core::types::{AgentTaskInput, SkillInput, SkillOutput};
use poria_resources::{terminal, ClaudeAgentPool, TerminalExecInput};

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

fn extra_path<'a>(input: &'a SkillInput, key: &str) -> Option<&'a str> {
    input
        .extra
        .get(key)
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

async fn collect_git_diff(worktree: &str, base_branch: &str) -> String {
    let quoted_base = shell_single_quote(base_branch);
    let command = format!("git diff {quoted_base}");
    let from_base = terminal::exec(TerminalExecInput {
        command,
        cwd: Some(worktree.to_string()),
        env: None,
        timeout_ms: Some(30_000),
    })
    .await
    .map(|result| result.stdout)
    .unwrap_or_default();
    if !from_base.trim().is_empty() {
        return from_base;
    }
    terminal::exec(TerminalExecInput {
        command: "git diff".into(),
        cwd: Some(worktree.to_string()),
        env: None,
        timeout_ms: Some(15_000),
    })
    .await
    .map(|result| result.stdout)
    .unwrap_or_default()
}

fn extract_cr_score(text: &str) -> Option<String> {
    for grade in ["B+", "S", "A", "B", "C", "D"] {
        let patterns = [
            format!("评分: {grade}"),
            format!("评分：{grade}"),
            format!("Score: {grade}"),
            format!("建议: {grade}"),
            format!("建议：{grade}"),
        ];
        if patterns.iter().any(|pattern| text.contains(pattern)) {
            return Some(grade.to_string());
        }
    }
    if text.contains("有条件通过") {
        return Some("B+".into());
    }
    if text.contains("| 通过 |") || text.contains("建议 | 通过") {
        return Some("A".into());
    }
    None
}

fn adopt_cr_from_worktree(feature_ctx: &FeatureContext, worktree: &str) {
    if feature_ctx.has_artifact(ARTIFACT_CR) {
        return;
    }
    let candidate = Path::new(worktree).join(ARTIFACT_CR);
    if !candidate.is_file() {
        return;
    }
    if let Ok(content) = std::fs::read_to_string(&candidate) {
        let _ = feature_ctx.write_artifact(ARTIFACT_CR, &content);
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

        let feature_dir = extra_path(&input, "feature_dir")
            .map(str::to_string)
            .or_else(|| input.pipeline.config.project_dir.clone())
            .ok_or("missing feature_dir in skill input")?;
        let worktree_path = extra_path(&input, "worktree_path")
            .map(str::to_string)
            .or_else(|| {
                let workdir = ctx.workdir.trim();
                if workdir.is_empty() {
                    None
                } else {
                    Some(workdir.to_string())
                }
            })
            .ok_or("missing worktree_path in skill input")?;

        let feature_ctx = FeatureContext::from_root(Path::new(&feature_dir))
            .ok_or("feature context not found")?;

        let repo = input.pipeline.config.repos.first();
        let base_branch = extra_path(&input, "base_branch")
            .map(str::to_string)
            .or_else(|| repo.map(|item| item.base_branch.clone()))
            .unwrap_or_else(|| "master".into());
        let target_branch = extra_path(&input, "target_branch")
            .map(str::to_string)
            .or_else(|| repo.map(|item| item.branch.clone()))
            .unwrap_or_else(|| "feature".into());

        let git_diff = extra_path(&input, "git_diff")
            .map(str::to_string)
            .unwrap_or_else(String::new);
        let git_diff = if git_diff.trim().is_empty() {
            collect_git_diff(&worktree_path, &base_branch).await
        } else {
            git_diff
        };

        let mut vars = HashMap::new();
        vars.insert("feature_dir".into(), feature_dir);
        vars.insert("project_root".into(), worktree_path.clone());
        vars.insert("git_diff".into(), git_diff);
        vars.insert("base_branch".into(), base_branch);
        vars.insert("target_branch".into(), target_branch);

        let cr_path = feature_ctx.artifact_path(ARTIFACT_CR);
        let system_prompt = format!(
            "{rendered}\n\n## 桌面端非交互覆盖（优先于上文任何等待指令）\n\
             没有用户可以回复。禁止提问、禁止派生子 agent / Agent / Task。\n\
             当前 cwd 是前端 git worktree。用 Read/Grep/Bash 核对 diff，不要改业务代码。\n\
             第一个工具调用必须是 Write `{cr}`，写出完整 CR.md。",
            rendered = render_prompt(CR_WEB_PROMPT, &vars),
            cr = cr_path.display(),
        );

        let agent_input = AgentTaskInput {
            prompt: format!(
                "这是桌面端非交互执行。按 architecture-first 审查当前 worktree 相对基准分支的变更。\
                 不要改代码。第一个工具调用必须 Write `{cr}`。",
                cr = cr_path.display()
            ),
            worktree_path: worktree_path.clone(),
            system_prompt: Some(system_prompt),
            model: None,
            max_budget_usd: Some(5.0),
            max_turns: Some(30),
            timeout_ms: Some(15 * 60_000),
            extra_tools: Some(vec![
                "Read".into(),
                "Write".into(),
                "Bash".into(),
                "Grep".into(),
            ]),
        };

        let result = agent_pool.dispatch(agent_input).await;
        adopt_cr_from_worktree(&feature_ctx, &worktree_path);
        let cr_exists = feature_ctx.has_artifact(ARTIFACT_CR);
        if !result.success && !cr_exists {
            return Err(format!(
                "Agent dispatch failed: {}",
                result.error.unwrap_or_default()
            )
            .into());
        }
        if !cr_exists {
            return Err("CR.md was not written".into());
        }

        let cr_body = feature_ctx
            .read_artifact(ARTIFACT_CR)?
            .unwrap_or_default();
        let cr_score = extract_cr_score(&cr_body)
            .or_else(|| result.result.as_deref().and_then(extract_cr_score))
            .unwrap_or_else(|| "B+".into());

        Ok(SkillOutput {
            output: json!({
                "crReportPath": cr_path,
                "crExists": true,
                "crScore": cr_score,
                "agentSessionId": result.session_id,
                "costUsd": result.cost_usd,
            }),
            gates_pass: Some(true),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::extract_cr_score;

    #[test]
    fn extract_cr_score_prefers_b_plus_before_b() {
        assert_eq!(extract_cr_score("建议: B+").as_deref(), Some("B+"));
        assert_eq!(extract_cr_score("评分：A").as_deref(), Some("A"));
    }
}
