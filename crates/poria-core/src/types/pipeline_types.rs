use serde::{Deserialize, Serialize};

use super::gate::GateRule;
use super::repo::RepoConfig;
use super::rollback::RollbackInstruction;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageEnum {
    Init,
    ReviewPrd,
    Design,
    Workspace,
    Dev,
    Cr,
    Deploy,
}

pub const STAGE_ORDER: &[StageEnum] = &[
    StageEnum::Init,
    StageEnum::ReviewPrd,
    StageEnum::Design,
    StageEnum::Workspace,
    StageEnum::Dev,
    StageEnum::Cr,
    StageEnum::Deploy,
];

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    pub gates: Vec<GateRule>,
    pub trd_scope: Vec<String>,
    pub repos: Vec<RepoConfig>,
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
