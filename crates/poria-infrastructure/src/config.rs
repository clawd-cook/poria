use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub use crate::auth::get_config_file_path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoriaConfig {
    pub gates: GateConfig,
    pub timeouts: TimeoutConfig,
    pub retry: RetryConfig,
    pub paths: PathConfig,
    /// Optional absolute path to the `claude` CLI. Empty / whitespace is unset.
    #[serde(default)]
    pub claude_path: Option<String>,
    /// Newline-separated Dev local-verify commands. Empty / unset → package.json convention.
    #[serde(default)]
    pub dev_verify_commands: Option<String>,
    /// Max in-flight pipelines in the desktop worker. Default 2.
    #[serde(default = "default_max_parallel_pipelines")]
    pub max_parallel_pipelines: i32,
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
            claude_path: None,
            dev_verify_commands: None,
            max_parallel_pipelines: 2,
        }
    }
}

impl PoriaConfig {
    /// Trimmed override path, or `None` when unset / blank.
    pub fn effective_claude_path(&self) -> Option<&str> {
        self.claude_path
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
    }

    /// Trimmed verify override, or `None` when unset / blank (auto-detect).
    pub fn effective_dev_verify_commands(&self) -> Option<&str> {
        self.dev_verify_commands
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
    }

    /// Clamp Settings parallelism into 1..=8.
    pub fn effective_max_parallel_pipelines(&self) -> usize {
        clamp_max_parallel_pipelines(self.max_parallel_pipelines)
    }
}

fn default_max_parallel_pipelines() -> i32 {
    2
}

pub fn clamp_max_parallel_pipelines(value: i32) -> usize {
    value.clamp(1, 8) as usize
}

pub fn load_config(overrides: Option<PoriaConfig>) -> PoriaConfig {
    load_config_from(
        get_config_file_path(None),
        PathBuf::from("poria.config.json"),
        overrides,
    )
}

/// Load config preferring `user_config`, then CWD fallback when the user file is missing.
pub fn load_config_from(
    user_config: PathBuf,
    cwd_fallback: PathBuf,
    overrides: Option<PoriaConfig>,
) -> PoriaConfig {
    let mut config = PoriaConfig::default();

    if user_config.is_file() {
        if let Ok(content) = std::fs::read_to_string(&user_config) {
            if let Ok(file_config) = serde_json::from_str::<PoriaConfig>(&content) {
                config = file_config;
            }
        }
    } else if let Ok(content) = std::fs::read_to_string(&cwd_fallback) {
        if let Ok(file_config) = serde_json::from_str::<PoriaConfig>(&content) {
            config = file_config;
        }
    }

    apply_env_overrides(&mut config);

    if let Some(overrides) = overrides {
        config = overrides;
    }

    config
}

fn apply_env_overrides(config: &mut PoriaConfig) {
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
}

pub fn normalize_claude_path(value: Option<&str>) -> Option<String> {
    normalize_optional_string(value)
}

pub fn normalize_optional_string(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::path::{Path, PathBuf};

    #[test]
    fn test_default_config() {
        let config = PoriaConfig::default();
        assert_eq!(config.gates.cr_score_threshold, "B+");
        assert_eq!(config.gates.test_coverage_threshold, 80.0);
        assert_eq!(config.retry.max_stage_retries, 3);
        assert_eq!(config.claude_path, None);
        assert_eq!(config.dev_verify_commands, None);
        assert_eq!(config.effective_max_parallel_pipelines(), 2);
    }

    #[test]
    fn test_load_config_defaults() {
        let missing_user = tempfile::tempdir().unwrap().path().join("config.json");
        let missing_cwd = tempfile::tempdir()
            .unwrap()
            .path()
            .join("poria.config.json");
        let config = load_config_from(missing_user, missing_cwd, None);
        assert_eq!(config.gates.cr_score_threshold, "B+");
        assert_eq!(config.claude_path, None);
        assert_eq!(config.dev_verify_commands, None);
    }

    #[test]
    fn test_config_claude_path_defaults_when_field_missing() {
        let mut value = serde_json::to_value(PoriaConfig::default()).unwrap();
        value.as_object_mut().unwrap().remove("claude_path");
        let parsed: PoriaConfig = serde_json::from_value(value).unwrap();
        assert_eq!(parsed.claude_path, None);
        assert_eq!(parsed.effective_claude_path(), None);
    }

    #[test]
    fn test_config_empty_claude_path_is_unset() {
        let config = PoriaConfig {
            claude_path: Some("  ".into()),
            ..PoriaConfig::default()
        };
        assert_eq!(config.effective_claude_path(), None);
        assert_eq!(normalize_claude_path(Some("")), None);
        assert_eq!(
            normalize_claude_path(Some(" /bin/claude ")).as_deref(),
            Some("/bin/claude")
        );

        let json = serde_json::to_string(&config).unwrap();
        let parsed: PoriaConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.claude_path.as_deref(), Some("  "));
        assert_eq!(parsed.effective_claude_path(), None);
    }

    #[test]
    fn test_load_config_prefers_user_file_over_cwd() {
        let user_dir = tempfile::tempdir().unwrap();
        let cwd_dir = tempfile::tempdir().unwrap();
        let user_file = user_dir.path().join("config.json");
        let cwd_file = cwd_dir.path().join("poria.config.json");

        let user_config = PoriaConfig {
            claude_path: Some("/opt/homebrew/bin/claude".into()),
            gates: GateConfig {
                cr_score_threshold: "A".into(),
                ..PoriaConfig::default().gates
            },
            ..PoriaConfig::default()
        };
        std::fs::write(&user_file, serde_json::to_string(&user_config).unwrap()).unwrap();

        let cwd_config = PoriaConfig {
            claude_path: Some("/tmp/cwd-claude".into()),
            gates: GateConfig {
                cr_score_threshold: "C".into(),
                ..PoriaConfig::default().gates
            },
            ..PoriaConfig::default()
        };
        std::fs::write(&cwd_file, serde_json::to_string(&cwd_config).unwrap()).unwrap();

        let loaded = load_config_from(user_file, cwd_file, None);
        assert_eq!(
            loaded.effective_claude_path(),
            Some("/opt/homebrew/bin/claude")
        );
        assert_eq!(loaded.gates.cr_score_threshold, "A");
    }

    #[test]
    fn test_load_config_cwd_fallback_when_user_file_missing() {
        let user_file = tempfile::tempdir().unwrap().path().join("config.json");
        let cwd_dir = tempfile::tempdir().unwrap();
        let cwd_file = cwd_dir.path().join("poria.config.json");

        let cwd_config = PoriaConfig {
            claude_path: Some("/usr/local/bin/claude".into()),
            ..PoriaConfig::default()
        };
        std::fs::write(&cwd_file, serde_json::to_string(&cwd_config).unwrap()).unwrap();

        let loaded = load_config_from(user_file, cwd_file, None);
        assert_eq!(
            loaded.effective_claude_path(),
            Some("/usr/local/bin/claude")
        );
    }

    #[test]
    fn test_config_dev_verify_commands_default_and_blank() {
        let mut value = serde_json::to_value(PoriaConfig::default()).unwrap();
        value.as_object_mut().unwrap().remove("dev_verify_commands");
        let parsed: PoriaConfig = serde_json::from_value(value).unwrap();
        assert_eq!(parsed.effective_dev_verify_commands(), None);

        let blank = PoriaConfig {
            dev_verify_commands: Some("  \n".into()),
            ..PoriaConfig::default()
        };
        assert_eq!(blank.effective_dev_verify_commands(), None);
        assert_eq!(
            PoriaConfig {
                dev_verify_commands: Some("pnpm typecheck\npnpm test --run".into()),
                ..PoriaConfig::default()
            }
            .effective_dev_verify_commands(),
            Some("pnpm typecheck\npnpm test --run")
        );
        assert_eq!(clamp_max_parallel_pipelines(0), 1);
        assert_eq!(clamp_max_parallel_pipelines(99), 8);
    }

    #[test]
    fn test_config_file_path_sits_next_to_auth() {
        let root = Path::new("/tmp/.poria");
        assert_eq!(
            get_config_file_path(Some(root)),
            PathBuf::from("/tmp/.poria/config.json")
        );
    }

    #[test]
    fn test_missing_claude_path_field_is_null_in_round_trip_value() {
        let parsed: PoriaConfig = serde_json::from_value(Value::Object(
            serde_json::to_value(PoriaConfig::default())
                .unwrap()
                .as_object()
                .unwrap()
                .clone(),
        ))
        .unwrap();
        assert_eq!(parsed.claude_path, None);
    }
}
