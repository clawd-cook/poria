use tauri::Manager;

mod commands;
mod db;

pub struct AppState {
    pub db: std::sync::Mutex<db::Database>,
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

            let database = db::Database::open(&db_path).expect("Failed to open database");

            app.manage(AppState {
                db: std::sync::Mutex::new(database),
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
