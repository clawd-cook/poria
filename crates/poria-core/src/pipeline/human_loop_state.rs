use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::types::{IssueClass, Pipeline, Stage, StageStatus, ISSUE_POLICIES};

const HUMAN_LOOP_KEY: &str = "humanLoop";

/// Persisted HITL bookkeeping on the blocked stage output.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HumanLoopState {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notified_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notify_target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub escalated_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manual_at: Option<String>,
    #[serde(default)]
    pub reply_consumed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub since: Option<String>,
}

pub fn parse_duration_spec(spec: &str) -> Duration {
    let spec = spec.trim().to_ascii_lowercase();
    if let Some(num) = spec.strip_suffix('h') {
        if let Ok(n) = num.parse::<u64>() {
            return Duration::from_secs(n.saturating_mul(3600));
        }
    }
    if let Some(num) = spec.strip_suffix('m') {
        if let Ok(n) = num.parse::<u64>() {
            return Duration::from_secs(n.saturating_mul(60));
        }
    }
    if let Some(num) = spec.strip_suffix('s') {
        if let Ok(n) = num.parse::<u64>() {
            return Duration::from_secs(n);
        }
    }
    Duration::ZERO
}

pub fn escalate_after(issue_class: &str) -> Option<Duration> {
    ISSUE_POLICIES
        .get(&IssueClass::from_key(issue_class))
        .and_then(|policy| policy.escalate_at.as_deref())
        .map(parse_duration_spec)
        .filter(|d| !d.is_zero())
}

pub fn jme_notify_target(pipeline: &Pipeline, issue_class: &str) -> String {
    let class = IssueClass::from_key(issue_class);
    let role = ISSUE_POLICIES
        .get(&class)
        .and_then(|policy| policy.notify_roles.first())
        .map(String::as_str)
        .unwrap_or("developer");
    match role {
        "developer" => {
            let operator = pipeline.operator.trim();
            if operator.is_empty() {
                "developer".into()
            } else {
                operator.to_string()
            }
        }
        other => other.to_string(),
    }
}

pub fn hitl_notify_message(
    pipeline: &Pipeline,
    stage_name: &str,
    issue_class: &str,
    detail: &str,
) -> String {
    format!(
        "【Poria】流水线 {} ({}) 阶段 {} 需要处理。问题: {}。{}\n回复「修复」继续、「跳过」跳过该阶段、「取消」终止流水线。",
        pipeline.demand_code,
        pipeline.id,
        stage_name,
        issue_class,
        detail.trim()
    )
}

pub fn hitl_escalate_message(pipeline: &Pipeline, detail: &str) -> String {
    format!(
        "【Poria 升级】流水线 {} ({}) 超时未回复。{}\n回复「修复」/「跳过」/「取消」。超时后将降级为手动处理。",
        pipeline.demand_code,
        pipeline.id,
        detail.trim()
    )
}

pub fn blocked_stage_index(pipeline: &Pipeline) -> Option<usize> {
    if let Some(idx) = super::advance::awaiting_advance_stage_index(pipeline) {
        return Some(idx);
    }
    pipeline
        .stages
        .iter()
        .position(|stage| stage.status == StageStatus::Blocked)
        .or_else(|| {
            pipeline
                .stages
                .iter()
                .position(|stage| stage.status == StageStatus::Failed && stage.issue.is_some())
        })
}

pub fn read_human_loop_state(stage: &Stage) -> HumanLoopState {
    stage
        .output
        .as_ref()
        .and_then(|value| value.get(HUMAN_LOOP_KEY))
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default()
}

pub fn write_human_loop_state(stage: &mut Stage, state: &HumanLoopState) {
    let mut object = stage
        .output
        .as_ref()
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default();
    object.insert(
        HUMAN_LOOP_KEY.into(),
        serde_json::to_value(state).unwrap_or(serde_json::json!({})),
    );
    stage.output = Some(serde_json::Value::Object(object));
}

/// First caller wins. Returns false if a desktop or JME reply was already applied.
pub fn try_consume_human_reply(stage: &mut Stage) -> bool {
    let mut state = read_human_loop_state(stage);
    if state.reply_consumed {
        return false;
    }
    state.reply_consumed = true;
    write_human_loop_state(stage, &state);
    true
}

pub fn stamp_human_loop_notified(stage: &mut Stage, target: &str, at: DateTime<Utc>) {
    let stamp = at.to_rfc3339();
    write_human_loop_state(
        stage,
        &HumanLoopState {
            notified_at: Some(stamp.clone()),
            notify_target: Some(target.to_string()),
            escalated_at: None,
            manual_at: None,
            reply_consumed: false,
            since: Some(stamp),
        },
    );
}

pub fn rfc3339_elapsed(stamp: &str, now: DateTime<Utc>) -> Option<Duration> {
    DateTime::parse_from_rfc3339(stamp).ok().map(|parsed| {
        now.signed_duration_since(parsed.with_timezone(&Utc))
            .to_std()
            .unwrap_or_default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{PipelineConfig, PipelineStatus, StageEnum};

    fn stage() -> Stage {
        Stage {
            id: None,
            pipeline_id: "pl".into(),
            name: StageEnum::Dev,
            status: StageStatus::Blocked,
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

    #[test]
    fn consume_reply_once() {
        let mut stage = stage();
        assert!(try_consume_human_reply(&mut stage));
        assert!(!try_consume_human_reply(&mut stage));
        assert!(read_human_loop_state(&stage).reply_consumed);
    }

    #[test]
    fn developer_target_uses_operator() {
        let pipeline = Pipeline {
            id: "pl".into(),
            demand_id: 1,
            demand_code: "D1".into(),
            demand_name: None,
            status: PipelineStatus::Blocked,
            raw_link: String::new(),
            operator: "erp.li".into(),
            has_regressed: false,
            config: PipelineConfig::default(),
            stages: vec![],
            repos: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        assert_eq!(jme_notify_target(&pipeline, "compilation_error"), "erp.li");
        assert_eq!(
            jme_notify_target(&pipeline, "requirement_ambiguous"),
            "product"
        );
    }

    #[test]
    fn escalate_after_parses_hours() {
        assert_eq!(
            escalate_after("compilation_error"),
            Some(Duration::from_secs(2 * 3600))
        );
        assert_eq!(escalate_after("llm_rate_limit"), None);
    }
}
