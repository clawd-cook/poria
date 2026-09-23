//! Stage-boundary dialogue (方案 A): stop after each successful node until the user advances.

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::types::{Pipeline, PipelineStatus, Stage, StageEnum, StageIssue};

pub const AWAITING_ADVANCE_ISSUE_CLASS: &str = "awaiting_advance";
const AWAITING_ADVANCE_KEY: &str = "awaitingAdvance";

/// Exit-gate strength for stage-boundary dialogue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateLevel {
    /// Soft dialogue; skip allowed without second confirm.
    Dialogue,
    /// Must confirm continue (or explicit force skip).
    Locked,
}

/// Allowed actions on an awaiting-advance dialogue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdvanceAction {
    Continue,
    Redo,
    Skip,
    Annotate,
}

impl AdvanceAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Continue => "continue",
            Self::Redo => "redo",
            Self::Skip => "skip",
            Self::Annotate => "annotate",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim() {
            "continue" => Some(Self::Continue),
            "redo" => Some(Self::Redo),
            "skip" => Some(Self::Skip),
            "annotate" => Some(Self::Annotate),
            _ => None,
        }
    }
}

/// Payload shown in the workbench dialogue surface.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AdvanceDialogue {
    pub pipeline_id: String,
    pub node_id: String,
    pub summary: String,
    pub artifact_paths: Vec<String>,
    pub allowed_actions: Vec<String>,
    pub gate_level: GateLevel,
}

/// Confirm-locked nodes (R4): must not skip without an explicit force flag.
pub fn gate_level_for(stage: StageEnum) -> GateLevel {
    match stage {
        StageEnum::ReviewPrd
        | StageEnum::Design
        | StageEnum::TestPlan
        | StageEnum::RunAutotest
        | StageEnum::HandoffQa
        | StageEnum::Deploy
        | StageEnum::Archive => GateLevel::Locked,
        _ => GateLevel::Dialogue,
    }
}

pub fn default_allowed_actions(gate: GateLevel) -> Vec<String> {
    let mut actions = vec![
        AdvanceAction::Continue.as_str().to_string(),
        AdvanceAction::Redo.as_str().to_string(),
        AdvanceAction::Annotate.as_str().to_string(),
    ];
    if gate == GateLevel::Dialogue {
        actions.push(AdvanceAction::Skip.as_str().to_string());
    } else {
        // Locked gates still expose skip but IPC requires `force: true`.
        actions.push(AdvanceAction::Skip.as_str().to_string());
    }
    actions
}

pub fn build_advance_summary(stage: StageEnum, output: &serde_json::Value) -> String {
    let label = stage.label();
    if let Some(msg) = output
        .get("summary")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return format!("{label} 已完成：{msg}");
    }
    format!("{label} 已完成，请确认后继续下一阶段。")
}

pub fn collect_artifact_paths(output: &serde_json::Value) -> Vec<String> {
    let mut paths = Vec::new();
    if let Some(arr) = output.get("artifactPaths").and_then(|v| v.as_array()) {
        for item in arr {
            if let Some(path) = item.as_str().map(str::trim).filter(|s| !s.is_empty()) {
                paths.push(path.to_string());
            }
        }
    }
    for key in [
        "prdPath",
        "prdReviewPath",
        "trdPath",
        "backendTrdPath",
        "taskPath",
        "crPath",
        "testPlanPath",
        "testCasesPath",
        "handoffReportPath",
        "archivePath",
        "projectDir",
        "workspacePath",
        "worktreePath",
        "mrUrl",
    ] {
        if let Some(path) = output
            .get(key)
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            if !paths.iter().any(|p| p == path) {
                paths.push(path.to_string());
            }
        }
    }
    if let Some(urls) = output.get("mrUrls").and_then(|v| v.as_array()) {
        for url in urls {
            if let Some(path) = url.as_str().map(str::trim).filter(|s| !s.is_empty()) {
                if !paths.iter().any(|p| p == path) {
                    paths.push(path.to_string());
                }
            }
        }
    }
    paths
}

pub fn write_awaiting_advance(stage: &mut Stage, dialogue: &AdvanceDialogue) {
    let mut object = stage
        .output
        .as_ref()
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default();
    object.insert(
        AWAITING_ADVANCE_KEY.into(),
        serde_json::to_value(dialogue).unwrap_or(serde_json::json!({})),
    );
    stage.output = Some(serde_json::Value::Object(object));
    stage.issue = Some(StageIssue {
        class: AWAITING_ADVANCE_ISSUE_CLASS.into(),
        message: dialogue.summary.clone(),
        retryable: true,
    });
}

pub fn read_awaiting_advance(stage: &Stage) -> Option<AdvanceDialogue> {
    stage
        .output
        .as_ref()
        .and_then(|value| value.get(AWAITING_ADVANCE_KEY))
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
}

pub fn clear_awaiting_advance(stage: &mut Stage) {
    if let Some(serde_json::Value::Object(mut object)) = stage.output.clone() {
        object.remove(AWAITING_ADVANCE_KEY);
        stage.output = Some(serde_json::Value::Object(object));
    }
    if stage
        .issue
        .as_ref()
        .is_some_and(|issue| issue.class == AWAITING_ADVANCE_ISSUE_CLASS)
    {
        stage.issue = None;
    }
}

pub fn is_awaiting_advance(stage: &Stage) -> bool {
    stage
        .issue
        .as_ref()
        .is_some_and(|issue| issue.class == AWAITING_ADVANCE_ISSUE_CLASS)
        || read_awaiting_advance(stage).is_some()
}

/// Index of the stage holding an unresolved advance dialogue (usually last completed).
pub fn awaiting_advance_stage_index(pipeline: &Pipeline) -> Option<usize> {
    pipeline
        .stages
        .iter()
        .rposition(|stage| is_awaiting_advance(stage))
}

/// Enter stop-and-talk after a successful stage. Sets pipeline to Blocked.
pub fn enter_awaiting_advance(pipeline: &mut Pipeline, stage_idx: usize) -> AdvanceDialogue {
    let stage_name = pipeline.stages[stage_idx].name;
    let output = pipeline.stages[stage_idx]
        .output
        .clone()
        .unwrap_or(serde_json::json!({}));
    let gate = gate_level_for(stage_name);
    let dialogue = AdvanceDialogue {
        pipeline_id: pipeline.id.clone(),
        node_id: stage_name.as_str().to_string(),
        summary: build_advance_summary(stage_name, &output),
        artifact_paths: collect_artifact_paths(&output),
        allowed_actions: default_allowed_actions(gate),
        gate_level: gate,
    };
    write_awaiting_advance(&mut pipeline.stages[stage_idx], &dialogue);
    // Note was for this completed stage; drop so the next dialogue annotate starts fresh.
    pipeline.config.advance_note = None;
    pipeline.status = PipelineStatus::Blocked;
    pipeline.updated_at = Utc::now();
    dialogue
}

/// Whether production autopilot may chain stages. Always false except fixture env.
pub fn may_auto_chain_stages() -> bool {
    std::env::var("PORIA_PIPELINE_FIXTURE")
        .map(|v| v == "1")
        .unwrap_or(false)
}

/// Human-readable detail for `human:request`.
pub fn advance_request_detail(dialogue: &AdvanceDialogue) -> String {
    let artifacts = if dialogue.artifact_paths.is_empty() {
        String::new()
    } else {
        format!("\n产物：{}", dialogue.artifact_paths.join("、"))
    };
    format!(
        "{}\n可选：{}{}",
        dialogue.summary,
        dialogue.allowed_actions.join(" / "),
        artifacts
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{PipelineConfig, StageStatus, STAGE_ORDER};

    fn pipeline_with_completed(stage: StageEnum) -> Pipeline {
        let stages: Vec<Stage> = STAGE_ORDER
            .iter()
            .map(|name| {
                let status = if *name == stage {
                    StageStatus::Completed
                } else if STAGE_ORDER.iter().position(|s| s == name)
                    < STAGE_ORDER.iter().position(|s| *s == stage)
                {
                    StageStatus::Completed
                } else {
                    StageStatus::Pending
                };
                Stage {
                    id: None,
                    pipeline_id: "pl".into(),
                    name: *name,
                    status,
                    skill_id: None,
                    retry_count: 0,
                    max_retries: 3,
                    input: None,
                    output: if *name == stage {
                        Some(serde_json::json!({
                            "summary": "ok",
                            "projectDir": "/tmp/proj"
                        }))
                    } else {
                        None
                    },
                    gate_results: None,
                    issue: None,
                    rollback: None,
                    agent_session_id: None,
                    started_at: None,
                    completed_at: None,
                }
            })
            .collect();
        Pipeline {
            id: "pl".into(),
            demand_id: 1,
            demand_code: "D1".into(),
            demand_name: None,
            status: PipelineStatus::Running,
            raw_link: String::new(),
            operator: "erp".into(),
            has_regressed: false,
            config: PipelineConfig::default(),
            stages,
            repos: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn locked_gates_cover_confirm_locked_nodes() {
        assert_eq!(gate_level_for(StageEnum::Design), GateLevel::Locked);
        assert_eq!(gate_level_for(StageEnum::TestPlan), GateLevel::Locked);
        assert_eq!(gate_level_for(StageEnum::Deploy), GateLevel::Locked);
        assert_eq!(gate_level_for(StageEnum::Init), GateLevel::Dialogue);
        assert_eq!(gate_level_for(StageEnum::Dev), GateLevel::Dialogue);
        assert_eq!(gate_level_for(StageEnum::Lint), GateLevel::Dialogue);
    }

    #[test]
    fn enter_awaiting_advance_blocks_pipeline() {
        let mut pipeline = pipeline_with_completed(StageEnum::Init);
        let idx = pipeline
            .stages
            .iter()
            .position(|s| s.name == StageEnum::Init)
            .unwrap();
        let dialogue = enter_awaiting_advance(&mut pipeline, idx);
        assert_eq!(pipeline.status, PipelineStatus::Blocked);
        assert_eq!(dialogue.node_id, "init");
        assert!(is_awaiting_advance(&pipeline.stages[idx]));
        assert_eq!(awaiting_advance_stage_index(&pipeline), Some(idx));
    }

    #[test]
    fn enter_awaiting_advance_clears_consumed_note() {
        let mut pipeline = pipeline_with_completed(StageEnum::Init);
        pipeline.config.advance_note = Some("for next".into());
        let idx = pipeline
            .stages
            .iter()
            .position(|s| s.name == StageEnum::Init)
            .unwrap();
        enter_awaiting_advance(&mut pipeline, idx);
        assert!(pipeline.config.advance_note.is_none());
    }

    #[test]
    fn advance_action_parse() {
        assert_eq!(AdvanceAction::parse("continue"), Some(AdvanceAction::Continue));
        assert_eq!(AdvanceAction::parse("redo"), Some(AdvanceAction::Redo));
        assert_eq!(AdvanceAction::parse("nope"), None);
    }
}
