use std::collections::HashSet;

use serde::{Deserialize, Serialize};

// ---------- Types ----------

/// Severity of an output guard violation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViolationSeverity {
    Block,
    Warn,
}

/// Type of an output guard violation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViolationType {
    TrdScopeEmpty,
    OutOfScope,
    DiffTooLarge,
    BlockedDependency,
}

/// A single output guard violation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Violation {
    #[serde(rename = "type")]
    pub violation_type: ViolationType,
    pub severity: ViolationSeverity,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependency: Option<String>,
}

/// Configuration for the output guard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputGuardConfig {
    /// Glob patterns of allowed file paths (from TRD scope).
    pub allowed_paths: Vec<String>,
    /// Maximum diff lines before triggering a warning (default 500).
    pub max_diff_lines: usize,
    /// Known malicious/blocked dependency names.
    pub blocked_dependencies: Vec<String>,
}

/// Agent output to validate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOutput {
    pub changed_files: Vec<String>,
    pub total_diff_lines: usize,
    pub added_dependencies: Vec<DependencyEntry>,
}

/// A dependency entry with just a name.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyEntry {
    pub name: String,
}

/// Result of the output guard check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardResult {
    pub pass: bool,
    pub violations: Vec<Violation>,
}

// ---------- Defaults ----------

const DEFAULT_MAX_DIFF_LINES: usize = 500;

// ---------- Output Guard ----------

/// Agent output guard -- checks file scope, diff size, and dependency safety.
///
/// - File scope: Agent changes must match allowed glob patterns.
///   Empty allowed_paths => warn (not block) -- more likely TRD extraction failure
///   than "forbid all files".
/// - Diff size: total_diff_lines > max_diff_lines => warn (needs human review).
/// - Dependency safety: new deps in blocked_dependencies list => block.
///
/// pass = true when no "block" severity violations exist.
pub struct OutputGuard;

impl OutputGuard {
    /// Check agent output against the guard configuration.
    pub fn check(agent_output: &AgentOutput, config: &OutputGuardConfig) -> GuardResult {
        let mut violations = Vec::new();

        // 1. File scope check
        Self::check_file_scope(&agent_output.changed_files, &config.allowed_paths, &mut violations);

        // 2. Diff size check
        Self::check_diff_size(agent_output.total_diff_lines, config.max_diff_lines, &mut violations);

        // 3. Dependency safety check
        Self::check_dependencies(
            &agent_output.added_dependencies,
            &config.blocked_dependencies,
            &mut violations,
        );

        let has_blocking = violations
            .iter()
            .any(|v| v.severity == ViolationSeverity::Block);

        GuardResult {
            pass: !has_blocking,
            violations,
        }
    }

    fn check_file_scope(
        changed_files: &[String],
        allowed_paths: &[String],
        violations: &mut Vec<Violation>,
    ) {
        // trdScope empty fallback: warn, not block
        if allowed_paths.is_empty() {
            violations.push(Violation {
                violation_type: ViolationType::TrdScopeEmpty,
                severity: ViolationSeverity::Warn,
                message: "trdScope is empty -- file scope guard skipped, all changes allowed"
                    .into(),
                file: None,
                actual: None,
                threshold: None,
                dependency: None,
            });
            return;
        }

        for file in changed_files {
            let matches = allowed_paths
                .iter()
                .any(|pattern| glob_match(pattern, file));

            if !matches {
                violations.push(Violation {
                    violation_type: ViolationType::OutOfScope,
                    severity: ViolationSeverity::Block,
                    message: format!("File \"{}\" is outside allowed scope", file),
                    file: Some(file.clone()),
                    actual: None,
                    threshold: None,
                    dependency: None,
                });
            }
        }
    }

    fn check_diff_size(
        total_diff_lines: usize,
        max_diff_lines: usize,
        violations: &mut Vec<Violation>,
    ) {
        let threshold = if max_diff_lines > 0 {
            max_diff_lines
        } else {
            DEFAULT_MAX_DIFF_LINES
        };

        if total_diff_lines > threshold {
            violations.push(Violation {
                violation_type: ViolationType::DiffTooLarge,
                severity: ViolationSeverity::Warn,
                message: format!(
                    "Diff size ({} lines) exceeds threshold ({} lines) -- needs human review",
                    total_diff_lines, threshold
                ),
                file: None,
                actual: Some(total_diff_lines),
                threshold: Some(threshold),
                dependency: None,
            });
        }
    }

    fn check_dependencies(
        added_dependencies: &[DependencyEntry],
        blocked_dependencies: &[String],
        violations: &mut Vec<Violation>,
    ) {
        if blocked_dependencies.is_empty() {
            return;
        }

        let blocked_set: HashSet<&str> = blocked_dependencies.iter().map(|s| s.as_str()).collect();

        for dep in added_dependencies {
            if blocked_set.contains(dep.name.as_str()) {
                violations.push(Violation {
                    violation_type: ViolationType::BlockedDependency,
                    severity: ViolationSeverity::Block,
                    message: format!("Dependency \"{}\" is on the blocked list", dep.name),
                    file: None,
                    actual: None,
                    threshold: None,
                    dependency: Some(dep.name.clone()),
                });
            }
        }
    }
}

/// Simple glob matching using the `glob` crate's `Pattern`.
/// Falls back to exact string match if the pattern is invalid.
fn glob_match(pattern: &str, path: &str) -> bool {
    match glob::Pattern::new(pattern) {
        Ok(p) => p.matches(path),
        Err(_) => pattern == path,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> OutputGuardConfig {
        OutputGuardConfig {
            allowed_paths: vec!["src/**".into()],
            max_diff_lines: 500,
            blocked_dependencies: vec!["malicious-pkg".into()],
        }
    }

    #[test]
    fn test_pass_when_all_clear() {
        let output = AgentOutput {
            changed_files: vec!["src/main.rs".into()],
            total_diff_lines: 100,
            added_dependencies: vec![],
        };
        let result = OutputGuard::check(&output, &default_config());
        assert!(result.pass);
        assert!(result.violations.is_empty());
    }

    #[test]
    fn test_empty_scope_warns() {
        let config = OutputGuardConfig {
            allowed_paths: vec![],
            max_diff_lines: 500,
            blocked_dependencies: vec![],
        };
        let output = AgentOutput {
            changed_files: vec!["anything.rs".into()],
            total_diff_lines: 10,
            added_dependencies: vec![],
        };
        let result = OutputGuard::check(&output, &config);
        assert!(result.pass); // warn, not block
        assert_eq!(result.violations.len(), 1);
        assert_eq!(
            result.violations[0].violation_type,
            ViolationType::TrdScopeEmpty
        );
        assert_eq!(result.violations[0].severity, ViolationSeverity::Warn);
    }

    #[test]
    fn test_out_of_scope_blocks() {
        let output = AgentOutput {
            changed_files: vec!["config/secret.toml".into()],
            total_diff_lines: 10,
            added_dependencies: vec![],
        };
        let result = OutputGuard::check(&output, &default_config());
        assert!(!result.pass);
        assert_eq!(result.violations.len(), 1);
        assert_eq!(
            result.violations[0].violation_type,
            ViolationType::OutOfScope
        );
        assert_eq!(result.violations[0].severity, ViolationSeverity::Block);
    }

    #[test]
    fn test_diff_too_large_warns() {
        let output = AgentOutput {
            changed_files: vec!["src/main.rs".into()],
            total_diff_lines: 1000,
            added_dependencies: vec![],
        };
        let result = OutputGuard::check(&output, &default_config());
        assert!(result.pass); // warn, not block
        assert_eq!(result.violations.len(), 1);
        assert_eq!(
            result.violations[0].violation_type,
            ViolationType::DiffTooLarge
        );
        assert_eq!(result.violations[0].actual, Some(1000));
        assert_eq!(result.violations[0].threshold, Some(500));
    }

    #[test]
    fn test_blocked_dependency_blocks() {
        let output = AgentOutput {
            changed_files: vec!["src/main.rs".into()],
            total_diff_lines: 10,
            added_dependencies: vec![DependencyEntry {
                name: "malicious-pkg".into(),
            }],
        };
        let result = OutputGuard::check(&output, &default_config());
        assert!(!result.pass);
        assert_eq!(result.violations.len(), 1);
        assert_eq!(
            result.violations[0].violation_type,
            ViolationType::BlockedDependency
        );
        assert_eq!(
            result.violations[0].dependency.as_deref(),
            Some("malicious-pkg")
        );
    }

    #[test]
    fn test_multiple_violations() {
        let output = AgentOutput {
            changed_files: vec!["out/file.txt".into()],
            total_diff_lines: 1000,
            added_dependencies: vec![DependencyEntry {
                name: "malicious-pkg".into(),
            }],
        };
        let result = OutputGuard::check(&output, &default_config());
        assert!(!result.pass);
        assert_eq!(result.violations.len(), 3); // out_of_scope + diff_too_large + blocked_dep
    }

    #[test]
    fn test_default_max_diff_lines_used_when_zero() {
        let config = OutputGuardConfig {
            allowed_paths: vec!["**".into()],
            max_diff_lines: 0, // should fall back to 500
            blocked_dependencies: vec![],
        };
        let output = AgentOutput {
            changed_files: vec!["src/main.rs".into()],
            total_diff_lines: 501,
            added_dependencies: vec![],
        };
        let result = OutputGuard::check(&output, &config);
        assert_eq!(result.violations.len(), 1);
        assert_eq!(result.violations[0].threshold, Some(500));
    }
}
