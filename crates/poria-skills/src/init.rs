use std::path::PathBuf;

use async_trait::async_trait;
use serde_json::json;

use poria_channels::joyspace::{export_page_markdown, JoySpaceAuth};
use poria_core::contracts::{CapabilityMetadata, Skill, SkillContext};
use poria_core::feature_context::{FeatureContext, ARTIFACT_BACKEND_TRD, ARTIFACT_PRD};
use poria_core::types::{SkillInput, SkillOutput};

use crate::fixture::is_fixture_mode;

pub struct InitSkill {
    metadata: CapabilityMetadata,
}

impl InitSkill {
    pub fn new() -> Self {
        Self {
            metadata: CapabilityMetadata {
                id: "skill:init".into(),
                name: "Init".into(),
                description: "Export JoySpace docs and create frontend/backend worktrees".into(),
                version: "0.2.0".into(),
            },
        }
    }
}

impl Default for InitSkill {
    fn default() -> Self {
        Self::new()
    }
}

fn fixture_output() -> SkillOutput {
    SkillOutput {
        output: json!({
            "projectDir": "/tmp/poria-fixture/project",
            "prdPath": "/tmp/poria-fixture/project/PRD.md",
            "prdTitle": "Fixture PRD",
            "backendTrdPath": "/tmp/poria-fixture/project/BACKEND_TRD.md",
            "backendTrdTitle": "Fixture Backend TRD",
            "worktreePath": "/tmp/poria-fixture/frontend",
            "backendWorktreePath": "/tmp/poria-fixture/backend",
            "repos": [
                {
                    "name": "main",
                    "branch": "feature_TEST",
                    "baseBranch": "master",
                    "worktreePath": "/tmp/poria-fixture/frontend",
                    "gitlabProjectPath": "group/repo"
                }
            ],
            "demandMetadata": {
                "demandId": 1,
                "demandCode": "TEST",
                "name": "Fixture Demand",
                "prdUrl": "http://fixture",
                "attachments": [],
                "rawLink": "http://fixture"
            }
        }),
        gates_pass: None,
    }
}

fn cookie_from_context(ctx: &SkillContext) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    ctx.credentials
        .get("cookie")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| "请先登录后再导出 JoySpace 文档".into())
}

fn require_url(value: Option<&str>, label: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let url = value.map(str::trim).unwrap_or("");
    if url.is_empty() {
        return Err(format!("缺少 {label}").into());
    }
    Ok(url.to_string())
}

#[async_trait]
impl Skill for InitSkill {
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

        let workdir = ctx.workdir.trim();
        if workdir.is_empty() {
            return Err("InitSkill 需要项目目录 workdir".into());
        }

        let cookie = cookie_from_context(&ctx)?;
        let team_id = ctx
            .credentials
            .get("team_id")
            .and_then(|v| v.as_str());
        let auth = JoySpaceAuth::new(cookie, team_id);

        let prd_url = require_url(input.pipeline.config.prd_url.as_deref(), "JoySpace PRD 链接")?;
        let backend_trd_url = require_url(
            input.pipeline.config.backend_trd_url.as_deref(),
            "JoySpace 后端 TRD 链接",
        )?;

        let feature = FeatureContext::create_at(
            PathBuf::from(workdir),
            &input.pipeline.id,
            input.pipeline.demand_id,
        )?;

        let prd = export_page_markdown(&auth, &prd_url).await?;
        feature.write_artifact(ARTIFACT_PRD, &prd.markdown)?;

        let backend_trd = export_page_markdown(&auth, &backend_trd_url).await?;
        feature.write_artifact(ARTIFACT_BACKEND_TRD, &backend_trd.markdown)?;

        Ok(SkillOutput {
            output: json!({
                "projectDir": feature.root.to_string_lossy(),
                "prdPath": feature.artifact_path(ARTIFACT_PRD).to_string_lossy(),
                "prdTitle": prd.title,
                "backendTrdPath": feature.artifact_path(ARTIFACT_BACKEND_TRD).to_string_lossy(),
                "backendTrdTitle": backend_trd.title,
                "prdWarnings": prd.warnings,
                "backendTrdWarnings": backend_trd.warnings,
                "demandMetadata": {
                    "demandId": input.pipeline.demand_id,
                    "demandCode": input.pipeline.demand_code,
                    "name": input.pipeline.demand_name,
                    "prdUrl": prd_url,
                    "backendTrdUrl": backend_trd_url,
                    "rawLink": input.pipeline.raw_link,
                }
            }),
            gates_pass: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_output_includes_worktree_paths() {
        let output = fixture_output().output;
        assert_eq!(
            output.get("worktreePath").and_then(|v| v.as_str()),
            Some("/tmp/poria-fixture/frontend")
        );
        assert_eq!(
            output.get("backendWorktreePath").and_then(|v| v.as_str()),
            Some("/tmp/poria-fixture/backend")
        );
        let repos = output.get("repos").and_then(|v| v.as_array()).unwrap();
        assert_eq!(
            repos[0].get("worktreePath").and_then(|v| v.as_str()),
            Some("/tmp/poria-fixture/frontend")
        );
        assert_eq!(
            repos[0].get("baseBranch").and_then(|v| v.as_str()),
            Some("master")
        );
    }
}
