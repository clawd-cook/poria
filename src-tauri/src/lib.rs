use std::sync::{Arc, Mutex};
use tauri::Manager;

mod auto_run;
mod commands;

pub use auto_run::AutoRunScheduler;

pub struct AppState {
    pub agent_pool: Arc<poria_resources::ClaudeAgentPool>,
    pub auto_run: Arc<Mutex<AutoRunScheduler>>,
    pub event_store: Arc<poria_infrastructure::store::EventStore>,
    pub repo_store: Arc<poria_infrastructure::store::RegisteredRepoStore>,
    pub session_tracker: Arc<poria_resources::SessionTracker>,
    pub store: Arc<poria_infrastructure::store::SqlitePipelineStore>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let db_path = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                .join("poria.db");

            // In development mode, prefer workspace/db/poria.db if it exists
            let db_path = if std::path::Path::new("workspace/db/poria.db").exists() {
                std::path::PathBuf::from("workspace/db/poria.db")
            } else {
                db_path
            };

            // Open database in RW mode via poria-infrastructure
            let conn = poria_infrastructure::store::init_database(&db_path)
                .expect("Failed to open database");

            // Open a second connection for the event store (each store owns its connection)
            let event_conn = poria_infrastructure::store::init_database(&db_path)
                .expect("Failed to open event database connection");

            let store = Arc::new(poria_infrastructure::store::SqlitePipelineStore::new(conn));
            let event_store = Arc::new(poria_infrastructure::store::EventStore::new(event_conn));
            let repo_conn = poria_infrastructure::store::init_database(&db_path)
                .expect("Failed to open repo database connection");
            let repo_store = Arc::new(poria_infrastructure::store::RegisteredRepoStore::new(
                repo_conn,
            ));
            if let Err(e) = repo_store.fail_interrupted_clones() {
                tracing::warn!(error = %e, "failed to mark interrupted clones as failed");
            }
            let agent_pool = Arc::new(poria_resources::ClaudeAgentPool::new_with_path_provider(
                3,
                || {
                    poria_infrastructure::config::load_config(None)
                        .effective_claude_path()
                        .map(str::to_string)
                },
            ));
            let session_tracker = Arc::new(poria_resources::SessionTracker::new());

            app.manage(AppState {
                agent_pool,
                auto_run: Arc::new(Mutex::new(AutoRunScheduler::default())),
                event_store,
                repo_store,
                session_tracker,
                store: store.clone(),
            });

            let poll_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                    let Some(state) = poll_handle.try_state::<AppState>() else {
                        continue;
                    };
                    let _ =
                        commands::pipeline::poll_waiting_merges_once(&poll_handle, &state.store)
                            .await;
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::pipeline::list_pipelines,
            commands::pipeline::get_pipeline,
            commands::pipeline::submit_pipeline,
            commands::pipeline::cancel_pipeline,
            commands::pipeline::human_loop_respond,
            commands::pipeline::confirm_trd,
            commands::pipeline::confirm_merge_ready,
            commands::pipeline::get_pipeline_events,
            commands::pipeline::execute_stage,
            commands::pipeline::skip_stage,
            commands::pipeline::open_workspace,
            commands::auth::get_auth_status,
            commands::auth::start_login,
            commands::auth::logout,
            commands::demands::list_demands,
            commands::demands::preview_demand_prd,
            commands::demands::resolve_demand_link,
            commands::projects::list_demand_project,
            commands::projects::read_demand_project_file,
            commands::projects::write_demand_project_file,
            commands::config::get_config,
            commands::config::update_config,
            commands::config::probe_claude,
            commands::skills::list_skills,
            commands::skills::get_skill,
            commands::channels::list_channels,
            commands::repos::register_repo,
            commands::repos::list_repos,
            commands::repos::retry_clone,
            commands::repos::list_repo_branches,
            commands::repos::sync_repo,
            commands::repos::update_repo_default_branch,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
