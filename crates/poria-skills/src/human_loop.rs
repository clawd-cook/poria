use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use poria_core::types::{Pipeline, Stage};

use crate::error::SkillError;
use crate::fixture::is_fixture_mode;

/// The action a human chose in a human-in-the-loop interaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HumanAction {
    Resume,
    Skip,
    Cancel,
}

/// A parsed human reply containing the detected action and the original text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HumanReply {
    pub action: HumanAction,
    pub raw_message: String,
}

/// Trait for human-in-the-loop coordination.
#[async_trait]
pub trait HumanLoop: Send + Sync {
    /// Notify a human about a stage issue.
    async fn notify(
        &self,
        pipeline: &Pipeline,
        stage: &Stage,
        issue_class: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Send a follow-up notification for an already-notified stage.
    async fn renotify(
        &self,
        pipeline: &Pipeline,
        stage: &Stage,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Escalate a critical message to a human.
    async fn escalate(
        &self,
        pipeline: &Pipeline,
        message: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Poll for a human reply. Returns `None` if no reply is available yet.
    async fn poll_reply(
        &self,
        pipeline: &Pipeline,
    ) -> Result<Option<HumanReply>, Box<dyn std::error::Error + Send + Sync>>;
}

/// Fixture/stub coordinator for human-in-the-loop interactions.
///
/// In fixture mode every method succeeds immediately (no-op for void methods,
/// `None` for poll). Outside fixture mode, methods return an error indicating
/// the real implementation is not yet available.
pub struct HumanLoopCoordinator {
    fixture: bool,
}

impl HumanLoopCoordinator {
    pub fn new() -> Self {
        Self {
            fixture: is_fixture_mode(),
        }
    }
}

impl Default for HumanLoopCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl HumanLoop for HumanLoopCoordinator {
    async fn notify(
        &self,
        _pipeline: &Pipeline,
        _stage: &Stage,
        _issue_class: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.fixture {
            return Ok(());
        }
        Err(Box::new(SkillError::NotImplemented(
            "HumanLoopCoordinator.notify".into(),
        )))
    }

    async fn renotify(
        &self,
        _pipeline: &Pipeline,
        _stage: &Stage,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.fixture {
            return Ok(());
        }
        Err(Box::new(SkillError::NotImplemented(
            "HumanLoopCoordinator.renotify".into(),
        )))
    }

    async fn escalate(
        &self,
        _pipeline: &Pipeline,
        _message: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.fixture {
            return Ok(());
        }
        Err(Box::new(SkillError::NotImplemented(
            "HumanLoopCoordinator.escalate".into(),
        )))
    }

    async fn poll_reply(
        &self,
        _pipeline: &Pipeline,
    ) -> Result<Option<HumanReply>, Box<dyn std::error::Error + Send + Sync>> {
        if self.fixture {
            return Ok(None);
        }
        Err(Box::new(SkillError::NotImplemented(
            "HumanLoopCoordinator.poll_reply".into(),
        )))
    }
}

/// Parse free-form human text into a [`HumanReply`].
///
/// Pattern priority (matching the TypeScript implementation):
/// 1. **Cancel** -- cancel, abort, stop (and CJK equivalents)
/// 2. **Skip**   -- skip, ignore (and CJK equivalents)
/// 3. **Resume** -- fix, retry, resume (and CJK equivalents)
///
/// Returns `None` when the text is empty or matches no known pattern.
pub fn parse_human_reply(text: &str) -> Option<HumanReply> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    let lower = trimmed.to_lowercase();

    // Cancel patterns (checked first -- highest priority)
    if lower.contains("\u{53D6}\u{6D88}")       // 取消
        || lower.contains("cancel")
        || lower.contains("\u{7EC8}\u{6B62}")   // 终止
        || lower.contains("abort")
        || lower.contains("stop")
    {
        return Some(HumanReply {
            action: HumanAction::Cancel,
            raw_message: trimmed.to_string(),
        });
    }

    // Skip patterns
    if lower.contains("\u{8DF3}\u{8FC7}")       // 跳过
        || lower.contains("skip")
        || lower.contains("\u{5FFD}\u{7565}")   // 忽略
        || lower.contains("ignore")
    {
        return Some(HumanReply {
            action: HumanAction::Skip,
            raw_message: trimmed.to_string(),
        });
    }

    // Resume patterns
    if lower.contains("\u{4FEE}\u{590D}")       // 修复
        || lower.contains("fix")
        || lower.contains("\u{5DF2}\u{4FEE}\u{590D}")   // 已修复
        || lower.contains("\u{5DF2}\u{89E3}\u{51B3}")   // 已解决
        || lower.contains("\u{91CD}\u{8BD5}")   // 重试
        || lower.contains("retry")
        || lower.contains("resume")
    {
        return Some(HumanReply {
            action: HumanAction::Resume,
            raw_message: trimmed.to_string(),
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- parse_human_reply tests ---

    #[test]
    fn test_empty_returns_none() {
        assert!(parse_human_reply("").is_none());
        assert!(parse_human_reply("   ").is_none());
    }

    #[test]
    fn test_cancel_patterns() {
        let cases = [
            "\u{53D6}\u{6D88}",
            "cancel",
            "Cancel",
            "\u{7EC8}\u{6B62}",
            "abort",
            "ABORT",
            "stop",
            "Stop",
        ];
        for text in cases {
            let reply = parse_human_reply(text).unwrap();
            assert_eq!(reply.action, HumanAction::Cancel, "failed for: {text}");
        }
    }

    #[test]
    fn test_skip_patterns() {
        let cases = [
            "\u{8DF3}\u{8FC7}",
            "skip",
            "SKIP",
            "\u{5FFD}\u{7565}",
            "ignore",
            "Ignore",
        ];
        for text in cases {
            let reply = parse_human_reply(text).unwrap();
            assert_eq!(reply.action, HumanAction::Skip, "failed for: {text}");
        }
    }

    #[test]
    fn test_resume_patterns() {
        let cases = [
            "\u{4FEE}\u{590D}",
            "fix",
            "FIX",
            "\u{5DF2}\u{4FEE}\u{590D}",
            "\u{5DF2}\u{89E3}\u{51B3}",
            "\u{91CD}\u{8BD5}",
            "retry",
            "resume",
            "Resume",
        ];
        for text in cases {
            let reply = parse_human_reply(text).unwrap();
            assert_eq!(reply.action, HumanAction::Resume, "failed for: {text}");
        }
    }

    #[test]
    fn test_cancel_takes_priority_over_resume() {
        // "cancel" should win even if "fix" is also present
        let reply = parse_human_reply("fix cancel").unwrap();
        assert_eq!(reply.action, HumanAction::Cancel);
    }

    #[test]
    fn test_cancel_takes_priority_over_skip() {
        let reply = parse_human_reply("skip abort").unwrap();
        assert_eq!(reply.action, HumanAction::Cancel);
    }

    #[test]
    fn test_skip_takes_priority_over_resume() {
        let reply = parse_human_reply("fix skip").unwrap();
        assert_eq!(reply.action, HumanAction::Skip);
    }

    #[test]
    fn test_no_match_returns_none() {
        assert!(parse_human_reply("hello world").is_none());
        assert!(parse_human_reply("good morning").is_none());
    }

    #[test]
    fn test_raw_message_is_trimmed() {
        let reply = parse_human_reply("  cancel  ").unwrap();
        assert_eq!(reply.raw_message, "cancel");
    }
}
