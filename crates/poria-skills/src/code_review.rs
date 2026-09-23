use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;

use poria_core::contracts::{CapabilityMetadata, Skill, SkillContext};
use poria_core::feature_context::{FeatureContext, ARTIFACT_CR};
use poria_core::types::{AgentTaskInput, SkillInput, SkillOutput};
use poria_resources::ClaudeAgentPool;

use crate::claude_prompt::{
    advance_note_from_input, backend_dir, backend_trd_url, build_claude_skill_prompt_with_note,
    extra_nonempty, frontend_base_branch,
    resolve_feature_dir, resolve_workspace_cwd, SKILL_CODE_REVIEW,
};
use crate::cr_findings::mr_notes_from_cr;
use crate::error::SkillError;
use crate::fixture::is_fixture_mode;

/// Short reviewer identity for `claude --system-prompt`. Procedure stays in SKILL.md.
pub const CR_REVIEWER_SYSTEM_PROMPT: &str =
    "你是独立代码评审者，不是本次变更的作者。禁止沿用 gen-code 会话。不要改业务代码。安全与覆盖率以扫描报告为准，禁止自打分。";

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
            "mrNotes": [],
            "independentReviewer": true,
            "securityPass": true,
            "securitySource": "npm_audit",
            "securityScanPassed": true
        }),
        gates_pass: None,
    }
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
    if candidate
        .symlink_metadata()
        .map(|meta| meta.file_type().is_symlink())
        .unwrap_or(false)
    {
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

        let feature_dir = resolve_feature_dir(&input)?;
        let workspace_path = resolve_workspace_cwd(&input, &ctx.workdir)?;
        let frontend_worktree = extra_nonempty(&input, "worktree_path")
            .map(str::to_string)
            .ok_or("missing worktree_path in skill input")?;
        let backend = backend_dir(&input);

        let feature_ctx = FeatureContext::from_root(Path::new(&feature_dir))
            .ok_or("feature context not found")?;

        let prompt = build_claude_skill_prompt_with_note(
            SKILL_CODE_REVIEW,
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
            system_prompt: Some(CR_REVIEWER_SYSTEM_PROMPT.into()),
            model: extra_nonempty(&input, "cr_model").map(str::to_string),
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
        adopt_cr_from_worktree(&feature_ctx, &frontend_worktree);
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

        let cr_body = feature_ctx.read_artifact(ARTIFACT_CR)?.unwrap_or_default();
        let cr_score = extract_cr_score(&cr_body)
            .or_else(|| result.result.as_deref().and_then(extract_cr_score))
            .unwrap_or_else(|| "B+".into());
        let mr_notes = mr_notes_from_cr(&cr_body);

        let security =
            crate::quality_gates::collect_security_scan(Path::new(&frontend_worktree)).await;
        let security_message = crate::quality_gates::security_block_message(&security);
        if !security_message.is_empty() {
            tracing::warn!(pipeline_id = %input.pipeline.id, "{security_message}");
        }

        Ok(SkillOutput {
            output: json!({
                "crReportPath": feature_ctx.artifact_path(ARTIFACT_CR),
                "crExists": true,
                "crScore": cr_score,
                "mrNotes": mr_notes,
                "independentReviewer": true,
                "securityPass": security.pass,
                "securitySource": security.source,
                "securityHigh": security.high,
                "securityCritical": security.critical,
                "securityDetail": security.detail,
                "agentSessionId": result.session_id,
                "costUsd": result.cost_usd,
            }),
            gates_pass: Some(security.pass == Some(true)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_cr_score_prefers_b_plus_before_b() {
        assert_eq!(extract_cr_score("建议: B+").as_deref(), Some("B+"));
        assert_eq!(extract_cr_score("评分：A").as_deref(), Some("A"));
    }

    #[test]
    fn reviewer_system_prompt_is_independent() {
        assert!(CR_REVIEWER_SYSTEM_PROMPT.contains("独立"));
        assert!(CR_REVIEWER_SYSTEM_PROMPT.contains("gen-code"));
        assert!(CR_REVIEWER_SYSTEM_PROMPT.contains("禁止自打分"));
    }
}
