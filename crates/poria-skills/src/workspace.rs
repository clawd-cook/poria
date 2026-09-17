use std::path::Path;

use async_trait::async_trait;
use serde_json::json;

use poria_core::contracts::{CapabilityMetadata, Skill, SkillContext};
use poria_core::types::{SkillInput, SkillOutput};
use poria_resources::{git_fetch, WorktreeResource};

use crate::fixture::is_fixture_mode;

pub struct WorkspaceSkill {
    metadata: CapabilityMetadata,
}

impl WorkspaceSkill {
    pub fn new() -> Self {
        Self {
            metadata: CapabilityMetadata {
                id: "skill:workspace".into(),
                name: "Workspace".into(),
                description: "Create git worktree and bind Xingyun branch".into(),
                version: "0.2.0".into(),
            },
        }
    }
}

impl Default for WorkspaceSkill {
    fn default() -> Self {
        Self::new()
    }
}

fn fixture_output() -> SkillOutput {
    SkillOutput {
        output: json!({
            "repos": [
                {
                    "name": "main",
                    "branch": "feature_TEST",
                    "baseBranch": "master",
                    "changeId": "chg-fixture-001",
                    "worktreePath": "/tmp/poria-fixture/worktree",
                    "gitlabProjectPath": "group/repo"
                }
            ]
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

#[async_trait]
impl Skill for WorkspaceSkill {
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

        let repo = input
            .pipeline
            .config
            .repos
            .first()
            .ok_or("缺少前端仓库，无法创建工作区")?;
        let git_root = extra_path(&input, "repo_root").ok_or("missing repo_root in skill input")?;
        let workspace_root = extra_path(&input, "workspace_root")
            .map(str::to_string)
            .or_else(|| {
                let workdir = ctx.workdir.trim();
                if workdir.is_empty() {
                    None
                } else {
                    Some(workdir.to_string())
                }
            })
            .ok_or("missing workspace_root in skill input")?;

        let git_root_path = Path::new(git_root);
        if !git_root_path.join(".git").exists() {
            return Err(format!("前端托管副本不是 git 仓库: {git_root}").into());
        }

        let _ = git_fetch(git_root_path).await;

        let worktree = WorktreeResource::new(Some(workspace_root));
        let created = worktree
            .create(
                repo,
                &input.pipeline.id,
                &repo.base_branch,
                git_root_path,
            )
            .await?;

        Ok(SkillOutput {
            output: json!({
                "repos": [{
                    "name": repo.name,
                    "branch": created.branch,
                    "baseBranch": repo.base_branch,
                    "changeId": serde_json::Value::Null,
                    "worktreePath": created.worktree_path,
                    "gitlabProjectPath": repo.gitlab_project_path,
                }]
            }),
            gates_pass: Some(true),
        })
    }
}
