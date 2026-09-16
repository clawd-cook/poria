use regex::Regex;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

static CONFIG_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| vec![
    Regex::new(r"^\.env").unwrap(),
    Regex::new(r"package\.json$").unwrap(),
    Regex::new(r"tsconfig.*\.json$").unwrap(),
    Regex::new(r"pnpm-workspace\.yaml$").unwrap(),
    Regex::new(r"\.eslintrc").unwrap(),
    Regex::new(r"vite\.config").unwrap(),
    Regex::new(r"next\.config").unwrap(),
    Regex::new(r"webpack\.config").unwrap(),
]);

static CRITICAL_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| vec![
    Regex::new(r"^\.github/").unwrap(),
    Regex::new(r"^\.gitlab-ci").unwrap(),
    Regex::new(r"Dockerfile").unwrap(),
    Regex::new(r"docker-compose").unwrap(),
    Regex::new(r"^deploy/").unwrap(),
    Regex::new(r"^infra/").unwrap(),
]);

pub fn classify_risk(changed_files: &[String], diff_lines: i32) -> RiskLevel {
    let has_critical = changed_files.iter().any(|f| CRITICAL_PATTERNS.iter().any(|p| p.is_match(f)));
    if has_critical {
        return RiskLevel::Critical;
    }

    let has_config = changed_files.iter().any(|f| CONFIG_PATTERNS.iter().any(|p| p.is_match(f)));
    if has_config && diff_lines > 100 {
        return RiskLevel::High;
    }
    if has_config {
        return RiskLevel::Medium;
    }

    if diff_lines > 500 {
        return RiskLevel::High;
    }
    if diff_lines > 200 {
        return RiskLevel::Medium;
    }

    RiskLevel::Low
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_critical_dockerfile() {
        assert_eq!(classify_risk(&["Dockerfile".into()], 10), RiskLevel::Critical);
    }

    #[test]
    fn test_critical_github_actions() {
        assert_eq!(classify_risk(&[".github/workflows/ci.yml".into()], 5), RiskLevel::Critical);
    }

    #[test]
    fn test_config_high() {
        assert_eq!(classify_risk(&["package.json".into()], 150), RiskLevel::High);
    }

    #[test]
    fn test_config_medium() {
        assert_eq!(classify_risk(&["package.json".into()], 50), RiskLevel::Medium);
    }

    #[test]
    fn test_large_diff_high() {
        assert_eq!(classify_risk(&["src/app.ts".into()], 600), RiskLevel::High);
    }

    #[test]
    fn test_medium_diff() {
        assert_eq!(classify_risk(&["src/app.ts".into()], 300), RiskLevel::Medium);
    }

    #[test]
    fn test_small_change_low() {
        assert_eq!(classify_risk(&["src/app.ts".into()], 50), RiskLevel::Low);
    }

    #[test]
    fn test_empty_files_low() {
        assert_eq!(classify_risk(&[], 0), RiskLevel::Low);
    }
}
