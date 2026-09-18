use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;

use poria_core::contracts::{CapabilityMetadata, Skill, SkillContext};
use poria_core::feature_context::{FeatureContext, ARTIFACT_TASK, ARTIFACT_TRD};
use poria_core::types::{AgentTaskInput, SkillInput, SkillOutput};
use poria_resources::{terminal, ClaudeAgentPool, TerminalExecInput};

use crate::claude_prompt::{
    backend_dir, backend_trd_url, build_claude_skill_prompt, extra_nonempty, frontend_base_branch,
    resolve_feature_dir, resolve_workspace_cwd, SKILL_GEN_CODE,
};
use crate::error::SkillError;
use crate::fixture::is_fixture_mode;

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

fn collect_changed_files(porcelain: &str) -> Vec<String> {
    porcelain
        .lines()
        .filter_map(|line| {
            if line.len() < 4 {
                return None;
            }
            let path = line[3..].trim();
            if path.is_empty() {
                None
            } else {
                Some(path.to_string())
            }
        })
        .collect()
}

async fn git_porcelain(worktree: &str) -> String {
    terminal::exec(TerminalExecInput {
        command: "git status --porcelain".into(),
        cwd: Some(worktree.to_string()),
        env: None,
        timeout_ms: Some(15_000),
    })
    .await
    .map(|result| result.stdout)
    .unwrap_or_default()
}

fn adopt_task_md_from_worktree(feature_ctx: &FeatureContext, worktree: &str) {
    if feature_ctx.has_artifact(ARTIFACT_TASK) {
        return;
    }
    let candidate = Path::new(worktree).join(ARTIFACT_TASK);
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
        let _ = feature_ctx.write_artifact(ARTIFACT_TASK, &content);
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

        let feature_dir = resolve_feature_dir(&input)?;
        let workspace_path = resolve_workspace_cwd(&input, &ctx.workdir)?;
        let frontend_worktree = extra_nonempty(&input, "worktree_path")
            .map(str::to_string)
            .ok_or("missing worktree_path in skill input")?;
        let backend = backend_dir(&input);

        let feature_ctx = FeatureContext::from_root(Path::new(&feature_dir))
            .ok_or("feature context not found")?;

        if feature_ctx
            .read_artifact(ARTIFACT_TRD)?
            .filter(|content| !content.trim().is_empty())
            .is_none()
        {
            return Err("TRD.md not found".into());
        }

        let prompt = build_claude_skill_prompt(
            SKILL_GEN_CODE,
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
        adopt_task_md_from_worktree(&feature_ctx, &frontend_worktree);
        let task_exists = feature_ctx.has_artifact(ARTIFACT_TASK);
        if !result.success && !task_exists {
            return Err(format!(
                "Agent dispatch failed: {}",
                result.error.unwrap_or_default()
            )
            .into());
        }
        if !task_exists {
            return Err("TASK.md was not written".into());
        }

        let porcelain = git_porcelain(&frontend_worktree).await;
        let changed_files = collect_changed_files(&porcelain);

        Ok(SkillOutput {
            output: json!({
                "taskPath": feature_ctx.artifact_path(ARTIFACT_TASK),
                "taskExists": true,
                "worktreePath": frontend_worktree,
                "changedFiles": changed_files,
                "agentSessionId": result.session_id,
                "costUsd": result.cost_usd,
            }),
            gates_pass: Some(true),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::collect_changed_files;

    #[test]
    fn collect_changed_files_parses_porcelain() {
        let files = collect_changed_files(" M src/foo.ts\n?? src/bar.ts\n");
        assert_eq!(files, vec!["src/foo.ts", "src/bar.ts"]);
    }
}
