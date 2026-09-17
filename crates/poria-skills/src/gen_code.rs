use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;

use poria_core::contracts::{CapabilityMetadata, Skill, SkillContext};
use poria_core::feature_context::{FeatureContext, ARTIFACT_PRD, ARTIFACT_TASK, ARTIFACT_TRD};
use poria_core::types::{AgentTaskInput, SkillInput, SkillOutput};
use poria_resources::{terminal, ClaudeAgentPool, TerminalExecInput};

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

fn extra_path<'a>(input: &'a SkillInput, key: &str) -> Option<&'a str> {
    input
        .extra
        .get(key)
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn resolve_feature_dir(input: &SkillInput, ctx: &SkillContext) -> Result<String, &'static str> {
    extra_path(input, "feature_dir")
        .map(str::to_string)
        .or_else(|| input.pipeline.config.project_dir.clone())
        .or_else(|| {
            let workdir = ctx.workdir.trim();
            if workdir.is_empty() {
                None
            } else {
                Some(workdir.to_string())
            }
        })
        .ok_or("missing feature_dir in skill input")
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

        let feature_dir = resolve_feature_dir(&input, &ctx)?;
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

        let trd_content = feature_ctx
            .read_artifact(ARTIFACT_TRD)?
            .ok_or("TRD.md not found")?;
        let prd_content = feature_ctx.read_artifact(ARTIFACT_PRD)?.unwrap_or_default();

        let mut vars = HashMap::new();
        vars.insert("feature_dir".into(), feature_dir.clone());
        vars.insert("project_root".into(), worktree_path.clone());
        vars.insert("trd_content".into(), trd_content);
        vars.insert("prd_content".into(), prd_content);
        insert_backend_coding_aid_vars(&mut vars, &input.pipeline.config, &feature_ctx);

        let task_path = feature_ctx.artifact_path(ARTIFACT_TASK);
        let system_prompt = format!(
            "{rendered}\n\n## 桌面端非交互覆盖（优先于上文任何等待指令）\n\
             没有用户可以回复。禁止提问、禁止等待 TASK.md 确认、禁止派生子 agent / Explore / Agent / Task。\n\
             当前 cwd 是前端 git worktree：`{worktree}`。业务代码只改这里。\n\
             先用 Write 把完整 TASK.md 写到 `{task}`，然后立即按计划实现全部 T-n，不要停下。\n\
             禁止读取或修改 backend_repo_path 下任何源码；不得覆盖或改名前端 TRD.md。",
            rendered = render_prompt(CODE_IMPL_PROMPT, &vars),
            worktree = worktree_path,
            task = task_path.display(),
        );

        let agent_input = AgentTaskInput {
            prompt: format!(
                "这是桌面端非交互执行。材料已在系统提示中。先 Write `{task}`，再在当前前端 worktree 实现代码。\
                 不要等用户确认，不要子 agent。后端 TRD 与后端仓只读，禁止改后端仓。",
                task = task_path.display()
            ),
            worktree_path: worktree_path.clone(),
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
        adopt_task_md_from_worktree(&feature_ctx, &worktree_path);
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

        let porcelain = git_porcelain(&worktree_path).await;
        let changed_files = collect_changed_files(&porcelain);

        Ok(SkillOutput {
            output: json!({
                "taskPath": task_path,
                "taskExists": true,
                "worktreePath": worktree_path,
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
