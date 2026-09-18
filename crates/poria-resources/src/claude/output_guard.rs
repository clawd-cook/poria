use std::collections::HashSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::terminal::{git_diff_numstat, git_show_path, git_status_porcelain};

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

/// Names that must never be added by GenCode (typosquats / historically malware).
pub const DEFAULT_BLOCKED_DEPENDENCIES: &[&str] = &[
    "flatmap-stream",
    "event-stream",
    "crossenv",
    "cross-env.js",
    "electron-native-notify",
    "malicious-pkg",
];

impl Default for OutputGuardConfig {
    fn default() -> Self {
        Self {
            allowed_paths: Vec::new(),
            max_diff_lines: DEFAULT_MAX_DIFF_LINES,
            blocked_dependencies: default_blocked_dependencies(),
        }
    }
}

impl OutputGuardConfig {
    /// Guard config from Design `trd_scope` globs plus default blocked deps.
    pub fn from_trd_scope(allowed_paths: Vec<String>) -> Self {
        Self {
            allowed_paths,
            max_diff_lines: DEFAULT_MAX_DIFF_LINES,
            blocked_dependencies: default_blocked_dependencies(),
        }
    }
}

pub fn default_blocked_dependencies() -> Vec<String> {
    DEFAULT_BLOCKED_DEPENDENCIES
        .iter()
        .map(|name| (*name).to_string())
        .collect()
}

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
        Self::check_file_scope(
            &agent_output.changed_files,
            &config.allowed_paths,
            &mut violations,
        );

        // 2. Diff size check
        Self::check_diff_size(
            agent_output.total_diff_lines,
            config.max_diff_lines,
            &mut violations,
        );

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

impl GuardResult {
    pub fn blocking(&self) -> Vec<&Violation> {
        self.violations
            .iter()
            .filter(|v| v.severity == ViolationSeverity::Block)
            .collect()
    }

    pub fn warnings(&self) -> Vec<&Violation> {
        self.violations
            .iter()
            .filter(|v| v.severity == ViolationSeverity::Warn)
            .collect()
    }

    /// Error string classified as `OutOfScopeChange` / `SecurityViolation`.
    pub fn block_error_message(&self) -> String {
        let blocking = self.blocking();
        let files: Vec<&str> = blocking.iter().filter_map(|v| v.file.as_deref()).collect();
        let deps: Vec<&str> = blocking
            .iter()
            .filter_map(|v| v.dependency.as_deref())
            .collect();
        let details = blocking
            .iter()
            .map(|v| v.message.as_str())
            .collect::<Vec<_>>()
            .join("; ");
        format!(
            "OutputGuardError: out_of_scope files: [{}]; blocked_dependency: [{}]; {}。不得进入 CR。",
            files.join(", "),
            deps.join(", "),
            details
        )
    }

    pub fn ui_gate_results(&self) -> serde_json::Value {
        serde_json::json!([{
            "gate": "OutputGuard",
            "passed": self.pass,
            "actual": if self.pass { "pass" } else { "block" },
            "threshold": "pass",
        }])
    }
}

/// Collect worktree dirt for OutputGuard (changed files, diff size, new deps).
pub async fn collect_worktree_agent_output(worktree: &Path) -> AgentOutput {
    let porcelain = git_status_porcelain(worktree).await.unwrap_or_default();
    let changed_files = parse_porcelain_paths(&porcelain);
    let mut total_diff_lines =
        parse_numstat_total(&git_diff_numstat(worktree).await.unwrap_or_default());
    for file in &changed_files {
        if porcelain_untracked(&porcelain, file) {
            total_diff_lines += count_file_lines(&worktree.join(file));
        }
    }
    let added_dependencies = collect_added_dependencies(worktree, &changed_files).await;
    AgentOutput {
        changed_files,
        total_diff_lines,
        added_dependencies,
    }
}

pub fn parse_porcelain_paths(porcelain: &str) -> Vec<String> {
    porcelain
        .lines()
        .filter_map(|line| {
            if line.len() < 4 {
                return None;
            }
            let rest = line[3..].trim();
            if rest.is_empty() {
                return None;
            }
            let path = if let Some((_, new_path)) = rest.split_once(" -> ") {
                new_path
            } else {
                rest
            };
            let cleaned = path.trim_matches('"').trim();
            if cleaned.is_empty() || cleaned.ends_with('/') {
                None
            } else {
                Some(cleaned.to_string())
            }
        })
        .collect()
}

pub fn parse_numstat_total(numstat: &str) -> usize {
    numstat
        .lines()
        .map(|line| {
            let mut parts = line.split('\t');
            let added = parse_numstat_count(parts.next());
            let deleted = parse_numstat_count(parts.next());
            added.saturating_add(deleted)
        })
        .sum()
}

fn parse_numstat_count(raw: Option<&str>) -> usize {
    match raw {
        Some("-") | None => 0,
        Some(value) => value.parse().unwrap_or(0),
    }
}

fn porcelain_untracked(porcelain: &str, file: &str) -> bool {
    porcelain
        .lines()
        .any(|line| line.starts_with("??") && line[2..].trim().trim_matches('"') == file)
}

fn count_file_lines(path: &Path) -> usize {
    std::fs::read_to_string(path)
        .map(|content| content.lines().count())
        .unwrap_or(0)
}

async fn collect_added_dependencies(
    worktree: &Path,
    changed_files: &[String],
) -> Vec<DependencyEntry> {
    let mut added = Vec::new();
    if changed_files
        .iter()
        .any(|f| f == "package.json" || f.ends_with("/package.json"))
    {
        let rel = changed_files
            .iter()
            .find(|f| f.ends_with("package.json"))
            .map(String::as_str)
            .unwrap_or("package.json");
        let current = std::fs::read_to_string(worktree.join(rel)).unwrap_or_default();
        let previous = git_show_path(worktree, &format!("HEAD:{rel}"))
            .await
            .ok()
            .flatten()
            .unwrap_or_default();
        added.extend(added_npm_dependencies(&previous, &current));
    }
    if changed_files
        .iter()
        .any(|f| f == "Cargo.toml" || f.ends_with("/Cargo.toml"))
    {
        let rel = changed_files
            .iter()
            .find(|f| f.ends_with("Cargo.toml"))
            .map(String::as_str)
            .unwrap_or("Cargo.toml");
        let current = std::fs::read_to_string(worktree.join(rel)).unwrap_or_default();
        let previous = git_show_path(worktree, &format!("HEAD:{rel}"))
            .await
            .ok()
            .flatten()
            .unwrap_or_default();
        added.extend(added_cargo_dependencies(&previous, &current));
    }
    added
}

pub fn added_npm_dependencies(previous: &str, current: &str) -> Vec<DependencyEntry> {
    let prev = npm_dep_names(previous);
    let curr = npm_dep_names(current);
    curr.into_iter()
        .filter(|name| !prev.contains(name))
        .map(|name| DependencyEntry { name })
        .collect()
}

fn npm_dep_names(json: &str) -> HashSet<String> {
    let mut names = HashSet::new();
    let Ok(value) = serde_json::from_str::<serde_json::Value>(json) else {
        return names;
    };
    let Some(obj) = value.as_object() else {
        return names;
    };
    for key in [
        "dependencies",
        "devDependencies",
        "optionalDependencies",
        "peerDependencies",
    ] {
        if let Some(map) = obj.get(key).and_then(|v| v.as_object()) {
            names.extend(map.keys().cloned());
        }
    }
    names
}

pub fn added_cargo_dependencies(previous: &str, current: &str) -> Vec<DependencyEntry> {
    let prev = cargo_dep_names(previous);
    let curr = cargo_dep_names(current);
    curr.into_iter()
        .filter(|name| !prev.contains(name))
        .map(|name| DependencyEntry { name })
        .collect()
}

fn cargo_dep_names(toml: &str) -> HashSet<String> {
    let mut names = HashSet::new();
    let mut in_deps = false;
    for line in toml.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_deps = matches!(
                trimmed,
                "[dependencies]"
                    | "[dev-dependencies]"
                    | "[build-dependencies]"
                    | "[workspace.dependencies]"
            );
            continue;
        }
        if !in_deps || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((name, _)) = trimmed.split_once('=') {
            let name = name.trim();
            if !name.is_empty() && !name.contains('.') {
                names.insert(name.to_string());
            }
        }
    }
    names
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

    #[test]
    fn porcelain_paths_skip_directories_and_take_rename_target() {
        let files = parse_porcelain_paths(
            " M src/foo.ts\n?? src/bar.ts\nR  old.ts -> src/new.ts\n?? tmp/\n",
        );
        assert_eq!(
            files,
            vec![
                "src/foo.ts".to_string(),
                "src/bar.ts".to_string(),
                "src/new.ts".to_string()
            ]
        );
    }

    #[test]
    fn numstat_sums_added_and_deleted() {
        let total = parse_numstat_total("10\t2\tsrc/a.ts\n-\t-\tbin.png\n3\t1\tsrc/b.ts\n");
        assert_eq!(total, 16);
    }

    #[test]
    fn npm_added_deps_detects_new_keys() {
        let previous = r#"{"dependencies":{"antd":"6.0.0"}}"#;
        let current = r#"{"dependencies":{"antd":"6.0.0","flatmap-stream":"1.0.0"},"devDependencies":{"vitest":"4.0.0"}}"#;
        let added = added_npm_dependencies(previous, current);
        let names: Vec<_> = added.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"flatmap-stream"));
        assert!(names.contains(&"vitest"));
        assert!(!names.contains(&"antd"));
    }

    #[test]
    fn block_error_message_lists_files_and_deps() {
        let output = AgentOutput {
            changed_files: vec!["config/secret.toml".into()],
            total_diff_lines: 10,
            added_dependencies: vec![DependencyEntry {
                name: "malicious-pkg".into(),
            }],
        };
        let result = OutputGuard::check(&output, &default_config());
        let message = result.block_error_message();
        assert!(message.contains("OutputGuardError"));
        assert!(message.contains("out_of_scope"));
        assert!(message.contains("config/secret.toml"));
        assert!(message.contains("blocked_dependency"));
        assert!(message.contains("malicious-pkg"));
        assert!(message.contains("不得进入 CR"));
    }
}
