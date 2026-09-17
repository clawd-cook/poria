use crate::types::{PipelineStatus, StageStatus};

#[derive(Debug, thiserror::Error)]
#[error("Invalid {entity_type} transition: {from} → {to}")]
pub struct InvalidTransitionError {
    pub entity_type: String,
    pub from: String,
    pub to: String,
}

const PIPELINE_TRANSITIONS: &[(PipelineStatus, &[PipelineStatus])] = &[
    (
        PipelineStatus::Created,
        &[PipelineStatus::Running, PipelineStatus::Cancelled],
    ),
    (
        PipelineStatus::Running,
        &[
            PipelineStatus::WaitingMerge,
            PipelineStatus::Blocked,
            PipelineStatus::Failed,
            PipelineStatus::Cancelled,
        ],
    ),
    (
        PipelineStatus::Blocked,
        &[PipelineStatus::Running, PipelineStatus::Cancelled],
    ),
    (
        PipelineStatus::WaitingMerge,
        &[
            PipelineStatus::Completed,
            PipelineStatus::Failed,
            PipelineStatus::Cancelled,
        ],
    ),
    (PipelineStatus::Completed, &[]),
    (PipelineStatus::Failed, &[PipelineStatus::Cancelled]),
    (PipelineStatus::Cancelled, &[]),
];

const STAGE_TRANSITIONS: &[(StageStatus, &[StageStatus])] = &[
    (
        StageStatus::Pending,
        &[StageStatus::Running, StageStatus::Skipped],
    ),
    (
        StageStatus::Running,
        &[
            StageStatus::Completed,
            StageStatus::Failed,
            StageStatus::Blocked,
        ],
    ),
    (StageStatus::Completed, &[]),
    (
        StageStatus::Failed,
        &[StageStatus::Running, StageStatus::Blocked],
    ),
    (StageStatus::Blocked, &[StageStatus::Running]),
    (StageStatus::Skipped, &[]),
];

fn allowed_pipeline_transitions(from: PipelineStatus) -> &'static [PipelineStatus] {
    for (status, targets) in PIPELINE_TRANSITIONS {
        if *status == from {
            return targets;
        }
    }
    &[]
}

fn allowed_stage_transitions(from: StageStatus) -> &'static [StageStatus] {
    for (status, targets) in STAGE_TRANSITIONS {
        if *status == from {
            return targets;
        }
    }
    &[]
}

pub fn can_pipeline_transition(from: PipelineStatus, to: PipelineStatus) -> bool {
    allowed_pipeline_transitions(from).contains(&to)
}

pub fn can_stage_transition(from: StageStatus, to: StageStatus) -> bool {
    allowed_stage_transitions(from).contains(&to)
}

pub fn transition_pipeline(
    status: &mut PipelineStatus,
    to: PipelineStatus,
) -> Result<(), InvalidTransitionError> {
    if !can_pipeline_transition(*status, to) {
        return Err(InvalidTransitionError {
            entity_type: "pipeline".into(),
            from: format!("{:?}", status),
            to: format!("{:?}", to),
        });
    }
    *status = to;
    Ok(())
}

pub fn transition_stage(
    status: &mut StageStatus,
    to: StageStatus,
) -> Result<(), InvalidTransitionError> {
    if !can_stage_transition(*status, to) {
        return Err(InvalidTransitionError {
            entity_type: "stage".into(),
            from: format!("{:?}", status),
            to: format!("{:?}", to),
        });
    }
    *status = to;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_created_to_running() {
        assert!(can_pipeline_transition(
            PipelineStatus::Created,
            PipelineStatus::Running
        ));
    }

    #[test]
    fn test_pipeline_created_to_completed_invalid() {
        assert!(!can_pipeline_transition(
            PipelineStatus::Created,
            PipelineStatus::Completed
        ));
    }

    #[test]
    fn test_pipeline_running_to_blocked() {
        assert!(can_pipeline_transition(
            PipelineStatus::Running,
            PipelineStatus::Blocked
        ));
    }

    #[test]
    fn test_pipeline_completed_terminal() {
        assert!(!can_pipeline_transition(
            PipelineStatus::Completed,
            PipelineStatus::Running
        ));
        assert!(!can_pipeline_transition(
            PipelineStatus::Completed,
            PipelineStatus::Cancelled
        ));
    }

    #[test]
    fn test_pipeline_cancelled_terminal() {
        assert!(!can_pipeline_transition(
            PipelineStatus::Cancelled,
            PipelineStatus::Running
        ));
    }

    #[test]
    fn test_pipeline_failed_to_cancelled() {
        assert!(can_pipeline_transition(
            PipelineStatus::Failed,
            PipelineStatus::Cancelled
        ));
    }

    #[test]
    fn test_stage_pending_to_running() {
        assert!(can_stage_transition(
            StageStatus::Pending,
            StageStatus::Running
        ));
    }

    #[test]
    fn test_stage_pending_to_skipped() {
        assert!(can_stage_transition(
            StageStatus::Pending,
            StageStatus::Skipped
        ));
    }

    #[test]
    fn test_stage_running_to_completed() {
        assert!(can_stage_transition(
            StageStatus::Running,
            StageStatus::Completed
        ));
    }

    #[test]
    fn test_stage_completed_terminal() {
        assert!(!can_stage_transition(
            StageStatus::Completed,
            StageStatus::Running
        ));
    }

    #[test]
    fn test_stage_failed_can_retry() {
        assert!(can_stage_transition(
            StageStatus::Failed,
            StageStatus::Running
        ));
    }

    #[test]
    fn test_stage_blocked_to_running() {
        assert!(can_stage_transition(
            StageStatus::Blocked,
            StageStatus::Running
        ));
    }

    #[test]
    fn test_transition_pipeline_mutates() {
        let mut status = PipelineStatus::Created;
        transition_pipeline(&mut status, PipelineStatus::Running).unwrap();
        assert_eq!(status, PipelineStatus::Running);
    }

    #[test]
    fn test_transition_pipeline_invalid_errors() {
        let mut status = PipelineStatus::Completed;
        let err = transition_pipeline(&mut status, PipelineStatus::Running).unwrap_err();
        assert!(err.to_string().contains("Invalid pipeline transition"));
    }

    #[test]
    fn test_transition_stage_mutates() {
        let mut status = StageStatus::Pending;
        transition_stage(&mut status, StageStatus::Running).unwrap();
        assert_eq!(status, StageStatus::Running);
    }
}
