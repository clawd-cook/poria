use std::sync::Arc;
use tauri::Manager;

mod commands;

pub struct AppState {
    pub store: Arc<poria_infrastructure::store::SqlitePipelineStore>,
    pub event_store: Arc<poria_infrastructure::store::EventStore>,
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

            app.manage(AppState {
                store,
                event_store,
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::pipeline::list_pipelines,
            commands::pipeline::get_pipeline,
            commands::pipeline::submit_pipeline,
            commands::pipeline::cancel_pipeline,
            commands::pipeline::human_loop_respond,
            commands::pipeline::get_pipeline_events,
            commands::auth::get_auth_status,
            commands::config::get_config,
            commands::config::update_config,
            commands::skills::list_skills,
            commands::channels::list_channels,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
