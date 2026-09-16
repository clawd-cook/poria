use async_trait::async_trait;
use poria_core::contracts::Skill;
use poria_core::pipeline::PipelineEvent;
use poria_core::types::{Pipeline, SkillOutput, Stage, StageEnum};

/// Persistence layer for pipeline and stage records.
#[async_trait]
pub trait PipelineStore: Send + Sync {
    async fn load(&self, pipeline_id: &str) -> Result<Pipeline, Box<dyn std::error::Error + Send + Sync>>;
    fn save_stage_tx(&self, stage: Option<&Stage>, pipeline: &Pipeline, events: &[PipelineEvent]);
    async fn find_by_status(&self, status: &str) -> Result<Vec<Pipeline>, Box<dyn std::error::Error + Send + Sync>>;
}

/// Loads a skill implementation by its identifier.
#[async_trait]
pub trait SkillLoader: Send + Sync {
    async fn load(&self, skill_id: &str) -> Result<Box<dyn Skill>, Box<dyn std::error::Error + Send + Sync>>;
}

/// Ensures credentials are valid before stage execution.
#[async_trait]
pub trait CredentialGuard: Send + Sync {
    async fn ensure_valid(&self) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>>;
}

/// Orchestrates skill execution across multiple repositories.
#[async_trait]
pub trait MultiRepoOrchestrator: Send + Sync {
    async fn execute(
        &self,
        pipeline: &Pipeline,
        stage: &Stage,
        skill: &dyn Skill,
        credentials: &serde_json::Value,
    ) -> Result<SkillOutput, Box<dyn std::error::Error + Send + Sync>>;
}

/// Human-in-the-loop notification for blocked stages.
#[async_trait]
pub trait HumanLoop: Send + Sync {
    async fn notify(&self, pipeline: &Pipeline, stage: &Stage, issue_class: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn escalate(&self, pipeline: &Pipeline, message: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

/// Interaction with a coding platform (e.g. GitLab) for MR operations.
#[async_trait]
pub trait CodingChannel: Send + Sync {
    async fn get_mr_status(&self, mr_url: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;
    async fn close_mr(&self, project_path: &str, iid: i64) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn create_merge_request(&self, opts: serde_json::Value) -> Result<MrCreateResult, Box<dyn std::error::Error + Send + Sync>>;
}

#[derive(Debug, Clone)]
pub struct MrCreateResult {
    pub url: String,
}

/// Executes shell commands.
#[async_trait]
pub trait Terminal: Send + Sync {
    async fn exec(&self, command: &str, cwd: Option<&str>, timeout_ms: Option<i64>) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;
}

/// Sends instant messages (e.g. JME).
#[async_trait]
pub trait Messenger: Send + Sync {
    async fn send(&self, message: &str, target: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

/// File-system queries.
pub trait FileSystem: Send + Sync {
    fn file_exists(&self, path: &str) -> bool;
}

/// Simple process-level file lock.
pub trait FileLock: Send + Sync {
    fn acquire(&self);
    fn release(&self);
}

/// Work queue for pipeline IDs.
pub trait Queue: Send + Sync {
    fn dequeue(&self) -> Option<String>;
}

/// Recovers pipelines that were interrupted (e.g. after a crash).
#[async_trait]
pub trait Recovery: Send + Sync {
    async fn recover_all(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

/// Maps a stage to its corresponding skill identifier.
pub fn stage_skill_id(stage: StageEnum) -> &'static str {
    match stage {
        StageEnum::Init => "skill:init",
        StageEnum::ReviewPrd => "skill:review-prd",
        StageEnum::Design => "skill:gen-trd",
        StageEnum::Workspace => "skill:workspace",
        StageEnum::Dev => "skill:gen-code",
        StageEnum::Cr => "skill:code-review",
        StageEnum::Deploy => "skill:deploy",
    }
}

/// Stages that support multi-repo orchestration.
pub fn is_multi_repo_stage(stage: StageEnum) -> bool {
    matches!(stage, StageEnum::Dev | StageEnum::Cr | StageEnum::Deploy)
}

/// No-op implementation so `()` works as the default type parameter for
/// `PipelineExecutor<S, L, C, M = ()>` / `PipelineWorker<S, L, C, M = ()>`.
#[async_trait]
impl MultiRepoOrchestrator for () {
    async fn execute(
        &self,
        _pipeline: &Pipeline,
        _stage: &Stage,
        _skill: &dyn Skill,
        _credentials: &serde_json::Value,
    ) -> Result<SkillOutput, Box<dyn std::error::Error + Send + Sync>> {
        Err("MultiRepoOrchestrator not configured".into())
    }
}
