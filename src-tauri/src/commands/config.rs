use poria_infrastructure::config::{
    get_config_file_path, load_config, normalize_claude_path, normalize_optional_string,
    PoriaConfig,
};
use poria_resources::ClaudeProbeResult;
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
    /// Optional absolute `claude` CLI path. Empty/null means auto `which`.
    #[serde(default)]
    pub claude_path: Option<String>,
    /// Newline-separated Dev verify commands. Empty/null → package.json convention.
    #[serde(default)]
    pub dev_verify_commands: Option<String>,
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
        claude_path: c.effective_claude_path().map(str::to_string),
        dev_verify_commands: c.effective_dev_verify_commands().map(str::to_string),
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
    let config_path = get_config_file_path(None);

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
        claude_path: normalize_claude_path(config.claude_path.as_deref()),
        dev_verify_commands: normalize_optional_string(config.dev_verify_commands.as_deref()),
    };

    let json = serde_json::to_string_pretty(&full).map_err(|e| e.to_string())?;
    std::fs::write(&config_path, json).map_err(|e| e.to_string())?;

    Ok(())
}

/// Probe the Claude CLI using the same resolve path as Agent spawn.
///
/// `path_override` is the settings field (may be unsaved). Empty/null → auto `which`.
#[tauri::command]
pub async fn probe_claude(path_override: Option<String>) -> Result<ClaudeProbeResult, String> {
    Ok(poria_resources::probe_claude_cli(path_override.as_deref()).await)
}
