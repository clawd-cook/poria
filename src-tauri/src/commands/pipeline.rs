use chrono::Utc;
use serde::Serialize;
use tauri::Emitter;
use tauri::State;

use poria_channels::xingyun::parse_xingyun_demand_url;
use poria_core::pipeline::{create_pipeline_id, PipelineEvent as CorePipelineEvent};
use poria_core::types::{
    Pipeline, PipelineConfig, PipelineStatus, Stage, StageStatus, STAGE_ORDER,
};

use crate::AppState;

/// Frontend-facing summary of a pipeline (list view).
#[derive(Debug, Serialize, Clone)]
pub struct PipelineSummary {
    pub id: String,
    pub demand_name: String,
    pub demand_code: String,
    pub status: String,
    pub current_stage: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Frontend-facing stage detail.
#[derive(Debug, Serialize, Clone)]
pub struct StageDetail {
    pub name: String,
    pub status: String,
    pub retry_count: i32,
    pub output_summary: Option<String>,
    pub gate_results: Option<String>,
    pub issue: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

/// Frontend-facing pipeline detail (detail view).
#[derive(Debug, Serialize, Clone)]
pub struct PipelineDetail {
    pub id: String,
    pub demand_id: i64,
    pub demand_code: String,
    pub demand_name: String,
    pub status: String,
    pub raw_link: String,
    pub operator: String,
    pub has_regressed: bool,
    pub stages: Vec<StageDetail>,
    pub created_at: String,
    pub updated_at: String,
}

/// Frontend-facing pipeline event.
#[derive(Debug, Serialize, Clone)]
pub struct PipelineEventView {
    pub kind: String,
    pub payload: String,
    pub created_at: String,
}

/// Convert a core Pipeline to a PipelineSummary for the list view.
fn pipeline_to_summary(p: &Pipeline) -> PipelineSummary {
    let current_stage = p
        .stages
        .iter()
        .find(|s| s.status != StageStatus::Completed && s.status != StageStatus::Skipped)
        .map(|s| {
            serde_json::to_string(&s.name)
                .unwrap_or_default()
                .trim_matches('"')
                .to_string()
        });

    let status_str = serde_json::to_string(&p.status)
        .unwrap_or_default()
        .trim_matches('"')
        .to_string();

    PipelineSummary {
        id: p.id.clone(),
        demand_name: p.demand_name.clone().unwrap_or_default(),
        demand_code: p.demand_code.clone(),
        status: status_str,
        current_stage,
        created_at: p.created_at.to_rfc3339(),
        updated_at: p.updated_at.to_rfc3339(),
    }
}

/// Convert a core Stage to a StageDetail for the detail view.
fn stage_to_detail(s: &Stage) -> StageDetail {
    let name = serde_json::to_string(&s.name)
        .unwrap_or_default()
        .trim_matches('"')
        .to_string();

    let status = serde_json::to_string(&s.status)
        .unwrap_or_default()
        .trim_matches('"')
        .to_string();

    let output_summary = s.output.as_ref().map(|o| {
        let text = o.to_string();
        if text.len() > 500 {
            format!("{}...", &text[..500])
        } else {
            text
        }
    });

    let gate_results = s.gate_results.as_ref().map(|g| g.to_string());

    let issue = s
        .issue
        .as_ref()
        .map(|i| serde_json::to_string(i).unwrap_or_default());

    StageDetail {
        name,
        status,
        retry_count: s.retry_count,
        output_summary,
        gate_results,
        issue,
        started_at: s.started_at.map(|d| d.to_rfc3339()),
        completed_at: s.completed_at.map(|d| d.to_rfc3339()),
    }
}

/// Convert a core Pipeline to a PipelineDetail for the detail view.
fn pipeline_to_detail(p: &Pipeline) -> PipelineDetail {
    let status_str = serde_json::to_string(&p.status)
        .unwrap_or_default()
        .trim_matches('"')
        .to_string();

    PipelineDetail {
        id: p.id.clone(),
        demand_id: p.demand_id,
        demand_code: p.demand_code.clone(),
        demand_name: p.demand_name.clone().unwrap_or_default(),
        status: status_str,
        raw_link: p.raw_link.clone(),
        operator: p.operator.clone(),
        has_regressed: p.has_regressed,
        stages: p.stages.iter().map(stage_to_detail).collect(),
        created_at: p.created_at.to_rfc3339(),
        updated_at: p.updated_at.to_rfc3339(),
    }
}

#[tauri::command]
pub async fn list_pipelines(
    state: State<'_, AppState>,
) -> Result<Vec<PipelineSummary>, String> {
    let pipelines = state.store.list_all()?;
    Ok(pipelines.iter().map(pipeline_to_summary).collect())
}

#[tauri::command]
pub async fn get_pipeline(
    id: String,
    state: State<'_, AppState>,
) -> Result<PipelineDetail, String> {
    let pipeline = state
        .store
        .load(&id)?
        .ok_or_else(|| format!("Pipeline not found: {}", id))?;
    Ok(pipeline_to_detail(&pipeline))
}

#[tauri::command]
pub async fn get_pipeline_events(
    id: String,
    _limit: Option<i64>,
    state: State<'_, AppState>,
) -> Result<Vec<PipelineEventView>, String> {
    let events = state.event_store.query_by_pipeline(&id)?;
    Ok(events
        .iter()
        .map(|e| {
            let payload = serde_json::to_string(e).unwrap_or_default();
            let kind = serde_json::to_value(e)
                .ok()
                .and_then(|v| v.get("kind").and_then(|k| k.as_str()).map(String::from))
                .unwrap_or_else(|| "unknown".into());
            let created_at = serde_json::to_value(e)
                .ok()
                .and_then(|v| {
                    v.get("timestamp")
                        .and_then(|t| t.as_str())
                        .map(String::from)
                })
                .unwrap_or_else(|| Utc::now().to_rfc3339());

            PipelineEventView {
                kind,
                payload,
                created_at,
            }
        })
        .collect())
}

/// Submit a new pipeline from a xingyun demand link.
/// Validates the link, creates a Pipeline struct via poria-core types,
/// stores via SqlitePipelineStore, and emits a Tauri event.
#[tauri::command]
pub async fn submit_pipeline(
    link: String,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    // Parse and validate the xingyun demand URL
    let parsed = parse_xingyun_demand_url(&link)?;

    let pipeline_id = create_pipeline_id();
    let now = Utc::now();

    // Build initial pipeline with all stages in pending
    let pipeline = Pipeline {
        id: pipeline_id.clone(),
        demand_id: parsed.demand_id,
        demand_code: parsed.demand_code.clone().unwrap_or_default(),
        demand_name: None,
        status: PipelineStatus::Created,
        raw_link: parsed.url.clone(),
        operator: String::new(),
        has_regressed: false,
        config: PipelineConfig {
            gates: vec![],
            trd_scope: vec![],
            repos: vec![],
        },
        stages: STAGE_ORDER
            .iter()
            .map(|stage_enum| Stage {
                id: None,
                pipeline_id: pipeline_id.clone(),
                name: *stage_enum,
                status: StageStatus::Pending,
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
            })
            .collect(),
        repos: vec![],
        created_at: now,
        updated_at: now,
    };

    // Persist to database
    state.store.create(&pipeline)?;

    // Emit Tauri event so the UI can react
    app.emit("pipeline:created", &pipeline_id)
        .map_err(|e| e.to_string())?;

    Ok(pipeline_id)
}

/// Cancel a running pipeline.
/// Updates the pipeline status directly in the store and emits an event.
#[tauri::command]
pub async fn cancel_pipeline(
    id: String,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut pipeline = state
        .store
        .load(&id)?
        .ok_or_else(|| format!("Pipeline not found: {}", id))?;

    pipeline.status = PipelineStatus::Cancelled;
    pipeline.updated_at = Utc::now();

    // Cancel any running stages
    for stage in &mut pipeline.stages {
        if stage.status == StageStatus::Running || stage.status == StageStatus::Pending {
            stage.status = StageStatus::Skipped;
        }
    }

    let cancel_event = CorePipelineEvent::pipeline_cancelled(&id, "user");

    state
        .store
        .save_stage_tx(None, &pipeline, &[cancel_event])?;

    app.emit("pipeline:cancelled", &id)
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Respond to a human-loop request.
/// Emits a Tauri event for the execution loop to pick up.
/// `action` is one of: "resume", "skip", "cancel"
#[tauri::command]
pub async fn human_loop_respond(
    pipeline_id: String,
    action: String,
    app: tauri::AppHandle,
) -> Result<(), String> {
    // Validate action
    match action.as_str() {
        "resume" | "skip" | "cancel" => {}
        _ => {
            return Err(format!(
                "Invalid action '{}': must be resume, skip, or cancel",
                action
            ))
        }
    }

    #[derive(Serialize)]
    struct HumanResponse {
        pipeline_id: String,
        action: String,
    }

    let payload = HumanResponse {
        pipeline_id,
        action,
    };

    app.emit("human-loop:response", &payload)
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Request execution of the next pending stage in a pipeline.
#[tauri::command]
pub async fn execute_stage(
    pipeline_id: String,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let pipeline = state
        .store
        .load(&pipeline_id)?
        .ok_or_else(|| format!("Pipeline not found: {}", pipeline_id))?;

    let current = pipeline
        .stages
        .iter()
        .find(|s| s.status == StageStatus::Pending)
        .ok_or("No pending stages")?;

    let stage_name = serde_json::to_string(&current.name)
        .unwrap_or_default()
        .trim_matches('"')
        .to_string();

    app.emit(
        "stage:execute-requested",
        serde_json::json!({
            "pipeline_id": pipeline_id,
            "stage": stage_name,
        }),
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Skip a pending stage in a pipeline.
#[tauri::command]
pub async fn skip_stage(
    pipeline_id: String,
    stage_name: String,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut pipeline = state
        .store
        .load(&pipeline_id)?
        .ok_or_else(|| format!("Pipeline not found: {}", pipeline_id))?;

    if let Some(stage) = pipeline.stages.iter_mut().find(|s| {
        let name = serde_json::to_string(&s.name)
            .unwrap_or_default()
            .trim_matches('"')
            .to_string();
        name == stage_name && s.status == StageStatus::Pending
    }) {
        stage.status = StageStatus::Skipped;
        pipeline.updated_at = Utc::now();
        state
            .store
            .save_stage_tx(None, &pipeline, &[])
            .map_err(|e| e.to_string())?;

        app.emit(
            "stage:skipped",
            serde_json::json!({
                "pipeline_id": pipeline_id,
                "stage": stage_name,
            }),
        )
        .map_err(|e| e.to_string())?;
    } else {
        return Err(format!(
            "Stage '{}' not found or not pending",
            stage_name
        ));
    }

    Ok(())
}
