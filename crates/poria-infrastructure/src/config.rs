use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoriaConfig {
    pub gates: GateConfig,
    pub timeouts: TimeoutConfig,
    pub retry: RetryConfig,
    pub paths: PathConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateConfig {
    pub cr_score_threshold: String,
    pub test_coverage_threshold: f64,
    pub diff_size_threshold: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfig {
    pub git_short: i64,
    pub git_medium: i64,
    pub git_long: i64,
    pub build: i64,
    pub agent: i64,
    pub agent_idle: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max_stage_retries: i32,
    pub queue_poll_interval_ms: i64,
    pub mr_poll_interval_ms: i64,
    pub mr_timeout_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathConfig {
    pub db_path: String,
    pub archive_dir: String,
    pub backup_dir: String,
    pub log_dir: String,
}

impl Default for PoriaConfig {
    fn default() -> Self {
        Self {
            gates: GateConfig {
                cr_score_threshold: "B+".into(),
                test_coverage_threshold: 80.0,
                diff_size_threshold: 500,
            },
            timeouts: TimeoutConfig {
                git_short: 30_000,
                git_medium: 120_000,
                git_long: 300_000,
                build: 600_000,
                agent: 1_800_000,
                agent_idle: 300_000,
            },
            retry: RetryConfig {
                max_stage_retries: 3,
                queue_poll_interval_ms: 5_000,
                mr_poll_interval_ms: 60_000,
                mr_timeout_ms: 86_400_000,
            },
            paths: PathConfig {
                db_path: "workspace/db/poria.db".into(),
                archive_dir: "workspace/archive".into(),
                backup_dir: "workspace/db/backup".into(),
                log_dir: "workspace/logs".into(),
            },
        }
    }
}

pub fn load_config(overrides: Option<PoriaConfig>) -> PoriaConfig {
    let mut config = PoriaConfig::default();

    if let Ok(content) = std::fs::read_to_string("poria.config.json") {
        if let Ok(file_config) = serde_json::from_str::<PoriaConfig>(&content) {
            config = file_config;
        }
    }

    if let Ok(val) = std::env::var("PORIA_CR_SCORE_THRESHOLD") {
        config.gates.cr_score_threshold = val;
    }
    if let Ok(val) = std::env::var("PORIA_TEST_COVERAGE_THRESHOLD") {
        if let Ok(v) = val.parse() {
            config.gates.test_coverage_threshold = v;
        }
    }
    if let Ok(val) = std::env::var("PORIA_DIFF_SIZE_THRESHOLD") {
        if let Ok(v) = val.parse() {
            config.gates.diff_size_threshold = v;
        }
    }

    if let Some(overrides) = overrides {
        config = overrides;
    }

    config
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = PoriaConfig::default();
        assert_eq!(config.gates.cr_score_threshold, "B+");
        assert_eq!(config.gates.test_coverage_threshold, 80.0);
        assert_eq!(config.retry.max_stage_retries, 3);
    }

    #[test]
    fn test_load_config_defaults() {
        let config = load_config(None);
        assert_eq!(config.gates.cr_score_threshold, "B+");
    }
}
