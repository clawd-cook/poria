use serde::{Deserialize, Serialize};

use super::gate::GateRule;
use super::repo::RepoConfig;
use super::rollback::RollbackInstruction;

/// Full-chain delivery profile (轻舟 standard-openspec + Poria Init/Deploy/Archive).
///
/// Legacy six-stage JSON ids deserialize via serde aliases:
/// `review_prd`→clarify, `design`→propose, `dev`→implement, `cr`→code_review.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageEnum {
    Init,
    #[serde(rename = "clarify", alias = "review_prd")]
    ReviewPrd,
    #[serde(rename = "propose", alias = "design")]
    Design,
    TestPlan,
    #[serde(rename = "implement", alias = "dev")]
    Dev,
    Lint,
    #[serde(rename = "code_review", alias = "cr")]
    Cr,
    TestCases,
    RunAutotest,
    HandoffQa,
    Deploy,
    Archive,
}

pub const STAGE_ORDER: &[StageEnum] = &[
    StageEnum::Init,
    StageEnum::ReviewPrd,
    StageEnum::Design,
    StageEnum::TestPlan,
    StageEnum::Dev,
    StageEnum::Lint,
    StageEnum::Cr,
    StageEnum::TestCases,
    StageEnum::RunAutotest,
    StageEnum::HandoffQa,
    StageEnum::Deploy,
    StageEnum::Archive,
];

impl StageEnum {
    pub fn as_str(self) -> &'static str {
        match self {
            StageEnum::Init => "init",
            StageEnum::ReviewPrd => "clarify",
            StageEnum::Design => "propose",
            StageEnum::TestPlan => "test_plan",
            StageEnum::Dev => "implement",
            StageEnum::Lint => "lint",
            StageEnum::Cr => "code_review",
            StageEnum::TestCases => "test_cases",
            StageEnum::RunAutotest => "run_autotest",
            StageEnum::HandoffQa => "handoff_qa",
            StageEnum::Deploy => "deploy",
            StageEnum::Archive => "archive",
        }
    }

    /// Accepts canonical full-profile ids and legacy six-stage aliases.
    pub fn from_job_id(id: &str) -> Option<Self> {
        match id {
            "init" => Some(Self::Init),
            "clarify" | "review_prd" => Some(Self::ReviewPrd),
            "propose" | "design" => Some(Self::Design),
            "test_plan" => Some(Self::TestPlan),
            "implement" | "dev" => Some(Self::Dev),
            "lint" => Some(Self::Lint),
            "code_review" | "cr" => Some(Self::Cr),
            "test_cases" => Some(Self::TestCases),
            "run_autotest" => Some(Self::RunAutotest),
            "handoff_qa" => Some(Self::HandoffQa),
            "deploy" => Some(Self::Deploy),
            "archive" => Some(Self::Archive),
            _ => None,
        }
    }

    /// Display label for desktop UI (Chinese).
    pub fn label(self) -> &'static str {
        match self {
            StageEnum::Init => "初始化",
            StageEnum::ReviewPrd => "需求澄清",
            StageEnum::Design => "技术方案",
            StageEnum::TestPlan => "测试计划",
            StageEnum::Dev => "编码实现",
            StageEnum::Lint => "静态检查",
            StageEnum::Cr => "代码审查",
            StageEnum::TestCases => "测试用例",
            StageEnum::RunAutotest => "自动化测试",
            StageEnum::HandoffQa => "转测交接",
            StageEnum::Deploy => "部署推送",
            StageEnum::Archive => "归档",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineStatus {
    Created,
    Running,
    WaitingMerge,
    Blocked,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Blocked,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageIssue {
    pub class: String,
    pub message: String,
    pub retryable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    pub pipeline_id: String,
    pub name: StageEnum,
    pub status: StageStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill_id: Option<String>,
    pub retry_count: i32,
    pub max_retries: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gate_results: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue: Option<StageIssue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rollback: Option<RollbackInstruction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackendContext {
    pub git_url: String,
    pub local_path: String,
    pub branch: String,
    pub scope: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PipelineConfig {
    pub gates: Vec<GateRule>,
    pub trd_scope: Vec<String>,
    pub repos: Vec<RepoConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prd_url: Option<String>,
    /// JoySpace URL for the backend TRD. Frontend coding aid only; do not overwrite `TRD.md`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend_trd_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend_context: Option<BackendContext>,
    /// Local folder `~/.poria/projects/<demand_code>` with extracted markdown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_dir: Option<String>,
    /// Frontend `TRD.md` has been confirmed (or confirmation explicitly skipped).
    #[serde(default)]
    pub trd_confirmed: bool,
    /// Workflow id stamped at submit (`demand-to-mr`). Absent on legacy pipelines.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_id: Option<String>,
    /// Materialized jobs from the workflow YAML (order = serial ready preference).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub jobs: Vec<crate::workflow::MaterializedJob>,
    /// Free-text note from stage-boundary dialogue; injected into the next stage prompt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub advance_note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    pub id: String,
    pub demand_id: i64,
    pub demand_code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub demand_name: Option<String>,
    pub status: PipelineStatus,
    pub raw_link: String,
    pub operator: String,
    pub has_regressed: bool,
    pub config: PipelineConfig,
    pub stages: Vec<Stage>,
    pub repos: Vec<RepoConfig>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInput {
    pub stage: Stage,
    pub pipeline: Pipeline,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillOutput {
    pub output: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gates_pass: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frontend_repo() -> RepoConfig {
        RepoConfig {
            name: "ls-entrance".into(),
            git_url: "git@coding.jd.com:ls/ls-entrance.git".into(),
            branch: "master".into(),
            base_branch: "master".into(),
            gitlab_project_path: "ls/ls-entrance".into(),
            depends_on: None,
            build_cmd: None,
        }
    }

    #[test]
    fn pipeline_config_deserializes_legacy_json_without_prd_or_backend() {
        let config: PipelineConfig =
            serde_json::from_str(r#"{"gates":[],"trd_scope":[],"repos":[]}"#).unwrap();
        assert!(config.prd_url.is_none());
        assert!(config.backend_trd_url.is_none());
        assert!(config.backend_context.is_none());
        assert!(config.repos.is_empty());
        assert!(!config.trd_confirmed);
    }

    #[test]
    fn pipeline_config_round_trips_prd_and_backend_context() {
        let config = PipelineConfig {
            gates: vec![],
            trd_scope: vec![],
            repos: vec![frontend_repo()],
            prd_url: Some("https://joyspace.jd.com/pages/abc".into()),
            backend_trd_url: Some("https://joyspace.jd.com/pages/backend-trd".into()),
            backend_context: Some(BackendContext {
                git_url: "git@coding.jd.com:ls/ls-api.git".into(),
                local_path: "/tmp/.poria/repos/ls/ls-api".into(),
                branch: "release".into(),
                scope: "ls".into(),
                name: "ls-api".into(),
            }),
            project_dir: None,
            trd_confirmed: false,
            ..Default::default()
        };

        let parsed: PipelineConfig =
            serde_json::from_str(&serde_json::to_string(&config).unwrap()).unwrap();
        assert_eq!(
            parsed.prd_url.as_deref(),
            Some("https://joyspace.jd.com/pages/abc")
        );
        assert_eq!(
            parsed.backend_trd_url.as_deref(),
            Some("https://joyspace.jd.com/pages/backend-trd")
        );
        let backend = parsed.backend_context.expect("backend_context");
        assert_eq!(backend.branch, "release");
        assert_eq!(backend.name, "ls-api");
        assert_eq!(parsed.repos.len(), 1);
        assert_eq!(parsed.repos[0].name, "ls-entrance");
        assert!(parsed.project_dir.is_none());
    }

    #[test]
    fn pipeline_config_deserializes_json_with_backend_trd_url() {
        let config: PipelineConfig = serde_json::from_str(
            r#"{"gates":[],"trd_scope":[],"repos":[],"backend_trd_url":"https://joyspace.jd.com/pages/be"}"#,
        )
        .unwrap();
        assert_eq!(
            config.backend_trd_url.as_deref(),
            Some("https://joyspace.jd.com/pages/be")
        );
        assert!(config.prd_url.is_none());
    }

    #[test]
    fn stage_order_is_full_chain_without_workspace() {
        assert_eq!(
            STAGE_ORDER,
            &[
                StageEnum::Init,
                StageEnum::ReviewPrd,
                StageEnum::Design,
                StageEnum::TestPlan,
                StageEnum::Dev,
                StageEnum::Lint,
                StageEnum::Cr,
                StageEnum::TestCases,
                StageEnum::RunAutotest,
                StageEnum::HandoffQa,
                StageEnum::Deploy,
                StageEnum::Archive,
            ]
        );
        assert_eq!(STAGE_ORDER.len(), 12);
        assert!(!STAGE_ORDER
            .iter()
            .any(|stage| format!("{stage:?}") == "Workspace"));
    }

    #[test]
    fn stage_enum_job_id_round_trips_and_accepts_legacy_aliases() {
        for stage in STAGE_ORDER {
            assert_eq!(StageEnum::from_job_id(stage.as_str()), Some(*stage));
        }
        assert_eq!(StageEnum::from_job_id("review_prd"), Some(StageEnum::ReviewPrd));
        assert_eq!(StageEnum::from_job_id("design"), Some(StageEnum::Design));
        assert_eq!(StageEnum::from_job_id("dev"), Some(StageEnum::Dev));
        assert_eq!(StageEnum::from_job_id("cr"), Some(StageEnum::Cr));
        assert_eq!(StageEnum::from_job_id("workspace"), None);
    }

    #[test]
    fn stage_enum_deserializes_legacy_json_ids() {
        let clarify: StageEnum = serde_json::from_str(r#""review_prd""#).unwrap();
        assert_eq!(clarify, StageEnum::ReviewPrd);
        assert_eq!(serde_json::to_string(&clarify).unwrap(), r#""clarify""#);

        let implement: StageEnum = serde_json::from_str(r#""dev""#).unwrap();
        assert_eq!(implement, StageEnum::Dev);
        assert_eq!(serde_json::to_string(&implement).unwrap(), r#""implement""#);
    }

    #[test]
    fn pipeline_config_does_not_require_backend_in_repos() {
        let config = PipelineConfig {
            repos: vec![frontend_repo()],
            prd_url: Some("https://joyspace.jd.com/pages/abc".into()),
            backend_context: Some(BackendContext {
                git_url: "git@coding.jd.com:ls/ls-api.git".into(),
                local_path: "/tmp/.poria/repos/ls/ls-api".into(),
                branch: "release".into(),
                scope: "ls".into(),
                name: "ls-api".into(),
            }),
            ..Default::default()
        };
        assert_eq!(config.repos.len(), 1);
        assert_ne!(config.repos[0].name, "ls-api");
    }
}
