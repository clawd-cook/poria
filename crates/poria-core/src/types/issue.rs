use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Stored on `Stage.issue.class` when SSO cookie is missing or rejected.
pub const AUTH_EXPIRED_ISSUE_CLASS: &str = "auth_expired";

/// User-facing copy for 401 / missing cookie (Xingyun / JoySpace / Coding).
pub const AUTH_EXPIRED_USER_MESSAGE: &str = "SSO Cookie 已过期，请重新登录";

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
    Unknown,
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
