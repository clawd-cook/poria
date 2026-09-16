use poria_infrastructure::config::{load_config, PoriaConfig};
use serde::{Deserialize, Serialize};

/// Frontend-facing config view that matches the full PoriaConfig structure.
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

/// Convert a full PoriaConfig to the frontend AppConfig view.
fn poria_config_to_app_config(c: &PoriaConfig) -> AppConfig {
    AppConfig {
        cr_score_threshold: c.gates.cr_score_threshold.clone(),
        test_coverage_threshold: c.gates.test_coverage_threshold,
        max_diff_lines: c.gates.diff_size_threshold,
        agent_timeout_ms: c.timeouts.agent,
        max_retries: c.retry.max_stage_retries,
        db_path: c.paths.db_path.clone(),
    }
}

/// Read configuration using poria-infrastructure, falling back to defaults.
#[tauri::command]
pub async fn get_config() -> Result<AppConfig, String> {
    let config = load_config(None);
    Ok(poria_config_to_app_config(&config))
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

    // Build a full PoriaConfig from the frontend values + defaults for the rest
    let base = load_config(None);
    let full = PoriaConfig {
        gates: poria_infrastructure::config::GateConfig {
            cr_score_threshold: config.cr_score_threshold,
            test_coverage_threshold: config.test_coverage_threshold,
            diff_size_threshold: config.max_diff_lines,
        },
        timeouts: poria_infrastructure::config::TimeoutConfig {
            agent: config.agent_timeout_ms,
            ..base.timeouts
        },
        retry: poria_infrastructure::config::RetryConfig {
            max_stage_retries: config.max_retries,
            ..base.retry
        },
        paths: poria_infrastructure::config::PathConfig {
            db_path: config.db_path,
            ..base.paths
        },
    };

    let json = serde_json::to_string_pretty(&full).map_err(|e| e.to_string())?;
    std::fs::write(&config_path, json).map_err(|e| e.to_string())?;

    Ok(())
}
