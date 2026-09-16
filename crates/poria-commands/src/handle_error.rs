use poria_core::types::{
    IssueClass, Pipeline, PipelineStatus, Stage, StageIssue, StageStatus, ISSUE_POLICIES,
};

use crate::exception_classifier;
use crate::traits::HumanLoop;

/// The result of handling a stage error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandleErrorResult {
    pub issue_class: IssueClass,
    pub action: ErrorAction,
}

/// What to do after a stage error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorAction {
    Retry,
    Blocked,
    Failed,
}

/// Inspects an error, applies the issue-policy table, mutates stage status
/// in-place, writes the recommended pipeline status to `pipeline_status_out`,
/// and returns the recommended action.
///
/// The caller is responsible for writing `pipeline_status_out` back to
/// `pipeline.status` -- this split avoids borrowing `&mut Pipeline` and
/// `&mut pipeline.stages[i]` simultaneously.
pub async fn handle_stage_error(
    pipeline_status: &mut PipelineStatus,
    stage: &mut Stage,
    error_message: &str,
    human_loop: Option<&(dyn HumanLoop + '_)>,
    pipeline_for_notify: Option<&Pipeline>,
) -> HandleErrorResult {
    let issue_class = exception_classifier::classify(error_message);
    let policy = ISSUE_POLICIES
        .get(&issue_class)
        .expect("ISSUE_POLICIES must cover every IssueClass variant");

    // Auto-retry path
    if policy.auto_retry > 0 && stage.retry_count < policy.auto_retry {
        stage.status = StageStatus::Failed;
        stage.retry_count += 1;
        stage.issue = Some(StageIssue {
            class: format!("{:?}", issue_class),
            message: error_message.to_string(),
            retryable: true,
        });
        return HandleErrorResult {
            issue_class,
            action: ErrorAction::Retry,
        };
    }

    // Human-escalation path
    if !policy.notify_roles.is_empty() {
        stage.status = StageStatus::Blocked;
        stage.issue = Some(StageIssue {
            class: format!("{:?}", issue_class),
            message: error_message.to_string(),
            retryable: false,
        });
        *pipeline_status = PipelineStatus::Blocked;
        if let (Some(hl), Some(pl)) = (human_loop, pipeline_for_notify) {
            let _ = hl
                .notify(pl, stage, &format!("{:?}", issue_class))
                .await;
        }
        return HandleErrorResult {
            issue_class,
            action: ErrorAction::Blocked,
        };
    }

    // Terminal failure
    stage.status = StageStatus::Failed;
    stage.issue = Some(StageIssue {
        class: format!("{:?}", issue_class),
        message: error_message.to_string(),
        retryable: false,
    });
    *pipeline_status = PipelineStatus::Failed;
    HandleErrorResult {
        issue_class,
        action: ErrorAction::Failed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use poria_core::types::{PipelineConfig, PipelineStatus, StageEnum, StageStatus};

    fn make_stage() -> Stage {
        Stage {
            id: None,
            pipeline_id: "pl-test".into(),
            name: StageEnum::Dev,
            status: StageStatus::Running,
            skill_id: None,
            retry_count: 0,
            max_retries: 3,
            input: None,
            output: None,
            gate_results: None,
            issue: None,
            rollback: None,
            agent_session_id: None,
            started_at: None,
            completed_at: None,
        }
    }

    fn _make_pipeline() -> Pipeline {
        Pipeline {
            id: "pl-test".into(),
            demand_id: 1,
            demand_code: "D-001".into(),
            demand_name: None,
            status: PipelineStatus::Running,
            raw_link: String::new(),
            operator: "dev".into(),
            has_regressed: false,
            config: PipelineConfig {
                gates: vec![],
                trd_scope: vec![],
                repos: vec![],
            },
            stages: vec![],
            repos: vec![],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_retry_on_compilation_error() {
        let mut status = PipelineStatus::Running;
        let mut stage = make_stage();
        let result =
            handle_stage_error(&mut status, &mut stage, "compilation error", None, None).await;
        assert_eq!(result.issue_class, IssueClass::CompilationError);
        assert_eq!(result.action, ErrorAction::Retry);
        assert_eq!(stage.status, StageStatus::Failed);
        assert_eq!(stage.retry_count, 1);
        assert!(stage.issue.as_ref().unwrap().retryable);
    }

    #[tokio::test]
    async fn test_retry_exhausted_becomes_blocked() {
        let mut status = PipelineStatus::Running;
        let mut stage = make_stage();
        stage.retry_count = 3;
        let result =
            handle_stage_error(&mut status, &mut stage, "compilation error", None, None).await;
        assert_eq!(result.action, ErrorAction::Blocked);
        assert_eq!(stage.status, StageStatus::Blocked);
        assert_eq!(status, PipelineStatus::Blocked);
    }

    #[tokio::test]
    async fn test_llm_rate_limit_retries() {
        let mut status = PipelineStatus::Running;
        let mut stage = make_stage();
        let result =
            handle_stage_error(&mut status, &mut stage, "rate limit reached", None, None).await;
        assert_eq!(result.issue_class, IssueClass::LlmRateLimit);
        assert_eq!(result.action, ErrorAction::Retry);
    }

    #[tokio::test]
    async fn test_llm_rate_limit_exhausted_fails() {
        let mut status = PipelineStatus::Running;
        let mut stage = make_stage();
        stage.retry_count = 5;
        let result =
            handle_stage_error(&mut status, &mut stage, "rate limit reached", None, None).await;
        assert_eq!(result.action, ErrorAction::Failed);
        assert_eq!(status, PipelineStatus::Failed);
    }
}
