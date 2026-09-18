use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use poria_core::pipeline::{
    blocked_stage_index, hitl_escalate_message, hitl_notify_message, jme_notify_target,
    read_human_loop_state,
};
use poria_core::types::{Pipeline, Stage};

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

/// Coordinates desktop + 京ME HITL. Fixture mode is a no-op; otherwise notify/read
/// go through JoyClaw (`poria-channels` JME). Gateway-down is best-effort.
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

fn stage_label(stage: &Stage) -> String {
    serde_json::to_value(stage.name)
        .ok()
        .and_then(|value| value.as_str().map(ToOwned::to_owned))
        .unwrap_or_else(|| format!("{:?}", stage.name))
}

fn notify_target_for(pipeline: &Pipeline, issue_class: &str, stage: Option<&Stage>) -> String {
    stage
        .map(read_human_loop_state)
        .and_then(|state| state.notify_target)
        .filter(|target| !target.trim().is_empty())
        .unwrap_or_else(|| jme_notify_target(pipeline, issue_class))
}

#[async_trait]
impl HumanLoop for HumanLoopCoordinator {
    async fn notify(
        &self,
        pipeline: &Pipeline,
        stage: &Stage,
        issue_class: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.fixture {
            return Ok(());
        }
        let target = jme_notify_target(pipeline, issue_class);
        let detail = stage
            .issue
            .as_ref()
            .map(|issue| issue.message.as_str())
            .unwrap_or("");
        let message = hitl_notify_message(pipeline, &stage_label(stage), issue_class, detail);
        match poria_channels::jme::send_message(&target, &message, Some(120), None).await {
            Ok(_) => Ok(()),
            Err(err) => {
                tracing::warn!(error = %err, target, "JME HITL notify failed");
                Err(err)
            }
        }
    }

    async fn renotify(
        &self,
        pipeline: &Pipeline,
        stage: &Stage,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let issue_class = stage
            .issue
            .as_ref()
            .map(|issue| issue.class.as_str())
            .unwrap_or("unknown");
        self.notify(pipeline, stage, issue_class).await
    }

    async fn escalate(
        &self,
        pipeline: &Pipeline,
        message: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.fixture {
            return Ok(());
        }
        let stage = blocked_stage_index(pipeline).and_then(|idx| pipeline.stages.get(idx));
        let issue_class = stage
            .and_then(|s| s.issue.as_ref())
            .map(|issue| issue.class.as_str())
            .unwrap_or("unknown");
        let target = notify_target_for(pipeline, issue_class, stage);
        let body = hitl_escalate_message(pipeline, message);
        match poria_channels::jme::send_message(&target, &body, Some(120), None).await {
            Ok(_) => Ok(()),
            Err(err) => {
                tracing::warn!(error = %err, target, "JME HITL escalate failed");
                Err(err)
            }
        }
    }

    async fn poll_reply(
        &self,
        pipeline: &Pipeline,
    ) -> Result<Option<HumanReply>, Box<dyn std::error::Error + Send + Sync>> {
        if self.fixture {
            return Ok(None);
        }
        let Some(idx) = blocked_stage_index(pipeline) else {
            return Ok(None);
        };
        let stage = &pipeline.stages[idx];
        let state = read_human_loop_state(stage);
        if state.reply_consumed || state.manual_at.is_some() {
            return Ok(None);
        }
        let issue_class = stage
            .issue
            .as_ref()
            .map(|issue| issue.class.as_str())
            .unwrap_or("unknown");
        let chat = notify_target_for(pipeline, issue_class, Some(stage));
        let since = state
            .since
            .or(state.notified_at)
            .unwrap_or_else(|| pipeline.updated_at.to_rfc3339());
        let replies = match poria_channels::jme::read_replies(&chat, &since, Some(120), None).await
        {
            Ok(replies) => replies,
            Err(err) => {
                tracing::debug!(error = %err, "JME HITL poll skipped");
                return Ok(None);
            }
        };
        for text in replies {
            if let Some(reply) = parse_human_reply(&text) {
                return Ok(Some(reply));
            }
        }
        Ok(None)
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

    fn blocked_pipeline() -> Pipeline {
        use chrono::Utc;
        use poria_core::types::{
            PipelineConfig, PipelineStatus, StageEnum, StageIssue, StageStatus,
        };
        Pipeline {
            id: "pl-hitl".into(),
            demand_id: 1,
            demand_code: "D1".into(),
            demand_name: None,
            status: PipelineStatus::Blocked,
            raw_link: String::new(),
            operator: "erp.li".into(),
            has_regressed: false,
            config: PipelineConfig::default(),
            stages: vec![Stage {
                id: None,
                pipeline_id: "pl-hitl".into(),
                name: StageEnum::Design,
                status: StageStatus::Blocked,
                skill_id: None,
                retry_count: 0,
                max_retries: 3,
                input: None,
                output: None,
                gate_results: None,
                issue: Some(StageIssue {
                    class: "requirement_ambiguous".into(),
                    message: "P0 unanswered".into(),
                    retryable: true,
                }),
                rollback: None,
                agent_session_id: None,
                started_at: None,
                completed_at: None,
            }],
            repos: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn notify_is_implemented_via_jme_fixture() {
        std::env::set_var("PORIA_JME_FIXTURE", "1");
        let coordinator = HumanLoopCoordinator { fixture: false };
        let pipeline = blocked_pipeline();
        let result = coordinator
            .notify(&pipeline, &pipeline.stages[0], "requirement_ambiguous")
            .await;
        std::env::remove_var("PORIA_JME_FIXTURE");
        assert!(result.is_ok(), "{result:?}");
    }

    #[tokio::test]
    async fn poll_reply_parses_jme_fixture_resume() {
        std::env::set_var("PORIA_JME_FIXTURE", "1");
        std::env::set_var("PORIA_JME_FIXTURE_REPLIES", "已修复");
        let coordinator = HumanLoopCoordinator { fixture: false };
        let reply = coordinator.poll_reply(&blocked_pipeline()).await.unwrap();
        std::env::remove_var("PORIA_JME_FIXTURE_REPLIES");
        std::env::remove_var("PORIA_JME_FIXTURE");
        assert_eq!(reply.unwrap().action, HumanAction::Resume);
    }
}
