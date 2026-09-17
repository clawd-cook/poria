use std::path::Path;

use async_trait::async_trait;
use serde_json::json;

use poria_channels::coding::{
    create_merge_request_live, find_mr_live, CreateMrInput, FindMrQuery, JacpCredentials, MrStatus,
};
use poria_channels::xingyun::{bind_branch, BindBranchInput};
use poria_core::contracts::{CapabilityMetadata, Skill, SkillContext};
use poria_core::types::{SkillInput, SkillOutput};
use poria_resources::{git_add_all, git_commit, git_has_changes, git_push_set_upstream};

use crate::fixture::is_fixture_mode;

pub struct DeploySkill {
    metadata: CapabilityMetadata,
}

impl DeploySkill {
    pub fn new() -> Self {
        Self {
            metadata: CapabilityMetadata {
                id: "skill:deploy".into(),
                name: "Deploy".into(),
                description:
                    "Bind Xingyun change, push branch, and create idempotent merge request".into(),
                version: "0.2.0".into(),
            },
        }
    }
}

impl Default for DeploySkill {
    fn default() -> Self {
        Self::new()
    }
}

fn fixture_output() -> SkillOutput {
    SkillOutput {
        output: json!({
            "mrUrl": "https://coding.jd.com/group/repo/-/merge_requests/1",
            "mrIid": 1,
            "repo": "main",
            "changeId": "chg-fixture-001",
            "branch": "feature_TEST",
            "pushed": true
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

fn credentials_from_ctx(
    ctx: &SkillContext,
) -> Result<JacpCredentials, Box<dyn std::error::Error + Send + Sync>> {
    let cookie = ctx
        .credentials
        .get("cookie")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .unwrap_or_default();
    let username = ctx
        .credentials
        .get("username")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .unwrap_or_default();
    if cookie.is_empty() {
        return Err("请先登录".into());
    }
    Ok(JacpCredentials {
        cookie: cookie.to_string(),
        username: username.to_string(),
    })
}

fn xingyun_creds(creds: &JacpCredentials) -> poria_channels::xingyun::JacpCredentials {
    poria_channels::xingyun::JacpCredentials {
        cookie: creds.cookie.clone(),
        username: creds.username.clone(),
    }
}

fn commit_message(demand_code: &str, demand_name: Option<&str>) -> String {
    match demand_name.map(str::trim).filter(|name| !name.is_empty()) {
        Some(name) => format!("feat({demand_code}): {name}"),
        None => format!("feat({demand_code}): deploy feature branch"),
    }
}

#[async_trait]
impl Skill for DeploySkill {
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
            .ok_or("缺少前端仓库，无法部署")?;
        let worktree = extra_path(&input, "worktree_path")
            .map(ToOwned::to_owned)
            .or_else(|| {
                let workdir = ctx.workdir.trim();
                if workdir.is_empty() {
                    None
                } else {
                    Some(workdir.to_string())
                }
            })
            .ok_or("missing worktree_path in skill input")?;
        let worktree_path = Path::new(&worktree);
        if !worktree_path.join(".git").exists() {
            return Err(format!("前端工作区无效: {worktree}").into());
        }

        let branch = extra_path(&input, "branch")
            .unwrap_or(repo.branch.as_str())
            .to_string();
        let base_branch = extra_path(&input, "base_branch")
            .unwrap_or(repo.base_branch.as_str())
            .to_string();
        let project_path = extra_path(&input, "gitlab_project_path")
            .unwrap_or(repo.gitlab_project_path.as_str())
            .to_string();
        if project_path.trim().is_empty() {
            return Err("缺少 gitlabProjectPath，无法创建 MR".into());
        }

        let creds = credentials_from_ctx(&ctx)?;
        if git_has_changes(worktree_path).await? {
            git_add_all(worktree_path).await?;
            let _ = git_commit(
                worktree_path,
                &commit_message(
                    &input.pipeline.demand_code,
                    input.pipeline.demand_name.as_deref(),
                ),
            )
            .await?;
        }

        // EasyCI SELECT requires the remote branch to exist (CREATE is not a
        // GraphQL enum). h2o CREATE is local git checkout + push, then SELECT.
        git_push_set_upstream(worktree_path, "origin", &branch).await?;

        let bound = bind_branch(
            &xingyun_creds(&creds),
            BindBranchInput {
                demand_id: Some(input.pipeline.demand_id),
                demand_code: Some(input.pipeline.demand_code.clone()),
                git_url: repo.git_url.clone(),
                branch: branch.clone(),
                base_branch: Some(base_branch.clone()),
            },
        )
        .await?;

        let find_query = FindMrQuery {
            project_path: project_path.clone(),
            source_branch: branch.clone(),
            target_branch: base_branch.clone(),
            state: Some(MrStatus::Opened),
        };
        let existing = find_mr_live(&creds, &find_query).await?;
        let (mr_url, mr_iid) = if let Some(mr) = existing {
            (mr.url, Some(mr.iid))
        } else {
            let title = match input
                .pipeline
                .demand_name
                .as_deref()
                .map(str::trim)
                .filter(|name| !name.is_empty())
            {
                Some(name) => format!("[{}] {name}", input.pipeline.demand_code),
                None => format!("[{}] {}", input.pipeline.demand_code, branch),
            };
            let created = create_merge_request_live(
                &creds,
                &CreateMrInput {
                    title,
                    description: None,
                    source_branch: Some(branch.clone()),
                    target_branch: Some(base_branch.clone()),
                    project_id: Some(project_path),
                },
            )
            .await?;
            (created.url, created.iid)
        };

        Ok(SkillOutput {
            output: json!({
                "mrUrl": mr_url,
                "mrIid": mr_iid,
                "repo": repo.name,
                "changeId": bound.change_id,
                "branch": branch,
                "pushed": true,
                "alreadyBound": bound.already_bound,
            }),
            gates_pass: Some(true),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commit_message_includes_demand_name() {
        assert_eq!(
            commit_message("R1", Some("供应商改名")),
            "feat(R1): 供应商改名"
        );
        assert_eq!(
            commit_message("R1", Some("  ")),
            "feat(R1): deploy feature branch"
        );
    }

    #[test]
    fn fixture_output_has_mr_url() {
        let output = fixture_output();
        assert_eq!(
            output.output.get("mrUrl").and_then(|v| v.as_str()),
            Some("https://coding.jd.com/group/repo/-/merge_requests/1")
        );
    }
}
