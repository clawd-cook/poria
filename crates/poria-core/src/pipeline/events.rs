use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::types::{DemandMetadata, GateResult, IssueClass, StageEnum};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PipelineEvent {
    PipelineCreated {
        pipeline_id: String,
        timestamp: DateTime<Utc>,
        demand_ref: Box<DemandMetadata>,
    },
    PipelineStarted {
        pipeline_id: String,
        timestamp: DateTime<Utc>,
    },
    PipelineCompleted {
        pipeline_id: String,
        timestamp: DateTime<Utc>,
    },
    PipelineFailed {
        pipeline_id: String,
        timestamp: DateTime<Utc>,
        reason: String,
    },
    PipelineCancelled {
        pipeline_id: String,
        timestamp: DateTime<Utc>,
        operator: String,
    },
    PipelineWaitingMerge {
        pipeline_id: String,
        timestamp: DateTime<Utc>,
        mr_urls: Vec<String>,
    },

    StageStarted {
        pipeline_id: String,
        timestamp: DateTime<Utc>,
        stage: StageEnum,
    },
    StageCompleted {
        pipeline_id: String,
        timestamp: DateTime<Utc>,
        stage: StageEnum,
        output: serde_json::Value,
    },
    StageFailed {
        pipeline_id: String,
        timestamp: DateTime<Utc>,
        stage: StageEnum,
        error: String,
        retry_count: i32,
    },
    StageBlocked {
        pipeline_id: String,
        timestamp: DateTime<Utc>,
        stage: StageEnum,
        issue_class: IssueClass,
    },
    StageResumed {
        pipeline_id: String,
        timestamp: DateTime<Utc>,
        stage: StageEnum,
        resolution: String,
    },
    StageRegressed {
        pipeline_id: String,
        timestamp: DateTime<Utc>,
        from: StageEnum,
        to: StageEnum,
        reason: String,
    },

    AgentDispatched {
        pipeline_id: String,
        timestamp: DateTime<Utc>,
        stage: StageEnum,
        session_id: String,
    },
    AgentProgress {
        pipeline_id: String,
        timestamp: DateTime<Utc>,
        stage: StageEnum,
        message: String,
    },
    AgentCompleted {
        pipeline_id: String,
        timestamp: DateTime<Utc>,
        stage: StageEnum,
        cost_usd: f64,
    },
    AgentFailed {
        pipeline_id: String,
        timestamp: DateTime<Utc>,
        stage: StageEnum,
        error: String,
    },

    GateEvaluated {
        pipeline_id: String,
        timestamp: DateTime<Utc>,
        stage: StageEnum,
        results: Vec<GateResult>,
    },
    GateRegressTriggered {
        pipeline_id: String,
        timestamp: DateTime<Utc>,
        rule: String,
        from: StageEnum,
        to: StageEnum,
    },

    HumanAssistRequested {
        pipeline_id: String,
        timestamp: DateTime<Utc>,
        issue_class: IssueClass,
        target: String,
    },
    HumanAssistReceived {
        pipeline_id: String,
        timestamp: DateTime<Utc>,
        action: String,
        message: String,
    },
    HumanAssistEscalated {
        pipeline_id: String,
        timestamp: DateTime<Utc>,
        level: i32,
    },

    CredentialRefreshed {
        pipeline_id: String,
        timestamp: DateTime<Utc>,
    },
    CredentialExpired {
        pipeline_id: String,
        timestamp: DateTime<Utc>,
        stage: StageEnum,
    },

    GitCommit {
        pipeline_id: String,
        timestamp: DateTime<Utc>,
        repo: String,
        hash: String,
    },
    GitPush {
        pipeline_id: String,
        timestamp: DateTime<Utc>,
        repo: String,
        branch: String,
    },
    MrCreated {
        pipeline_id: String,
        timestamp: DateTime<Utc>,
        repo: String,
        url: String,
        iid: i64,
    },
    MrMerged {
        pipeline_id: String,
        timestamp: DateTime<Utc>,
        repo: String,
        url: String,
    },
    WorktreeCreated {
        pipeline_id: String,
        timestamp: DateTime<Utc>,
        repo: String,
        path: String,
    },
    WorktreeCleaned {
        pipeline_id: String,
        timestamp: DateTime<Utc>,
        repo: String,
        reason: String,
    },
    RollbackExecuted {
        pipeline_id: String,
        timestamp: DateTime<Utc>,
        #[serde(rename = "type")]
        rollback_type: String,
        detail: String,
    },
}

fn now() -> DateTime<Utc> {
    Utc::now()
}

impl PipelineEvent {
    pub fn pipeline_created(pipeline_id: impl Into<String>, demand_ref: DemandMetadata) -> Self {
        Self::PipelineCreated {
            pipeline_id: pipeline_id.into(),
            timestamp: now(),
            demand_ref: Box::new(demand_ref),
        }
    }
    pub fn pipeline_started(pipeline_id: impl Into<String>) -> Self {
        Self::PipelineStarted {
            pipeline_id: pipeline_id.into(),
            timestamp: now(),
        }
    }
    pub fn pipeline_completed(pipeline_id: impl Into<String>) -> Self {
        Self::PipelineCompleted {
            pipeline_id: pipeline_id.into(),
            timestamp: now(),
        }
    }
    pub fn pipeline_failed(pipeline_id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::PipelineFailed {
            pipeline_id: pipeline_id.into(),
            timestamp: now(),
            reason: reason.into(),
        }
    }
    pub fn pipeline_cancelled(pipeline_id: impl Into<String>, operator: impl Into<String>) -> Self {
        Self::PipelineCancelled {
            pipeline_id: pipeline_id.into(),
            timestamp: now(),
            operator: operator.into(),
        }
    }
    pub fn pipeline_waiting_merge(pipeline_id: impl Into<String>, mr_urls: Vec<String>) -> Self {
        Self::PipelineWaitingMerge {
            pipeline_id: pipeline_id.into(),
            timestamp: now(),
            mr_urls,
        }
    }

    pub fn stage_started(pipeline_id: impl Into<String>, stage: StageEnum) -> Self {
        Self::StageStarted {
            pipeline_id: pipeline_id.into(),
            timestamp: now(),
            stage,
        }
    }
    pub fn stage_completed(
        pipeline_id: impl Into<String>,
        stage: StageEnum,
        output: serde_json::Value,
    ) -> Self {
        Self::StageCompleted {
            pipeline_id: pipeline_id.into(),
            timestamp: now(),
            stage,
            output,
        }
    }
    pub fn stage_failed(
        pipeline_id: impl Into<String>,
        stage: StageEnum,
        error: impl Into<String>,
        retry_count: i32,
    ) -> Self {
        Self::StageFailed {
            pipeline_id: pipeline_id.into(),
            timestamp: now(),
            stage,
            error: error.into(),
            retry_count,
        }
    }
    pub fn stage_blocked(
        pipeline_id: impl Into<String>,
        stage: StageEnum,
        issue_class: IssueClass,
    ) -> Self {
        Self::StageBlocked {
            pipeline_id: pipeline_id.into(),
            timestamp: now(),
            stage,
            issue_class,
        }
    }
    pub fn stage_resumed(
        pipeline_id: impl Into<String>,
        stage: StageEnum,
        resolution: impl Into<String>,
    ) -> Self {
        Self::StageResumed {
            pipeline_id: pipeline_id.into(),
            timestamp: now(),
            stage,
            resolution: resolution.into(),
        }
    }
    pub fn stage_regressed(
        pipeline_id: impl Into<String>,
        from: StageEnum,
        to: StageEnum,
        reason: impl Into<String>,
    ) -> Self {
        Self::StageRegressed {
            pipeline_id: pipeline_id.into(),
            timestamp: now(),
            from,
            to,
            reason: reason.into(),
        }
    }

    pub fn agent_dispatched(
        pipeline_id: impl Into<String>,
        stage: StageEnum,
        session_id: impl Into<String>,
    ) -> Self {
        Self::AgentDispatched {
            pipeline_id: pipeline_id.into(),
            timestamp: now(),
            stage,
            session_id: session_id.into(),
        }
    }
    pub fn agent_progress(
        pipeline_id: impl Into<String>,
        stage: StageEnum,
        message: impl Into<String>,
    ) -> Self {
        Self::AgentProgress {
            pipeline_id: pipeline_id.into(),
            timestamp: now(),
            stage,
            message: message.into(),
        }
    }
    pub fn agent_completed(
        pipeline_id: impl Into<String>,
        stage: StageEnum,
        cost_usd: f64,
    ) -> Self {
        Self::AgentCompleted {
            pipeline_id: pipeline_id.into(),
            timestamp: now(),
            stage,
            cost_usd,
        }
    }
    pub fn agent_failed(
        pipeline_id: impl Into<String>,
        stage: StageEnum,
        error: impl Into<String>,
    ) -> Self {
        Self::AgentFailed {
            pipeline_id: pipeline_id.into(),
            timestamp: now(),
            stage,
            error: error.into(),
        }
    }

    pub fn gate_evaluated(
        pipeline_id: impl Into<String>,
        stage: StageEnum,
        results: Vec<GateResult>,
    ) -> Self {
        Self::GateEvaluated {
            pipeline_id: pipeline_id.into(),
            timestamp: now(),
            stage,
            results,
        }
    }
    pub fn gate_regress_triggered(
        pipeline_id: impl Into<String>,
        rule: impl Into<String>,
        from: StageEnum,
        to: StageEnum,
    ) -> Self {
        Self::GateRegressTriggered {
            pipeline_id: pipeline_id.into(),
            timestamp: now(),
            rule: rule.into(),
            from,
            to,
        }
    }

    pub fn human_assist_requested(
        pipeline_id: impl Into<String>,
        issue_class: IssueClass,
        target: impl Into<String>,
    ) -> Self {
        Self::HumanAssistRequested {
            pipeline_id: pipeline_id.into(),
            timestamp: now(),
            issue_class,
            target: target.into(),
        }
    }
    pub fn human_assist_received(
        pipeline_id: impl Into<String>,
        action: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::HumanAssistReceived {
            pipeline_id: pipeline_id.into(),
            timestamp: now(),
            action: action.into(),
            message: message.into(),
        }
    }
    pub fn human_assist_escalated(pipeline_id: impl Into<String>, level: i32) -> Self {
        Self::HumanAssistEscalated {
            pipeline_id: pipeline_id.into(),
            timestamp: now(),
            level,
        }
    }

    pub fn credential_refreshed(pipeline_id: impl Into<String>) -> Self {
        Self::CredentialRefreshed {
            pipeline_id: pipeline_id.into(),
            timestamp: now(),
        }
    }
    pub fn credential_expired(pipeline_id: impl Into<String>, stage: StageEnum) -> Self {
        Self::CredentialExpired {
            pipeline_id: pipeline_id.into(),
            timestamp: now(),
            stage,
        }
    }

    pub fn git_commit(
        pipeline_id: impl Into<String>,
        repo: impl Into<String>,
        hash: impl Into<String>,
    ) -> Self {
        Self::GitCommit {
            pipeline_id: pipeline_id.into(),
            timestamp: now(),
            repo: repo.into(),
            hash: hash.into(),
        }
    }
    pub fn git_push(
        pipeline_id: impl Into<String>,
        repo: impl Into<String>,
        branch: impl Into<String>,
    ) -> Self {
        Self::GitPush {
            pipeline_id: pipeline_id.into(),
            timestamp: now(),
            repo: repo.into(),
            branch: branch.into(),
        }
    }
    pub fn mr_created(
        pipeline_id: impl Into<String>,
        repo: impl Into<String>,
        url: impl Into<String>,
        iid: i64,
    ) -> Self {
        Self::MrCreated {
            pipeline_id: pipeline_id.into(),
            timestamp: now(),
            repo: repo.into(),
            url: url.into(),
            iid,
        }
    }
    pub fn mr_merged(
        pipeline_id: impl Into<String>,
        repo: impl Into<String>,
        url: impl Into<String>,
    ) -> Self {
        Self::MrMerged {
            pipeline_id: pipeline_id.into(),
            timestamp: now(),
            repo: repo.into(),
            url: url.into(),
        }
    }
    pub fn worktree_created(
        pipeline_id: impl Into<String>,
        repo: impl Into<String>,
        path: impl Into<String>,
    ) -> Self {
        Self::WorktreeCreated {
            pipeline_id: pipeline_id.into(),
            timestamp: now(),
            repo: repo.into(),
            path: path.into(),
        }
    }
    pub fn worktree_cleaned(
        pipeline_id: impl Into<String>,
        repo: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::WorktreeCleaned {
            pipeline_id: pipeline_id.into(),
            timestamp: now(),
            repo: repo.into(),
            reason: reason.into(),
        }
    }
    pub fn rollback_executed(
        pipeline_id: impl Into<String>,
        rollback_type: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self::RollbackExecuted {
            pipeline_id: pipeline_id.into(),
            timestamp: now(),
            rollback_type: rollback_type.into(),
            detail: detail.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_serialization_tag() {
        let event = PipelineEvent::pipeline_started("pl-123");
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["kind"], "pipeline_started");
        assert_eq!(json["pipeline_id"], "pl-123");
        assert!(json["timestamp"].is_string());
    }

    #[test]
    fn test_stage_event_serialization() {
        let event = PipelineEvent::stage_started("pl-123", StageEnum::Dev);
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["kind"], "stage_started");
        assert_eq!(json["stage"], "dev");
    }

    #[test]
    fn test_event_roundtrip() {
        let event = PipelineEvent::pipeline_failed("pl-456", "timeout");
        let json_str = serde_json::to_string(&event).unwrap();
        let deserialized: PipelineEvent = serde_json::from_str(&json_str).unwrap();
        if let PipelineEvent::PipelineFailed { reason, .. } = deserialized {
            assert_eq!(reason, "timeout");
        } else {
            panic!("Wrong variant");
        }
    }
}
