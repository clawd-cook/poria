use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    /// CR score threshold, e.g. "B+"
    pub cr_score_threshold: String,
    /// Minimum test coverage percentage, e.g. 80.0
    pub test_coverage_threshold: f64,
    /// Maximum diff lines allowed, e.g. 500
    pub max_diff_lines: i32,
    /// Agent timeout in milliseconds, e.g. 1_800_000 (30 min)
    pub agent_timeout_ms: i64,
    /// Maximum retry count per stage, e.g. 3
    pub max_retries: i32,
    /// Path to the SQLite database directory
    pub db_path: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            cr_score_threshold: "B+".to_string(),
            test_coverage_threshold: 80.0,
            max_diff_lines: 500,
            agent_timeout_ms: 1_800_000,
            max_retries: 3,
            db_path: dirs::home_dir()
                .unwrap_or_default()
                .join(".poria/db")
                .to_string_lossy()
                .to_string(),
        }
    }
}

/// Read configuration from ~/.poria/config.json, falling back to defaults.
#[tauri::command]
pub async fn get_config() -> Result<AppConfig, String> {
    let config_path = dirs::home_dir()
        .unwrap_or_default()
        .join(".poria/config.json");

    if !config_path.exists() {
        return Ok(AppConfig::default());
    }

    let content = std::fs::read_to_string(&config_path).map_err(|e| e.to_string())?;
    let config: AppConfig =
        serde_json::from_str(&content).map_err(|e| e.to_string())?;

    Ok(config)
}

/// Persist configuration to ~/.poria/config.json.
#[tauri::command]
pub async fn update_config(config: AppConfig) -> Result<(), String> {
    let config_path = dirs::home_dir()
        .unwrap_or_default()
        .join(".poria/config.json");

    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let json = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    std::fs::write(&config_path, json).map_err(|e| e.to_string())?;

    Ok(())
}
