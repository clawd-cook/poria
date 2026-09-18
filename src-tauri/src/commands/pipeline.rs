use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_opener::OpenerExt;

use poria_channels::coding::{normalize_git_url, repo_search_path_from_git_url};
use poria_channels::xingyun::{feature_branch_name, is_joyspace_prd_link, xingyun_demand_view_url};
use poria_core::contracts::{Skill, SkillContext};
use poria_core::pipeline::{create_pipeline_id, PipelineEvent as CorePipelineEvent};
use poria_core::types::{
    BackendContext, Pipeline, PipelineConfig, PipelineStatus, RepoConfig, SkillInput, Stage,
    StageEnum, StageIssue, StageStatus, STAGE_ORDER,
};
use poria_infrastructure::auth::{
    assert_path_under_projects_root, assert_path_under_repos_root,
    assert_path_under_workspaces_root, demand_project_folder_name, get_credentials,
    get_demand_project_dir, get_pipeline_workspace_dir, get_workspaces_root,
};
use poria_infrastructure::store::{CloneStatus, RegisteredRepo, SqlitePipelineStore};
use poria_resources::WorktreeResource;
use poria_skills::{
    prepare_pipeline_workspace, CodeReviewSkill, DeploySkill, GenCodeSkill, GenTrdSkill, InitSkill,
    ReviewPrdSkill,
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
    pub workspace_path: Option<String>,
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
        workspace_path: init_output_path(p, "workspacePath"),
        created_at: p.created_at.to_rfc3339(),
        updated_at: p.updated_at.to_rfc3339(),
    }
}

#[tauri::command]
pub async fn list_pipelines(state: State<'_, AppState>) -> Result<Vec<PipelineSummary>, String> {
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

/// Submit a pipeline from the demand start wizard.
/// Frontend repo is the only `repos` entry; backend is read-only `backend_context`.
/// `backend_trd_url` is a required JoySpace URL stored on config (not frontend `TRD.md`).
#[tauri::command]
pub async fn submit_pipeline(
    demand_id: i64,
    frontend_repo_id: String,
    backend_repo_id: String,
    backend_branch: String,
    prd_url: String,
    backend_trd_url: String,
    demand_code: Option<String>,
    demand_name: Option<String>,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    if demand_id <= 0 {
        return Err("需求无效".into());
    }
    let frontend_repo_id = frontend_repo_id.trim().to_string();
    let backend_repo_id = backend_repo_id.trim().to_string();
    let backend_branch = backend_branch.trim().to_string();
    let prd_url = prd_url.trim().to_string();
    let backend_trd_url = backend_trd_url.trim().to_string();
    if frontend_repo_id.is_empty() {
        return Err("请选择前端仓库".into());
    }
    if backend_repo_id.is_empty() {
        return Err("请选择后端仓库".into());
    }
    if frontend_repo_id == backend_repo_id {
        return Err("前端仓库与后端仓库不能相同".into());
    }
    if backend_branch.is_empty() {
        return Err("请选择后端分支".into());
    }
    require_prd_and_backend_trd_urls(&prd_url, &backend_trd_url)?;

    let frontend = require_ready_repo(&state, &frontend_repo_id, "前端仓库")?;
    let backend = require_ready_repo(&state, &backend_repo_id, "后端仓库")?;

    let base_branch = frontend.default_branch.trim().to_string();
    if base_branch.is_empty() {
        return Err("前端仓库未设置主分支".into());
    }

    let demand_code = demand_code
        .as_deref()
        .map(str::trim)
        .filter(|code| !code.is_empty())
        .unwrap_or("")
        .to_string();
    let demand_name = demand_name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(|name| name.to_string());
    let feature_branch = feature_branch_name(
        if demand_code.is_empty() {
            None
        } else {
            Some(demand_code.as_str())
        },
        demand_id,
    );

    let gitlab_project_path = repo_search_path_from_git_url(&frontend.git_url)
        .unwrap_or_else(|| format!("{}/{}", frontend.scope, frontend.name));

    let frontend_repo = RepoConfig {
        name: frontend.name.clone(),
        git_url: frontend.git_url.clone(),
        branch: feature_branch,
        base_branch,
        gitlab_project_path,
        depends_on: None,
        build_cmd: None,
    };

    let backend_context = BackendContext {
        git_url: backend.git_url.clone(),
        local_path: backend.local_path.clone(),
        branch: backend_branch,
        scope: backend.scope.clone(),
        name: backend.name.clone(),
    };
    let raw_link = xingyun_demand_view_url(
        demand_id,
        if demand_code.is_empty() {
            None
        } else {
            Some(demand_code.as_str())
        },
    );
    let operator = get_credentials(None)
        .map(|creds| creds.username.trim().to_string())
        .filter(|username| !username.is_empty())
        .ok_or_else(|| "请先登录".to_string())?;

    let pipeline_id = create_pipeline_id();
    let now = Utc::now();
    let config = PipelineConfig {
        gates: vec![],
        trd_scope: vec![],
        repos: vec![frontend_repo.clone()],
        prd_url: Some(prd_url),
        backend_trd_url: Some(backend_trd_url),
        backend_context: Some(backend_context),
        project_dir: None,
    };

    let pipeline = Pipeline {
        id: pipeline_id.clone(),
        demand_id,
        demand_code,
        demand_name,
        status: PipelineStatus::Created,
        raw_link,
        operator,
        has_regressed: false,
        config: config.clone(),
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
        repos: config.repos.clone(),
        created_at: now,
        updated_at: now,
    };

    if pipeline.repos.len() != 1 {
        return Err("流水线只能包含前端仓库".into());
    }

    state.store.create(&pipeline)?;

    app.emit("pipeline:created", &pipeline_id)
        .map_err(|e| e.to_string())?;
    app.emit("pipeline:list-changed", &pipeline_id)
        .map_err(|e| e.to_string())?;

    enqueue_auto_run(app.clone(), &state, pipeline_id.clone());

    Ok(pipeline_id)
}

fn require_prd_and_backend_trd_urls(prd_url: &str, backend_trd_url: &str) -> Result<(), String> {
    if prd_url.is_empty() {
        return Err("请填写 JoySpace PRD 链接".into());
    }
    if !is_joyspace_prd_link(prd_url) {
        return Err("PRD 必须是 JoySpace 链接".into());
    }
    if backend_trd_url.is_empty() {
        return Err("请填写 JoySpace 后端 TRD 链接".into());
    }
    if !is_joyspace_prd_link(backend_trd_url) {
        return Err("后端 TRD 必须是 JoySpace 链接".into());
    }
    if backend_trd_url == prd_url {
        return Err("后端 TRD 不能与 PRD 使用相同链接".into());
    }
    Ok(())
}

fn require_ready_repo(state: &AppState, id: &str, label: &str) -> Result<RegisteredRepo, String> {
    let repo = state
        .repo_store
        .load(id)?
        .ok_or_else(|| format!("{label}不存在"))?;
    if repo.clone_status != CloneStatus::Ready {
        return Err(format!("{label}尚未克隆完成"));
    }
    assert_path_under_repos_root(None, PathBuf::from(&repo.local_path).as_path())?;
    Ok(repo)
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

    if let Ok(mut scheduler) = state.auto_run.lock() {
        scheduler.drop_queued(&id);
    }

    app.emit("pipeline:cancelled", &id)
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
    state: State<'_, AppState>,
) -> Result<(), String> {
    match action.as_str() {
        "resume" => {
            enqueue_auto_run(app, &state, pipeline_id);
            Ok(())
        }
        "skip" => {
            skip_actionable_stage(&pipeline_id, &app, &state)?;
            enqueue_auto_run(app, &state, pipeline_id);
            Ok(())
        }
        "cancel" => cancel_pipeline(pipeline_id, app, state).await,
        _ => Err(format!(
            "Invalid action '{}': must be resume, skip, or cancel",
            action
        )),
    }
}

/// Request execution of the next pending stage in a pipeline.
#[tauri::command]
pub async fn execute_stage(
    pipeline_id: String,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if let Some(current) = state
        .auto_run
        .lock()
        .map_err(|e| e.to_string())?
        .current()
        .map(str::to_string)
    {
        if current == pipeline_id {
            return Err("流水线正在自动执行".into());
        }
        return Err("请等待当前流水线自动执行完成".into());
    }
    let advanced =
        execute_next_stage(&pipeline_id, &app, &AutoRunRuntime::from_state(&state)).await?;
    if !advanced {
        return Err("没有待执行的阶段".into());
    }
    Ok(())
}

fn emit_pipeline_updated(app: &tauri::AppHandle, pipeline: &Pipeline) -> Result<(), String> {
    let current_stage = pipeline
        .stages
        .iter()
        .find(|s| s.status != StageStatus::Completed && s.status != StageStatus::Skipped)
        .map(|s| {
            serde_json::to_string(&s.name)
                .unwrap_or_default()
                .trim_matches('"')
                .to_string()
        })
        .unwrap_or_default();
    let status = serde_json::to_string(&pipeline.status)
        .unwrap_or_default()
        .trim_matches('"')
        .to_string();
    app.emit(
        "pipeline:updated",
        serde_json::json!({
            "id": pipeline.id,
            "status": status,
            "currentStage": current_stage,
        }),
    )
    .map_err(|e| e.to_string())?;
    app.emit("pipeline:list-changed", &pipeline.id)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(Clone)]
struct AutoRunRuntime {
    agent_pool: Arc<poria_resources::ClaudeAgentPool>,
    auto_run: Arc<std::sync::Mutex<crate::AutoRunScheduler>>,
    repo_store: Arc<poria_infrastructure::store::RegisteredRepoStore>,
    store: Arc<SqlitePipelineStore>,
}

impl AutoRunRuntime {
    fn from_state(state: &AppState) -> Self {
        Self {
            agent_pool: state.agent_pool.clone(),
            auto_run: state.auto_run.clone(),
            repo_store: state.repo_store.clone(),
            store: state.store.clone(),
        }
    }
}

fn enqueue_auto_run(app: AppHandle, state: &AppState, pipeline_id: String) {
    let start_id = match state.auto_run.lock() {
        Ok(mut scheduler) => scheduler.submit(pipeline_id),
        Err(error) => {
            tracing::error!(error = %error, "auto-run scheduler poisoned");
            None
        }
    };
    if let Some(id) = start_id {
        spawn_auto_run(app, AutoRunRuntime::from_state(state), id);
    }
}

fn spawn_auto_run(app: AppHandle, runtime: AutoRunRuntime, pipeline_id: String) {
    tauri::async_runtime::spawn(async move {
        run_auto_loop(app.clone(), runtime.clone(), pipeline_id.clone()).await;
        let next = runtime
            .auto_run
            .lock()
            .ok()
            .and_then(|mut scheduler| scheduler.finish(&pipeline_id));
        if let Some(next_id) = next {
            spawn_auto_run(app, runtime, next_id);
        }
    });
}

async fn run_auto_loop(app: AppHandle, runtime: AutoRunRuntime, pipeline_id: String) {
    loop {
        match execute_next_stage(&pipeline_id, &app, &runtime).await {
            Ok(true) => continue,
            Ok(false) => break,
            Err(message) => {
                let _ = emit_human_request(&app, &runtime.store, &pipeline_id, &message);
                break;
            }
        }
    }
}

async fn execute_next_stage(
    pipeline_id: &str,
    app: &AppHandle,
    runtime: &AutoRunRuntime,
) -> Result<bool, String> {
    let store = runtime.store.clone();
    let mut pipeline = store
        .load(pipeline_id)?
        .ok_or_else(|| format!("Pipeline not found: {pipeline_id}"))?;

    match pipeline.status {
        PipelineStatus::Cancelled | PipelineStatus::Completed | PipelineStatus::WaitingMerge => {
            return Ok(false);
        }
        _ => {}
    }

    let Some(stage_idx) = pipeline
        .stages
        .iter()
        .position(|s| s.status != StageStatus::Completed && s.status != StageStatus::Skipped)
    else {
        return Ok(false);
    };

    if pipeline.stages[stage_idx].status == StageStatus::Running {
        return Err("阶段正在执行".into());
    }

    let stage_name = pipeline.stages[stage_idx].name;
    match stage_name {
        StageEnum::Init => {
            run_init_stage(&mut pipeline, stage_idx, app, &store, &runtime.repo_store).await?;
        }
        StageEnum::ReviewPrd => {
            run_review_prd_stage(
                &mut pipeline,
                stage_idx,
                app,
                &store,
                runtime.agent_pool.clone(),
            )
            .await?;
        }
        StageEnum::Design => {
            run_design_stage(
                &mut pipeline,
                stage_idx,
                app,
                &store,
                runtime.agent_pool.clone(),
            )
            .await?;
        }
        StageEnum::Dev => {
            run_dev_stage(
                &mut pipeline,
                stage_idx,
                app,
                &store,
                runtime.agent_pool.clone(),
            )
            .await?;
        }
        StageEnum::Cr => {
            run_cr_stage(
                &mut pipeline,
                stage_idx,
                app,
                &store,
                runtime.agent_pool.clone(),
            )
            .await?;
        }
        StageEnum::Deploy => run_deploy_stage(&mut pipeline, stage_idx, app, &store).await?,
    }
    Ok(true)
}

fn emit_human_request(
    app: &AppHandle,
    store: &SqlitePipelineStore,
    pipeline_id: &str,
    detail: &str,
) -> Result<(), String> {
    let pipeline = store
        .load(pipeline_id)?
        .ok_or_else(|| format!("Pipeline not found: {pipeline_id}"))?;
    let stage = pipeline
        .stages
        .iter()
        .find(|s| s.status == StageStatus::Failed || s.status == StageStatus::Blocked)
        .or_else(|| {
            pipeline
                .stages
                .iter()
                .find(|s| s.status != StageStatus::Completed && s.status != StageStatus::Skipped)
        });
    let stage_name = stage
        .map(|s| {
            serde_json::to_string(&s.name)
                .unwrap_or_default()
                .trim_matches('"')
                .to_string()
        })
        .unwrap_or_default();
    let issue_class = stage
        .and_then(|s| s.issue.as_ref())
        .map(|issue| issue.class.clone())
        .unwrap_or_else(|| "stage_failed".into());
    app.emit(
        "human:request",
        serde_json::json!({
            "pipelineId": pipeline_id,
            "stage": stage_name,
            "issueClass": issue_class,
            "detail": detail,
        }),
    )
    .map_err(|e| e.to_string())
}

fn is_skippable_status(status: StageStatus) -> bool {
    matches!(
        status,
        StageStatus::Pending | StageStatus::Failed | StageStatus::Blocked
    )
}

fn skip_actionable_stage(
    pipeline_id: &str,
    app: &AppHandle,
    state: &AppState,
) -> Result<(), String> {
    let mut pipeline = state
        .store
        .load(pipeline_id)?
        .ok_or_else(|| format!("Pipeline not found: {pipeline_id}"))?;
    let Some(stage) = pipeline
        .stages
        .iter_mut()
        .find(|s| is_skippable_status(s.status))
    else {
        return Err("没有可跳过的阶段".into());
    };
    let stage_name = serde_json::to_string(&stage.name)
        .unwrap_or_default()
        .trim_matches('"')
        .to_string();
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
    emit_pipeline_updated(app, &pipeline)
}

async fn run_init_stage(
    pipeline: &mut Pipeline,
    stage_idx: usize,
    app: &tauri::AppHandle,
    store: &std::sync::Arc<SqlitePipelineStore>,
    repo_store: &std::sync::Arc<poria_infrastructure::store::RegisteredRepoStore>,
) -> Result<(), String> {
    let creds = get_credentials(None).ok_or_else(|| "请先登录".to_string())?;
    if creds.cookie.trim().is_empty() {
        return Err("请先登录".into());
    }

    let frontend = pipeline
        .config
        .repos
        .first()
        .cloned()
        .ok_or("缺少前端仓库，无法创建工作区")?;
    let backend = pipeline
        .config
        .backend_context
        .clone()
        .ok_or("缺少后端仓库上下文，无法创建工作区")?;
    let frontend_registered =
        resolve_registered_clone(repo_store, &frontend.git_url, &frontend.name)?;
    let backend_registered = resolve_registered_clone(repo_store, &backend.git_url, &backend.name)?;

    let folder = demand_project_folder_name(&pipeline.demand_code, pipeline.demand_id);
    let project_dir = get_demand_project_dir(None, &folder)?;

    pipeline.status = PipelineStatus::Running;
    pipeline.stages[stage_idx].status = StageStatus::Running;
    pipeline.stages[stage_idx].skill_id = Some("skill:init".into());
    pipeline.stages[stage_idx].started_at = Some(Utc::now());
    pipeline.stages[stage_idx].issue = None;
    pipeline.updated_at = Utc::now();
    store
        .save_stage_tx(
            Some(&pipeline.stages[stage_idx]),
            pipeline,
            &[CorePipelineEvent::stage_started(
                &pipeline.id,
                StageEnum::Init,
            )],
        )
        .map_err(|e| e.to_string())?;
    emit_pipeline_updated(app, pipeline)?;

    if let Err(message) = sync_repo_for_init(app, repo_store, &frontend_registered.id).await {
        return fail_init_stage(pipeline, stage_idx, app, store, message).await;
    }
    if let Err(message) = sync_repo_for_init(app, repo_store, &backend_registered.id).await {
        return fail_init_stage(pipeline, stage_idx, app, store, message).await;
    }

    let ctx = SkillContext {
        pipeline_id: pipeline.id.clone(),
        workdir: project_dir.to_string_lossy().to_string(),
        credentials: serde_json::json!({
            "cookie": creds.cookie,
            "username": creds.username,
        }),
    };
    let input = SkillInput {
        stage: pipeline.stages[stage_idx].clone(),
        pipeline: pipeline.clone(),
        extra: serde_json::Map::new(),
    };

    let mut output = match InitSkill::new().execute(input, ctx).await {
        Ok(output) => output.output,
        Err(err) => {
            return fail_init_stage(pipeline, stage_idx, app, store, err.to_string()).await;
        }
    };

    let workspace_dir = match get_pipeline_workspace_dir(None, &pipeline.id) {
        Ok(path) => path,
        Err(err) => {
            return fail_init_stage(pipeline, stage_idx, app, store, err).await;
        }
    };
    let bundled_skills = match crate::commands::skills::bundled_skills_dir(app) {
        Ok(path) => path,
        Err(err) => {
            return fail_init_stage(pipeline, stage_idx, app, store, err).await;
        }
    };
    if let Err(err) = prepare_pipeline_workspace(&workspace_dir, &project_dir, &bundled_skills) {
        let _ = cleanup_pipeline_workspace(&workspace_dir);
        return fail_init_stage(pipeline, stage_idx, app, store, err).await;
    }

    let worktree_root = get_workspaces_root(None);
    let worktree = WorktreeResource::new(Some(worktree_root.to_string_lossy().into_owned()));
    let frontend_git_root = PathBuf::from(&frontend_registered.local_path);
    let backend_git_root = PathBuf::from(&backend_registered.local_path);
    if let Err(err) = assert_path_under_repos_root(None, &frontend_git_root) {
        let _ = cleanup_pipeline_workspace(&workspace_dir);
        return fail_init_stage(pipeline, stage_idx, app, store, err).await;
    }
    if let Err(err) = assert_path_under_repos_root(None, &backend_git_root) {
        let _ = cleanup_pipeline_workspace(&workspace_dir);
        return fail_init_stage(pipeline, stage_idx, app, store, err).await;
    }

    let frontend_created = match worktree
        .create(
            &frontend,
            &pipeline.id,
            &frontend.base_branch,
            &frontend_git_root,
        )
        .await
    {
        Ok(created) => created,
        Err(err) => {
            let _ = worktree
                .remove_path(&frontend_git_root, &worktree.path(&frontend, &pipeline.id))
                .await;
            let _ = cleanup_pipeline_workspace(&workspace_dir);
            return fail_init_stage(pipeline, stage_idx, app, store, err.to_string()).await;
        }
    };

    let backend_created = match worktree
        .create_detached(
            &backend.name,
            &pipeline.id,
            &backend_git_root,
            &backend.branch,
        )
        .await
    {
        Ok(created) => created,
        Err(err) => {
            let _ = worktree
                .remove_path(
                    &frontend_git_root,
                    Path::new(&frontend_created.worktree_path),
                )
                .await;
            let _ = worktree
                .remove_path(&backend_git_root, &workspace_dir.join(&backend.name))
                .await;
            let _ = cleanup_pipeline_workspace(&workspace_dir);
            return fail_init_stage(pipeline, stage_idx, app, store, err.to_string()).await;
        }
    };

    if let Some(backend_ctx) = pipeline.config.backend_context.as_mut() {
        backend_ctx.local_path = backend_created.worktree_path.clone();
    }

    let workspace_path = workspace_dir.to_string_lossy().into_owned();
    output = merge_init_workspace_output(
        output,
        &frontend,
        &frontend_created.worktree_path,
        &frontend_created.branch,
        &backend_created.worktree_path,
        &workspace_path,
    );
    let rollback = init_rollback_instruction(
        stage_idx as i32,
        &frontend_created.worktree_path,
        &backend_created.worktree_path,
        &frontend_created.branch,
        &workspace_path,
    );

    pipeline.config.project_dir = Some(project_dir.to_string_lossy().to_string());
    pipeline.stages[stage_idx].status = StageStatus::Completed;
    pipeline.stages[stage_idx].output = Some(output.clone());
    pipeline.stages[stage_idx].rollback = Some(rollback);
    pipeline.stages[stage_idx].completed_at = Some(Utc::now());
    pipeline.updated_at = Utc::now();
    store
        .save_stage_tx(
            Some(&pipeline.stages[stage_idx]),
            pipeline,
            &[CorePipelineEvent::stage_completed(
                &pipeline.id,
                StageEnum::Init,
                output,
            )],
        )
        .map_err(|e| e.to_string())?;
    emit_pipeline_updated(app, pipeline)?;
    Ok(())
}

async fn sync_repo_for_init(
    app: &tauri::AppHandle,
    repo_store: &std::sync::Arc<poria_infrastructure::store::RegisteredRepoStore>,
    id: &str,
) -> Result<(), String> {
    match super::repos::run_repo_sync(repo_store, id).await {
        Ok(updated) => {
            let _ = app.emit("repo:updated", &updated);
            Ok(())
        }
        Err(err) => {
            if let Ok(Some(updated)) = repo_store.load(id) {
                let _ = app.emit("repo:updated", &updated);
            }
            Err(format!("同步仓库失败: {err}"))
        }
    }
}

async fn fail_init_stage(
    pipeline: &mut Pipeline,
    stage_idx: usize,
    app: &tauri::AppHandle,
    store: &std::sync::Arc<SqlitePipelineStore>,
    message: String,
) -> Result<(), String> {
    pipeline.status = PipelineStatus::Failed;
    pipeline.stages[stage_idx].status = StageStatus::Failed;
    pipeline.stages[stage_idx].completed_at = Some(Utc::now());
    pipeline.stages[stage_idx].issue = Some(StageIssue {
        class: "init_failed".into(),
        message: message.clone(),
        retryable: true,
    });
    pipeline.updated_at = Utc::now();
    store
        .save_stage_tx(
            Some(&pipeline.stages[stage_idx]),
            pipeline,
            &[CorePipelineEvent::stage_failed(
                &pipeline.id,
                StageEnum::Init,
                &message,
                pipeline.stages[stage_idx].retry_count,
            )],
        )
        .map_err(|e| e.to_string())?;
    emit_pipeline_updated(app, pipeline)?;
    Err(message)
}

fn merge_init_workspace_output(
    mut output: serde_json::Value,
    frontend: &RepoConfig,
    frontend_worktree: &str,
    frontend_branch: &str,
    backend_worktree: &str,
    workspace_path: &str,
) -> serde_json::Value {
    if let Some(obj) = output.as_object_mut() {
        obj.insert(
            "workspacePath".into(),
            serde_json::Value::String(workspace_path.to_string()),
        );
        obj.insert(
            "worktreePath".into(),
            serde_json::Value::String(frontend_worktree.to_string()),
        );
        obj.insert(
            "backendWorktreePath".into(),
            serde_json::Value::String(backend_worktree.to_string()),
        );
        obj.insert(
            "repos".into(),
            serde_json::json!([{
                "name": frontend.name,
                "branch": frontend_branch,
                "baseBranch": frontend.base_branch,
                "changeId": serde_json::Value::Null,
                "worktreePath": frontend_worktree,
                "gitlabProjectPath": frontend.gitlab_project_path,
            }]),
        );
    }
    output
}

fn init_rollback_instruction(
    stage_index: i32,
    frontend_worktree: &str,
    backend_worktree: &str,
    feature_branch: &str,
    workspace_path: &str,
) -> poria_core::types::RollbackInstruction {
    use poria_core::types::{RollbackCommand, RollbackCommandType, RollbackInstruction};
    RollbackInstruction {
        stage_index,
        commands: vec![
            RollbackCommand {
                command_type: RollbackCommandType::RemoveWorktree,
                params: [("path".into(), frontend_worktree.into())]
                    .into_iter()
                    .collect(),
            },
            RollbackCommand {
                command_type: RollbackCommandType::RemoveWorktree,
                params: [("path".into(), backend_worktree.into())]
                    .into_iter()
                    .collect(),
            },
            RollbackCommand {
                command_type: RollbackCommandType::RemoveDirectory,
                params: [("path".into(), workspace_path.into())]
                    .into_iter()
                    .collect(),
            },
            RollbackCommand {
                command_type: RollbackCommandType::DeleteBranch,
                params: [("branch".into(), feature_branch.into())]
                    .into_iter()
                    .collect(),
            },
        ],
    }
}

fn resolve_demand_project_dir(pipeline: &Pipeline) -> Result<PathBuf, String> {
    if let Some(dir) = pipeline
        .config
        .project_dir
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let path = PathBuf::from(dir);
        assert_path_under_projects_root(None, &path)?;
        return Ok(path);
    }
    let folder = demand_project_folder_name(&pipeline.demand_code, pipeline.demand_id);
    get_demand_project_dir(None, &folder)
}

async fn run_review_prd_stage(
    pipeline: &mut Pipeline,
    stage_idx: usize,
    app: &tauri::AppHandle,
    store: &std::sync::Arc<SqlitePipelineStore>,
    agent_pool: std::sync::Arc<poria_resources::ClaudeAgentPool>,
) -> Result<(), String> {
    let project_dir = resolve_demand_project_dir(pipeline)?;
    if !project_dir.join("PRD.md").is_file() {
        return Err("请先完成初始化，导出 PRD.md".into());
    }
    let workspace = resolve_pipeline_workspace(pipeline)?;
    let worktree = resolve_frontend_worktree(pipeline)?;
    if !worktree.join(".git").exists() {
        return Err(format!("前端工作区无效: {}", worktree.display()));
    }
    let workspace_str = workspace.to_string_lossy().into_owned();
    let worktree_str = worktree.to_string_lossy().into_owned();

    pipeline.status = PipelineStatus::Running;
    pipeline.stages[stage_idx].status = StageStatus::Running;
    pipeline.stages[stage_idx].skill_id = Some("skill:review-prd".into());
    pipeline.stages[stage_idx].started_at = Some(Utc::now());
    pipeline.stages[stage_idx].issue = None;
    pipeline.updated_at = Utc::now();
    store
        .save_stage_tx(
            Some(&pipeline.stages[stage_idx]),
            pipeline,
            &[CorePipelineEvent::stage_started(
                &pipeline.id,
                StageEnum::ReviewPrd,
            )],
        )
        .map_err(|e| e.to_string())?;
    emit_pipeline_updated(app, pipeline)?;

    let project_dir_str = project_dir.to_string_lossy().to_string();
    let mut extra = serde_json::Map::new();
    extra.insert(
        "feature_dir".into(),
        serde_json::Value::String(project_dir_str.clone()),
    );
    extra.insert(
        "workspace_path".into(),
        serde_json::Value::String(workspace_str.clone()),
    );
    extra.insert(
        "worktree_path".into(),
        serde_json::Value::String(worktree_str),
    );
    extra.insert(
        "reviewed_at".into(),
        serde_json::Value::String(Utc::now().to_rfc3339()),
    );

    let ctx = SkillContext {
        pipeline_id: pipeline.id.clone(),
        workdir: workspace_str,
        credentials: serde_json::json!({}),
    };
    let input = SkillInput {
        stage: pipeline.stages[stage_idx].clone(),
        pipeline: pipeline.clone(),
        extra,
    };

    match ReviewPrdSkill::new()
        .with_agent_pool(agent_pool)
        .execute(input, ctx)
        .await
    {
        Ok(output) => {
            pipeline.stages[stage_idx].status = StageStatus::Completed;
            pipeline.stages[stage_idx].output = Some(output.output.clone());
            pipeline.stages[stage_idx].completed_at = Some(Utc::now());
            pipeline.updated_at = Utc::now();
            store
                .save_stage_tx(
                    Some(&pipeline.stages[stage_idx]),
                    pipeline,
                    &[CorePipelineEvent::stage_completed(
                        &pipeline.id,
                        StageEnum::ReviewPrd,
                        output.output,
                    )],
                )
                .map_err(|e| e.to_string())?;
            emit_pipeline_updated(app, pipeline)?;
            Ok(())
        }
        Err(err) => {
            let message = err.to_string();
            pipeline.status = PipelineStatus::Failed;
            pipeline.stages[stage_idx].status = StageStatus::Failed;
            pipeline.stages[stage_idx].completed_at = Some(Utc::now());
            pipeline.stages[stage_idx].issue = Some(StageIssue {
                class: "review_prd_failed".into(),
                message: message.clone(),
                retryable: true,
            });
            pipeline.updated_at = Utc::now();
            store
                .save_stage_tx(
                    Some(&pipeline.stages[stage_idx]),
                    pipeline,
                    &[CorePipelineEvent::stage_failed(
                        &pipeline.id,
                        StageEnum::ReviewPrd,
                        &message,
                        pipeline.stages[stage_idx].retry_count,
                    )],
                )
                .map_err(|e| e.to_string())?;
            emit_pipeline_updated(app, pipeline)?;
            Err(message)
        }
    }
}

async fn run_design_stage(
    pipeline: &mut Pipeline,
    stage_idx: usize,
    app: &tauri::AppHandle,
    store: &std::sync::Arc<SqlitePipelineStore>,
    agent_pool: std::sync::Arc<poria_resources::ClaudeAgentPool>,
) -> Result<(), String> {
    let project_dir = resolve_demand_project_dir(pipeline)?;
    if !project_dir.join("PRD.md").is_file() {
        return Err("请先完成初始化，导出 PRD.md".into());
    }
    let workspace = resolve_pipeline_workspace(pipeline)?;
    let worktree = resolve_frontend_worktree(pipeline)?;
    if !worktree.join(".git").exists() {
        return Err(format!("前端工作区无效: {}", worktree.display()));
    }
    let workspace_str = workspace.to_string_lossy().into_owned();
    let worktree_str = worktree.to_string_lossy().into_owned();

    pipeline.status = PipelineStatus::Running;
    pipeline.stages[stage_idx].status = StageStatus::Running;
    pipeline.stages[stage_idx].skill_id = Some("skill:gen-trd".into());
    pipeline.stages[stage_idx].started_at = Some(Utc::now());
    pipeline.stages[stage_idx].issue = None;
    pipeline.updated_at = Utc::now();
    store
        .save_stage_tx(
            Some(&pipeline.stages[stage_idx]),
            pipeline,
            &[CorePipelineEvent::stage_started(
                &pipeline.id,
                StageEnum::Design,
            )],
        )
        .map_err(|e| e.to_string())?;
    emit_pipeline_updated(app, pipeline)?;

    let project_dir_str = project_dir.to_string_lossy().to_string();
    let mut extra = serde_json::Map::new();
    extra.insert(
        "feature_dir".into(),
        serde_json::Value::String(project_dir_str.clone()),
    );
    extra.insert(
        "workspace_path".into(),
        serde_json::Value::String(workspace_str.clone()),
    );
    extra.insert(
        "worktree_path".into(),
        serde_json::Value::String(worktree_str),
    );

    let ctx = SkillContext {
        pipeline_id: pipeline.id.clone(),
        workdir: workspace_str,
        credentials: serde_json::json!({}),
    };
    let input = SkillInput {
        stage: pipeline.stages[stage_idx].clone(),
        pipeline: pipeline.clone(),
        extra,
    };

    match GenTrdSkill::new()
        .with_agent_pool(agent_pool)
        .execute(input, ctx)
        .await
    {
        Ok(output) => {
            pipeline.stages[stage_idx].status = StageStatus::Completed;
            pipeline.stages[stage_idx].output = Some(output.output.clone());
            pipeline.stages[stage_idx].completed_at = Some(Utc::now());
            pipeline.updated_at = Utc::now();
            store
                .save_stage_tx(
                    Some(&pipeline.stages[stage_idx]),
                    pipeline,
                    &[CorePipelineEvent::stage_completed(
                        &pipeline.id,
                        StageEnum::Design,
                        output.output,
                    )],
                )
                .map_err(|e| e.to_string())?;
            emit_pipeline_updated(app, pipeline)?;
            Ok(())
        }
        Err(err) => {
            let message = err.to_string();
            pipeline.status = PipelineStatus::Failed;
            pipeline.stages[stage_idx].status = StageStatus::Failed;
            pipeline.stages[stage_idx].completed_at = Some(Utc::now());
            pipeline.stages[stage_idx].issue = Some(StageIssue {
                class: "design_failed".into(),
                message: message.clone(),
                retryable: true,
            });
            pipeline.updated_at = Utc::now();
            store
                .save_stage_tx(
                    Some(&pipeline.stages[stage_idx]),
                    pipeline,
                    &[CorePipelineEvent::stage_failed(
                        &pipeline.id,
                        StageEnum::Design,
                        &message,
                        pipeline.stages[stage_idx].retry_count,
                    )],
                )
                .map_err(|e| e.to_string())?;
            emit_pipeline_updated(app, pipeline)?;
            Err(message)
        }
    }
}

fn assert_path_under_workspaces_dir(path: &Path) -> Result<(), String> {
    assert_path_under_workspaces_root(None, path)
}

fn init_output_path(pipeline: &Pipeline, key: &str) -> Option<String> {
    pipeline
        .stages
        .iter()
        .find(|stage| stage.name == StageEnum::Init)
        .and_then(|stage| stage.output.as_ref())
        .and_then(|output| output.get(key))
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn resolve_pipeline_workspace(pipeline: &Pipeline) -> Result<PathBuf, String> {
    let init = pipeline
        .stages
        .iter()
        .find(|stage| stage.name == StageEnum::Init)
        .ok_or("请先完成初始化")?;
    if init.status != StageStatus::Completed {
        return Err("请先完成初始化".into());
    }
    if let Some(path) = init_output_path(pipeline, "workspacePath") {
        let workspace = PathBuf::from(path);
        assert_path_under_workspaces_dir(&workspace)?;
        if workspace.is_dir() {
            return Ok(workspace);
        }
    }
    let workspace = get_pipeline_workspace_dir(None, &pipeline.id)?;
    assert_path_under_workspaces_dir(&workspace)?;
    if !workspace.is_dir() {
        return Err(format!("工作区不存在: {}", workspace.display()));
    }
    Ok(workspace)
}

fn cleanup_pipeline_workspace(workspace_dir: &Path) -> Result<(), String> {
    assert_path_under_workspaces_dir(workspace_dir)?;
    if workspace_dir.exists() {
        std::fs::remove_dir_all(workspace_dir).map_err(|e| format!("无法删除工作区: {e}"))?;
    }
    Ok(())
}

fn resolve_registered_clone(
    repo_store: &poria_infrastructure::store::RegisteredRepoStore,
    git_url: &str,
    name: &str,
) -> Result<RegisteredRepo, String> {
    let wanted = normalize_git_url(git_url);
    repo_store
        .list_all()?
        .into_iter()
        .find(|registered| {
            registered.clone_status == CloneStatus::Ready
                && (normalize_git_url(&registered.git_url) == wanted || registered.name == name)
        })
        .ok_or_else(|| format!("未找到已就绪的仓库 {name}"))
}

fn resolve_frontend_worktree(pipeline: &Pipeline) -> Result<PathBuf, String> {
    if let Some(init) = pipeline
        .stages
        .iter()
        .find(|stage| stage.name == StageEnum::Init)
    {
        if init.status != StageStatus::Completed {
            return Err("请先完成初始化".into());
        }
        if let Some(path) = init
            .output
            .as_ref()
            .and_then(|output| {
                output
                    .get("worktreePath")
                    .and_then(|value| value.as_str())
                    .or_else(|| {
                        output
                            .get("repos")
                            .and_then(|repos| repos.as_array())
                            .and_then(|repos| repos.first())
                            .and_then(|repo| repo.get("worktreePath"))
                            .and_then(|value| value.as_str())
                    })
            })
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            let worktree = PathBuf::from(path);
            assert_path_under_workspaces_dir(&worktree)?;
            if worktree.exists() {
                return Ok(worktree);
            }
        }
    } else {
        return Err("请先完成初始化".into());
    }

    let repo = pipeline
        .config
        .repos
        .first()
        .ok_or("缺少前端仓库，无法进入开发")?;
    let worktree = get_workspaces_root(None)
        .join(&pipeline.id)
        .join(&repo.name);
    assert_path_under_workspaces_dir(&worktree)?;
    if !worktree.exists() {
        return Err(format!("前端工作区不存在: {}", worktree.display()));
    }
    Ok(worktree)
}

async fn run_dev_stage(
    pipeline: &mut Pipeline,
    stage_idx: usize,
    app: &tauri::AppHandle,
    store: &std::sync::Arc<SqlitePipelineStore>,
    agent_pool: std::sync::Arc<poria_resources::ClaudeAgentPool>,
) -> Result<(), String> {
    let project_dir = resolve_demand_project_dir(pipeline)?;
    if !project_dir.join("TRD.md").is_file() {
        return Err("请先完成技术设计，生成 TRD.md".into());
    }
    let workspace = resolve_pipeline_workspace(pipeline)?;
    let worktree = resolve_frontend_worktree(pipeline)?;
    if !worktree.join(".git").exists() {
        return Err(format!("前端工作区无效: {}", worktree.display()));
    }

    pipeline.status = PipelineStatus::Running;
    pipeline.stages[stage_idx].status = StageStatus::Running;
    pipeline.stages[stage_idx].skill_id = Some("skill:gen-code".into());
    pipeline.stages[stage_idx].started_at = Some(Utc::now());
    pipeline.stages[stage_idx].issue = None;
    pipeline.updated_at = Utc::now();
    store
        .save_stage_tx(
            Some(&pipeline.stages[stage_idx]),
            pipeline,
            &[CorePipelineEvent::stage_started(
                &pipeline.id,
                StageEnum::Dev,
            )],
        )
        .map_err(|e| e.to_string())?;
    emit_pipeline_updated(app, pipeline)?;

    let project_dir_str = project_dir.to_string_lossy().into_owned();
    let workspace_str = workspace.to_string_lossy().into_owned();
    let worktree_str = worktree.to_string_lossy().into_owned();
    let mut extra = serde_json::Map::new();
    extra.insert(
        "feature_dir".into(),
        serde_json::Value::String(project_dir_str),
    );
    extra.insert(
        "workspace_path".into(),
        serde_json::Value::String(workspace_str.clone()),
    );
    extra.insert(
        "worktree_path".into(),
        serde_json::Value::String(worktree_str),
    );

    let ctx = SkillContext {
        pipeline_id: pipeline.id.clone(),
        workdir: workspace_str,
        credentials: serde_json::json!({}),
    };
    let input = SkillInput {
        stage: pipeline.stages[stage_idx].clone(),
        pipeline: pipeline.clone(),
        extra,
    };

    match GenCodeSkill::new()
        .with_agent_pool(agent_pool)
        .execute(input, ctx)
        .await
    {
        Ok(output) => {
            pipeline.stages[stage_idx].status = StageStatus::Completed;
            pipeline.stages[stage_idx].output = Some(output.output.clone());
            pipeline.stages[stage_idx].completed_at = Some(Utc::now());
            pipeline.updated_at = Utc::now();
            store
                .save_stage_tx(
                    Some(&pipeline.stages[stage_idx]),
                    pipeline,
                    &[CorePipelineEvent::stage_completed(
                        &pipeline.id,
                        StageEnum::Dev,
                        output.output,
                    )],
                )
                .map_err(|e| e.to_string())?;
            emit_pipeline_updated(app, pipeline)?;
            Ok(())
        }
        Err(err) => {
            let message = err.to_string();
            pipeline.status = PipelineStatus::Failed;
            pipeline.stages[stage_idx].status = StageStatus::Failed;
            pipeline.stages[stage_idx].completed_at = Some(Utc::now());
            pipeline.stages[stage_idx].issue = Some(StageIssue {
                class: "dev_failed".into(),
                message: message.clone(),
                retryable: true,
            });
            pipeline.updated_at = Utc::now();
            store
                .save_stage_tx(
                    Some(&pipeline.stages[stage_idx]),
                    pipeline,
                    &[CorePipelineEvent::stage_failed(
                        &pipeline.id,
                        StageEnum::Dev,
                        &message,
                        pipeline.stages[stage_idx].retry_count,
                    )],
                )
                .map_err(|e| e.to_string())?;
            emit_pipeline_updated(app, pipeline)?;
            Err(message)
        }
    }
}

async fn run_cr_stage(
    pipeline: &mut Pipeline,
    stage_idx: usize,
    app: &tauri::AppHandle,
    store: &std::sync::Arc<SqlitePipelineStore>,
    agent_pool: std::sync::Arc<poria_resources::ClaudeAgentPool>,
) -> Result<(), String> {
    let project_dir = resolve_demand_project_dir(pipeline)?;
    let workspace = resolve_pipeline_workspace(pipeline)?;
    let worktree = resolve_frontend_worktree(pipeline)?;
    if !worktree.join(".git").exists() {
        return Err(format!("前端工作区无效: {}", worktree.display()));
    }

    pipeline.status = PipelineStatus::Running;
    pipeline.stages[stage_idx].status = StageStatus::Running;
    pipeline.stages[stage_idx].skill_id = Some("skill:code-review".into());
    pipeline.stages[stage_idx].started_at = Some(Utc::now());
    pipeline.stages[stage_idx].issue = None;
    pipeline.updated_at = Utc::now();
    store
        .save_stage_tx(
            Some(&pipeline.stages[stage_idx]),
            pipeline,
            &[CorePipelineEvent::stage_started(
                &pipeline.id,
                StageEnum::Cr,
            )],
        )
        .map_err(|e| e.to_string())?;
    emit_pipeline_updated(app, pipeline)?;

    let project_dir_str = project_dir.to_string_lossy().into_owned();
    let workspace_str = workspace.to_string_lossy().into_owned();
    let worktree_str = worktree.to_string_lossy().into_owned();
    let repo = pipeline.config.repos.first();
    let mut extra = serde_json::Map::new();
    extra.insert(
        "feature_dir".into(),
        serde_json::Value::String(project_dir_str),
    );
    extra.insert(
        "workspace_path".into(),
        serde_json::Value::String(workspace_str.clone()),
    );
    extra.insert(
        "worktree_path".into(),
        serde_json::Value::String(worktree_str),
    );
    if let Some(repo) = repo {
        extra.insert(
            "base_branch".into(),
            serde_json::Value::String(repo.base_branch.clone()),
        );
        extra.insert(
            "target_branch".into(),
            serde_json::Value::String(repo.branch.clone()),
        );
    }

    let ctx = SkillContext {
        pipeline_id: pipeline.id.clone(),
        workdir: workspace_str,
        credentials: serde_json::json!({}),
    };
    let input = SkillInput {
        stage: pipeline.stages[stage_idx].clone(),
        pipeline: pipeline.clone(),
        extra,
    };

    match CodeReviewSkill::new()
        .with_agent_pool(agent_pool)
        .execute(input, ctx)
        .await
    {
        Ok(output) => {
            pipeline.stages[stage_idx].status = StageStatus::Completed;
            pipeline.stages[stage_idx].output = Some(output.output.clone());
            pipeline.stages[stage_idx].completed_at = Some(Utc::now());
            pipeline.updated_at = Utc::now();
            store
                .save_stage_tx(
                    Some(&pipeline.stages[stage_idx]),
                    pipeline,
                    &[CorePipelineEvent::stage_completed(
                        &pipeline.id,
                        StageEnum::Cr,
                        output.output,
                    )],
                )
                .map_err(|e| e.to_string())?;
            emit_pipeline_updated(app, pipeline)?;
            Ok(())
        }
        Err(err) => {
            let message = err.to_string();
            pipeline.status = PipelineStatus::Failed;
            pipeline.stages[stage_idx].status = StageStatus::Failed;
            pipeline.stages[stage_idx].completed_at = Some(Utc::now());
            pipeline.stages[stage_idx].issue = Some(StageIssue {
                class: "cr_failed".into(),
                message: message.clone(),
                retryable: true,
            });
            pipeline.updated_at = Utc::now();
            store
                .save_stage_tx(
                    Some(&pipeline.stages[stage_idx]),
                    pipeline,
                    &[CorePipelineEvent::stage_failed(
                        &pipeline.id,
                        StageEnum::Cr,
                        &message,
                        pipeline.stages[stage_idx].retry_count,
                    )],
                )
                .map_err(|e| e.to_string())?;
            emit_pipeline_updated(app, pipeline)?;
            Err(message)
        }
    }
}

fn workspace_repo_field<'a>(pipeline: &'a Pipeline, key: &str) -> Option<&'a str> {
    pipeline
        .stages
        .iter()
        .find(|stage| stage.name == StageEnum::Init)
        .and_then(|stage| stage.output.as_ref())
        .and_then(|output| output.get("repos"))
        .and_then(|repos| repos.as_array())
        .and_then(|repos| repos.first())
        .and_then(|repo| repo.get(key))
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

async fn run_deploy_stage(
    pipeline: &mut Pipeline,
    stage_idx: usize,
    app: &tauri::AppHandle,
    store: &std::sync::Arc<SqlitePipelineStore>,
) -> Result<(), String> {
    let worktree = resolve_frontend_worktree(pipeline)?;
    if !worktree.join(".git").exists() {
        return Err(format!("前端工作区无效: {}", worktree.display()));
    }
    let repo = pipeline
        .config
        .repos
        .first()
        .cloned()
        .ok_or("缺少前端仓库，无法部署")?;
    let creds = get_credentials(None).ok_or_else(|| "请先登录".to_string())?;

    pipeline.status = PipelineStatus::Running;
    pipeline.stages[stage_idx].status = StageStatus::Running;
    pipeline.stages[stage_idx].skill_id = Some("skill:deploy".into());
    pipeline.stages[stage_idx].started_at = Some(Utc::now());
    pipeline.stages[stage_idx].issue = None;
    pipeline.updated_at = Utc::now();
    store
        .save_stage_tx(
            Some(&pipeline.stages[stage_idx]),
            pipeline,
            &[CorePipelineEvent::stage_started(
                &pipeline.id,
                StageEnum::Deploy,
            )],
        )
        .map_err(|e| e.to_string())?;
    emit_pipeline_updated(app, pipeline)?;

    let worktree_str = worktree.to_string_lossy().into_owned();
    let branch = workspace_repo_field(pipeline, "branch")
        .unwrap_or(repo.branch.as_str())
        .to_string();
    let base_branch = workspace_repo_field(pipeline, "baseBranch")
        .unwrap_or(repo.base_branch.as_str())
        .to_string();
    let gitlab_project_path = workspace_repo_field(pipeline, "gitlabProjectPath")
        .unwrap_or(repo.gitlab_project_path.as_str())
        .to_string();

    let mut extra = serde_json::Map::new();
    extra.insert(
        "worktree_path".into(),
        serde_json::Value::String(worktree_str.clone()),
    );
    extra.insert("branch".into(), serde_json::Value::String(branch));
    extra.insert("base_branch".into(), serde_json::Value::String(base_branch));
    extra.insert(
        "gitlab_project_path".into(),
        serde_json::Value::String(gitlab_project_path),
    );

    let ctx = SkillContext {
        pipeline_id: pipeline.id.clone(),
        workdir: worktree_str,
        credentials: serde_json::json!({
            "cookie": creds.cookie,
            "username": creds.username,
        }),
    };
    let input = SkillInput {
        stage: pipeline.stages[stage_idx].clone(),
        pipeline: pipeline.clone(),
        extra,
    };

    match DeploySkill::new().execute(input, ctx).await {
        Ok(output) => {
            pipeline.status = PipelineStatus::Completed;
            pipeline.stages[stage_idx].status = StageStatus::Completed;
            pipeline.stages[stage_idx].output = Some(output.output.clone());
            pipeline.stages[stage_idx].completed_at = Some(Utc::now());
            pipeline.updated_at = Utc::now();
            store
                .save_stage_tx(
                    Some(&pipeline.stages[stage_idx]),
                    pipeline,
                    &[CorePipelineEvent::stage_completed(
                        &pipeline.id,
                        StageEnum::Deploy,
                        output.output,
                    )],
                )
                .map_err(|e| e.to_string())?;
            emit_pipeline_updated(app, pipeline)?;
            Ok(())
        }
        Err(err) => {
            let message = err.to_string();
            pipeline.status = PipelineStatus::Failed;
            pipeline.stages[stage_idx].status = StageStatus::Failed;
            pipeline.stages[stage_idx].completed_at = Some(Utc::now());
            pipeline.stages[stage_idx].issue = Some(StageIssue {
                class: "deploy_failed".into(),
                message: message.clone(),
                retryable: true,
            });
            pipeline.updated_at = Utc::now();
            store
                .save_stage_tx(
                    Some(&pipeline.stages[stage_idx]),
                    pipeline,
                    &[CorePipelineEvent::stage_failed(
                        &pipeline.id,
                        StageEnum::Deploy,
                        &message,
                        pipeline.stages[stage_idx].retry_count,
                    )],
                )
                .map_err(|e| e.to_string())?;
            emit_pipeline_updated(app, pipeline)?;
            Err(message)
        }
    }
}

/// Skip a pending, failed, or blocked stage in a pipeline.
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
        name == stage_name && is_skippable_status(s.status)
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
        emit_pipeline_updated(&app, &pipeline)?;
    } else {
        return Err(format!("Stage '{}' not found or not skippable", stage_name));
    }

    Ok(())
}

#[tauri::command]
pub async fn open_workspace(
    pipeline_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let pipeline = state
        .store
        .load(&pipeline_id)?
        .ok_or_else(|| format!("Pipeline not found: {pipeline_id}"))?;
    let path = resolve_pipeline_workspace(&pipeline)?;
    app.opener()
        .open_path(path.to_string_lossy().as_ref(), None::<&str>)
        .map_err(|e| format!("无法打开工作区: {e}"))
}

#[cfg(test)]
mod tests {
    use super::{merge_init_workspace_output, require_prd_and_backend_trd_urls};
    use poria_core::types::RepoConfig;

    #[test]
    fn require_prd_and_backend_trd_rejects_empty_or_non_joyspace() {
        let prd = "https://joyspace.jd.com/pages/prd";
        assert_eq!(
            require_prd_and_backend_trd_urls(prd, "").unwrap_err(),
            "请填写 JoySpace 后端 TRD 链接"
        );
        assert_eq!(
            require_prd_and_backend_trd_urls(prd, "https://example.com/trd").unwrap_err(),
            "后端 TRD 必须是 JoySpace 链接"
        );
    }

    #[test]
    fn require_prd_and_backend_trd_rejects_matching_prd() {
        let url = "https://joyspace.jd.com/pages/same";
        assert_eq!(
            require_prd_and_backend_trd_urls(url, url).unwrap_err(),
            "后端 TRD 不能与 PRD 使用相同链接"
        );
    }

    #[test]
    fn require_prd_and_backend_trd_accepts_distinct_urls() {
        require_prd_and_backend_trd_urls(
            "https://joyspace.jd.com/pages/prd",
            "https://joyspace.jd.com/pages/backend-trd",
        )
        .unwrap();
    }

    #[test]
    fn merge_init_workspace_output_sets_frontend_and_backend_paths() {
        let frontend = RepoConfig {
            name: "ls-entrance".into(),
            git_url: "git@coding.jd.com:ls/ls-entrance.git".into(),
            branch: "feature_R1".into(),
            base_branch: "master".into(),
            gitlab_project_path: "ls/ls-entrance".into(),
            depends_on: None,
            build_cmd: None,
        };
        let merged = merge_init_workspace_output(
            serde_json::json!({
                "projectDir": "/tmp/.poria/projects/R1",
                "prdPath": "/tmp/.poria/projects/R1/PRD.md"
            }),
            &frontend,
            "/tmp/.poria/workspaces/p1/ls-entrance",
            "feature_R1",
            "/tmp/.poria/workspaces/p1/ls-api",
            "/tmp/.poria/workspaces/p1",
        );
        assert_eq!(
            merged.get("workspacePath").and_then(|v| v.as_str()),
            Some("/tmp/.poria/workspaces/p1")
        );
        assert_eq!(
            merged.get("worktreePath").and_then(|v| v.as_str()),
            Some("/tmp/.poria/workspaces/p1/ls-entrance")
        );
        assert_eq!(
            merged.get("backendWorktreePath").and_then(|v| v.as_str()),
            Some("/tmp/.poria/workspaces/p1/ls-api")
        );
        assert_eq!(merged["repos"][0]["baseBranch"].as_str(), Some("master"));
        assert!(merged.get("prdPath").is_some());
    }
}
