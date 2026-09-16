use serde::Serialize;
use tauri::Emitter;
use tauri::State;

use crate::db::{PipelineDetail, PipelineEvent, PipelineSummary};
use crate::AppState;

#[tauri::command]
pub async fn list_pipelines(
    state: State<'_, AppState>,
) -> Result<Vec<PipelineSummary>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.list_pipelines().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_pipeline(
    id: String,
    state: State<'_, AppState>,
) -> Result<PipelineDetail, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_pipeline(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_pipeline_events(
    id: String,
    limit: Option<i64>,
    state: State<'_, AppState>,
) -> Result<Vec<PipelineEvent>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_pipeline_events(&id, limit.unwrap_or(100))
        .map_err(|e| e.to_string())
}

/// Submit a new pipeline from a xingyun demand link.
/// MVP: validates the link format and emits an event for the sidecar to pick up.
#[tauri::command]
pub async fn submit_pipeline(
    link: String,
    app: tauri::AppHandle,
) -> Result<String, String> {
    // Validate link format: must be a xingyun demand URL
    if !link.contains("xingyun.jd.com") {
        return Err("Invalid link: must be a xingyun demand URL".into());
    }

    // Emit event for the Node sidecar to process
    app.emit("sidecar:submit", &link)
        .map_err(|e| e.to_string())?;

    Ok(format!("Pipeline submission queued for: {}", link))
}

/// Cancel a running pipeline.
#[tauri::command]
pub async fn cancel_pipeline(
    id: String,
    app: tauri::AppHandle,
) -> Result<(), String> {
    app.emit("sidecar:cancel", &id)
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Respond to a human-loop request.
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

    app.emit("sidecar:human-respond", &payload)
        .map_err(|e| e.to_string())?;

    Ok(())
}
