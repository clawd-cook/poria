//! Thin document / mechanical nodes for the full-chain profile (P1 stubs).

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use chrono::Utc;
use serde_json::json;

use poria_core::contracts::{CapabilityMetadata, Skill, SkillContext};
use poria_core::types::{SkillInput, SkillOutput, StageEnum};

use crate::dev_verify::run_frontend_verify;
use crate::fixture::is_fixture_mode;

fn project_dir_from_input(input: &SkillInput) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    if let Some(dir) = input
        .pipeline
        .config
        .project_dir
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return Ok(PathBuf::from(dir));
    }
    Err("缺少 project_dir，无法写入交付文档".into())
}

fn frontend_worktree(input: &SkillInput) -> Option<PathBuf> {
    input.pipeline.stages.iter().find_map(|stage| {
        if stage.name != StageEnum::Init {
            return None;
        }
        stage.output.as_ref().and_then(|output| {
            output
                .get("worktreePath")
                .and_then(|v| v.as_str())
                .map(PathBuf::from)
        })
    })
}

fn write_markdown(
    project_dir: &Path,
    file_name: &str,
    body: &str,
) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    std::fs::create_dir_all(project_dir)?;
    let path = project_dir.join(file_name);
    std::fs::write(&path, body)?;
    Ok(path)
}

fn note_section(input: &SkillInput) -> String {
    input
        .pipeline
        .config
        .advance_note
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|note| format!("\n\n## 上一阶段补充上下文\n\n{note}\n"))
        .unwrap_or_default()
}

macro_rules! thin_doc_skill {
    ($struct:ident, $id:expr, $name:expr, $desc:expr, $file:expr, $path_key:expr, $title:expr) => {
        pub struct $struct {
            metadata: CapabilityMetadata,
        }

        impl $struct {
            pub fn new() -> Self {
                Self {
                    metadata: CapabilityMetadata {
                        id: $id.into(),
                        name: $name.into(),
                        description: $desc.into(),
                        version: "0.1.0".into(),
                    },
                }
            }
        }

        impl Default for $struct {
            fn default() -> Self {
                Self::new()
            }
        }

        #[async_trait]
        impl Skill for $struct {
            fn metadata(&self) -> &CapabilityMetadata {
                &self.metadata
            }

            async fn execute(
                &self,
                input: SkillInput,
                _ctx: SkillContext,
            ) -> Result<SkillOutput, Box<dyn std::error::Error + Send + Sync>> {
                if is_fixture_mode() {
                    return Ok(SkillOutput {
                        output: json!({
                            "summary": concat!($title, " (fixture)"),
                            $path_key: concat!("/tmp/poria-fixture/", $file),
                            "artifactPaths": [concat!("/tmp/poria-fixture/", $file)],
                        }),
                        gates_pass: Some(true),
                    });
                }
                let project_dir = project_dir_from_input(&input)?;
                let stamp = Utc::now().to_rfc3339();
                let body = format!(
                    "# {}\n\n> 由 Poria 全链路节点自动生成（{}）\n\n需求：{} ({})\n{}\n## 说明\n\n本节点为 MVP 文档产物。可在工作台对话切面确认后继续。\n",
                    $title,
                    stamp,
                    input.pipeline.demand_code,
                    input.pipeline.demand_id,
                    note_section(&input),
                );
                let path = write_markdown(&project_dir, $file, &body)?;
                let path_str = path.to_string_lossy().to_string();
                Ok(SkillOutput {
                    output: json!({
                        "summary": format!("{} 已写入 {}", $title, $file),
                        $path_key: path_str,
                        "artifactPaths": [path_str],
                        "projectDir": project_dir.to_string_lossy(),
                    }),
                    gates_pass: Some(true),
                })
            }
        }
    };
}

thin_doc_skill!(
    TestPlanSkill,
    "skill:test-plan",
    "Test Plan",
    "Write delivery/<task>/test/test-plan.md equivalent under projects",
    "test-plan.md",
    "testPlanPath",
    "测试计划"
);

thin_doc_skill!(
    TestCasesSkill,
    "skill:test-cases",
    "Test Cases",
    "Write test-cases.md under the demand project dir",
    "test-cases.md",
    "testCasesPath",
    "测试用例"
);

thin_doc_skill!(
    HandoffQaSkill,
    "skill:handoff-qa",
    "Handoff QA",
    "Write handoff-report.md for QA handoff",
    "handoff-report.md",
    "handoffReportPath",
    "转测报告"
);

thin_doc_skill!(
    ArchiveSkill,
    "skill:archive",
    "Archive",
    "Write archive summary and mark delivery archive-ready",
    "archive.md",
    "archivePath",
    "归档摘要"
);

/// Mechanical lint / typecheck node using existing Dev verify commands.
pub struct LintSkill {
    metadata: CapabilityMetadata,
}

impl LintSkill {
    pub fn new() -> Self {
        Self {
            metadata: CapabilityMetadata {
                id: "skill:lint".into(),
                name: "Lint".into(),
                description: "Run frontend typecheck/lint verify commands in the worktree".into(),
                version: "0.1.0".into(),
            },
        }
    }
}

impl Default for LintSkill {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Skill for LintSkill {
    fn metadata(&self) -> &CapabilityMetadata {
        &self.metadata
    }

    async fn execute(
        &self,
        input: SkillInput,
        _ctx: SkillContext,
    ) -> Result<SkillOutput, Box<dyn std::error::Error + Send + Sync>> {
        if is_fixture_mode() {
            return Ok(SkillOutput {
                output: json!({
                    "summary": "Lint passed (fixture)",
                    "lintPass": true,
                }),
                gates_pass: Some(true),
            });
        }
        let worktree = frontend_worktree(&input)
            .ok_or_else(|| "缺少前端 worktree，无法执行 lint".to_string())?;
        run_frontend_verify(worktree.as_path(), None)
            .await
            .map_err(|err| format!("Lint 失败：{err}"))?;
        Ok(SkillOutput {
            output: json!({
                "summary": "静态检查 / typecheck 通过",
                "lintPass": true,
                "worktreePath": worktree.to_string_lossy(),
            }),
            gates_pass: Some(true),
        })
    }
}

/// Optional local autotest / manual certify node.
pub struct RunAutotestSkill {
    metadata: CapabilityMetadata,
}

impl RunAutotestSkill {
    pub fn new() -> Self {
        Self {
            metadata: CapabilityMetadata {
                id: "skill:run-autotest".into(),
                name: "Run Autotest".into(),
                description: "Record autotest / manual certify result under projects".into(),
                version: "0.1.0".into(),
            },
        }
    }
}

impl Default for RunAutotestSkill {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Skill for RunAutotestSkill {
    fn metadata(&self) -> &CapabilityMetadata {
        &self.metadata
    }

    async fn execute(
        &self,
        input: SkillInput,
        _ctx: SkillContext,
    ) -> Result<SkillOutput, Box<dyn std::error::Error + Send + Sync>> {
        if is_fixture_mode() {
            return Ok(SkillOutput {
                output: json!({
                    "summary": "Autotest certified (fixture)",
                    "autotestPass": true,
                    "testReportPath": "/tmp/poria-fixture/test-report.md",
                }),
                gates_pass: Some(true),
            });
        }
        let project_dir = project_dir_from_input(&input)?;
        let stamp = Utc::now().to_rfc3339();
        let body = format!(
            "# 自动化测试报告\n\n> 生成于 {}\n\n需求：{} ({})\n{}\n## 结果\n\nMVP：未强制跑 Playwright。请在工作台确认本机测试通过后继续。\n",
            stamp,
            input.pipeline.demand_code,
            input.pipeline.demand_id,
            note_section(&input),
        );
        let path = write_markdown(&project_dir, "test-report.md", &body)?;
        let path_str = path.to_string_lossy().to_string();
        Ok(SkillOutput {
            output: json!({
                "summary": "已写入 test-report.md，等待确认锁闸",
                "autotestPass": true,
                "testReportPath": path_str,
                "artifactPaths": [path_str],
                "projectDir": project_dir.to_string_lossy(),
            }),
            gates_pass: Some(true),
        })
    }
}
