use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Stored on `Stage.issue.class` when SSO cookie is missing or rejected.
pub const AUTH_EXPIRED_ISSUE_CLASS: &str = "auth_expired";

/// User-facing copy for 401 / missing cookie (Xingyun / JoySpace / Coding).
pub const AUTH_EXPIRED_USER_MESSAGE: &str = "SSO Cookie 已过期，请重新登录";

/// Stored on `Stage.issue.class` when Design is blocked on unanswered P0.
pub const REQUIREMENT_AMBIGUOUS_ISSUE_CLASS: &str = "requirement_ambiguous";

/// Stored on `Stage.issue.class` when Dev is blocked pending TRD confirmation.
pub const TRD_UNCONFIRMED_ISSUE_CLASS: &str = "trd_unconfirmed";
pub const TRD_UNCONFIRMED_USER_MESSAGE: &str = "TRD unconfirmed: 请确认前端 TRD.md 后再进入开发";

/// Stored when Dev OutputGuard Block violations (out-of-scope files).
pub const OUT_OF_SCOPE_ISSUE_CLASS: &str = "out_of_scope_change";

/// Stored when Dev OutputGuard Block violations (blocked dependencies).
pub const SECURITY_VIOLATION_ISSUE_CLASS: &str = "security_violation";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueClass {
    CompilationError,
    TestFailure,
    AgentTimeout,
    LlmRateLimit,
    RequirementAmbiguous,
    PrdInvalid,
    MergeConflict,
    LowCrScore,
    DiffTooLarge,
    PermissionDenied,
    InfraFailure,
    SecurityViolation,
    OutOfScopeChange,
    AuthExpired,
    TrdUnconfirmed,
    Unknown,
}

impl IssueClass {
    pub fn as_str(self) -> &'static str {
        match self {
            IssueClass::CompilationError => "compilation_error",
            IssueClass::TestFailure => "test_failure",
            IssueClass::AgentTimeout => "agent_timeout",
            IssueClass::LlmRateLimit => "llm_rate_limit",
            IssueClass::RequirementAmbiguous => "requirement_ambiguous",
            IssueClass::PrdInvalid => "prd_invalid",
            IssueClass::MergeConflict => "merge_conflict",
            IssueClass::LowCrScore => "low_cr_score",
            IssueClass::DiffTooLarge => "diff_too_large",
            IssueClass::PermissionDenied => "permission_denied",
            IssueClass::InfraFailure => "infra_failure",
            IssueClass::SecurityViolation => "security_violation",
            IssueClass::OutOfScopeChange => "out_of_scope_change",
            IssueClass::AuthExpired => "auth_expired",
            IssueClass::TrdUnconfirmed => "trd_unconfirmed",
            IssueClass::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssuePolicy {
    pub auto_retry: i32,
    pub notify_roles: Vec<String>,
    pub escalate_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_delay: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

pub static ISSUE_POLICIES: Lazy<HashMap<IssueClass, IssuePolicy>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert(
        IssueClass::CompilationError,
        IssuePolicy {
            auto_retry: 3,
            notify_roles: vec!["developer".into()],
            escalate_at: Some("2h".into()),
            retry_delay: None,
            note: None,
        },
    );
    m.insert(
        IssueClass::TestFailure,
        IssuePolicy {
            auto_retry: 3,
            notify_roles: vec!["developer".into()],
            escalate_at: Some("2h".into()),
            retry_delay: None,
            note: None,
        },
    );
    m.insert(
        IssueClass::AgentTimeout,
        IssuePolicy {
            auto_retry: 1,
            notify_roles: vec!["developer".into()],
            escalate_at: Some("1h".into()),
            retry_delay: None,
            note: None,
        },
    );
    m.insert(
        IssueClass::LlmRateLimit,
        IssuePolicy {
            auto_retry: 5,
            notify_roles: vec![],
            escalate_at: None,
            retry_delay: Some("5m".into()),
            note: None,
        },
    );
    m.insert(
        IssueClass::RequirementAmbiguous,
        IssuePolicy {
            auto_retry: 0,
            notify_roles: vec!["product".into()],
            escalate_at: Some("4h".into()),
            retry_delay: None,
            note: None,
        },
    );
    m.insert(
        IssueClass::PrdInvalid,
        IssuePolicy {
            auto_retry: 0,
            notify_roles: vec!["product".into()],
            escalate_at: Some("4h".into()),
            retry_delay: None,
            note: None,
        },
    );
    m.insert(
        IssueClass::MergeConflict,
        IssuePolicy {
            auto_retry: 0,
            notify_roles: vec!["developer".into()],
            escalate_at: Some("2h".into()),
            retry_delay: None,
            note: None,
        },
    );
    m.insert(
        IssueClass::LowCrScore,
        IssuePolicy {
            auto_retry: 1,
            notify_roles: vec!["developer".into()],
            escalate_at: Some("4h".into()),
            retry_delay: None,
            note: None,
        },
    );
    m.insert(
        IssueClass::DiffTooLarge,
        IssuePolicy {
            auto_retry: 0,
            notify_roles: vec!["developer".into()],
            escalate_at: Some("4h".into()),
            retry_delay: None,
            note: None,
        },
    );
    m.insert(
        IssueClass::PermissionDenied,
        IssuePolicy {
            auto_retry: 0,
            notify_roles: vec!["ops".into()],
            escalate_at: Some("1h".into()),
            retry_delay: None,
            note: None,
        },
    );
    m.insert(
        IssueClass::InfraFailure,
        IssuePolicy {
            auto_retry: 0,
            notify_roles: vec!["ops".into()],
            escalate_at: Some("1h".into()),
            retry_delay: None,
            note: None,
        },
    );
    m.insert(
        IssueClass::SecurityViolation,
        IssuePolicy {
            auto_retry: 0,
            notify_roles: vec!["developer".into(), "security".into()],
            escalate_at: Some("1h".into()),
            retry_delay: None,
            note: None,
        },
    );
    m.insert(
        IssueClass::OutOfScopeChange,
        IssuePolicy {
            auto_retry: 0,
            notify_roles: vec!["developer".into()],
            escalate_at: Some("2h".into()),
            retry_delay: None,
            note: None,
        },
    );
    m.insert(
        IssueClass::AuthExpired,
        IssuePolicy {
            auto_retry: 0,
            notify_roles: vec!["developer".into()],
            escalate_at: Some("1h".into()),
            retry_delay: None,
            note: Some("静默重读 auth.json 后仍失败则 Blocked，等待重新登录".into()),
        },
    );
    m.insert(
        IssueClass::TrdUnconfirmed,
        IssuePolicy {
            auto_retry: 0,
            notify_roles: vec!["developer".into()],
            escalate_at: Some("4h".into()),
            retry_delay: None,
            note: Some("确认前端 TRD.md 或明确跳过确认后再进入 Dev".into()),
        },
    );
    m.insert(
        IssueClass::Unknown,
        IssuePolicy {
            auto_retry: 0,
            notify_roles: vec!["developer".into()],
            escalate_at: Some("2h".into()),
            retry_delay: None,
            note: None,
        },
    );
    m
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_str_is_snake_case() {
        assert_eq!(IssueClass::CompilationError.as_str(), "compilation_error");
        assert_eq!(IssueClass::LowCrScore.as_str(), "low_cr_score");
        assert_eq!(IssueClass::AuthExpired.as_str(), AUTH_EXPIRED_ISSUE_CLASS);
    }
}
