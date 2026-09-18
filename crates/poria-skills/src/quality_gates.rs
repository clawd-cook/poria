use std::path::Path;

use poria_core::pipeline::{
    parse_cargo_audit, parse_istanbul_summary, parse_lcov, parse_npm_audit, CoverageReport,
    QualitySource, SecurityScanReport,
};
use poria_resources::{terminal, TerminalExecInput};

const COVERAGE_CANDIDATES: &[&str] = &[
    "coverage/coverage-summary.json",
    "coverage-summary.json",
    "coverage/coverage-final.json",
    "coverage/lcov.info",
    "lcov.info",
];

const AUDIT_CANDIDATES: &[&str] = &[
    "pnpm-audit.json",
    "npm-audit.json",
    "audit.json",
    "cargo-audit.json",
];

/// Read coverage from a known report file. Missing file → not a pass.
pub fn collect_coverage_report(worktree: &Path) -> CoverageReport {
    for rel in COVERAGE_CANDIDATES {
        let path = worktree.join(rel);
        if !path.is_file() {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        let mut report = if rel.ends_with(".json") {
            parse_istanbul_summary(&content)
        } else {
            parse_lcov(&content)
        };
        if report.percent.is_some() {
            report.path = Some(rel.to_string());
            return report;
        }
    }
    CoverageReport::missing()
}

/// Read a saved audit JSON, otherwise run `pnpm`/`npm`/`cargo` audit.
pub async fn collect_security_scan(worktree: &Path) -> SecurityScanReport {
    if let Some(report) = read_saved_audit(worktree) {
        return report;
    }
    if worktree.join("package.json").is_file() {
        let report = run_json_command(
            worktree,
            if worktree.join("pnpm-lock.yaml").is_file() {
                "pnpm audit --json"
            } else {
                "npm audit --json"
            },
        )
        .await;
        if let Some(json) = report {
            return parse_npm_audit(&json);
        }
        return SecurityScanReport::missing("pnpm/npm audit produced no JSON");
    }
    if worktree.join("Cargo.toml").is_file() {
        let report = run_json_command(worktree, "cargo audit --json").await;
        if let Some(json) = report {
            return parse_cargo_audit(&json);
        }
        return SecurityScanReport::missing("cargo audit produced no JSON");
    }
    SecurityScanReport::missing("no package.json or Cargo.toml to scan")
}

fn read_saved_audit(worktree: &Path) -> Option<SecurityScanReport> {
    for rel in AUDIT_CANDIDATES {
        let path = worktree.join(rel);
        if !path.is_file() {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        let report = if rel.contains("cargo") {
            parse_cargo_audit(&content)
        } else {
            parse_npm_audit(&content)
        };
        if report.source != QualitySource::Missing {
            return Some(report);
        }
    }
    None
}

async fn run_json_command(worktree: &Path, command: &str) -> Option<String> {
    let result = terminal::exec(TerminalExecInput {
        command: command.into(),
        cwd: Some(worktree.to_string_lossy().into_owned()),
        env: None,
        timeout_ms: Some(60_000),
    })
    .await
    .ok()?;
    let stdout = result.stdout.trim();
    if stdout.starts_with('{') || stdout.starts_with('[') {
        return Some(stdout.to_string());
    }
    let stderr = result.stderr.trim();
    if stderr.starts_with('{') || stderr.starts_with('[') {
        return Some(stderr.to_string());
    }
    None
}

pub fn coverage_block_message(report: &CoverageReport, threshold: f64) -> String {
    match report.percent {
        None => {
            "coverage missing: 未找到 coverage-summary.json / lcov.info，不能把覆盖率当通过".into()
        }
        Some(pct) if pct + f64::EPSILON < threshold => {
            format!(
                "coverage {}% below threshold {}% (source={})",
                pct,
                threshold,
                report.source.as_str()
            )
        }
        _ => String::new(),
    }
}

pub fn security_block_message(report: &SecurityScanReport) -> String {
    match report.pass {
        Some(true) => String::new(),
        Some(false) => format!(
            "security_scan blocked: high={} critical={} ({})",
            report.high,
            report.critical,
            report.detail.clone().unwrap_or_default()
        ),
        None => format!(
            "security_scan missing: {}",
            report
                .detail
                .clone()
                .unwrap_or_else(|| "no scanner output".into())
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn coverage_from_istanbul_file() {
        let dir = std::env::temp_dir().join(format!(
            "poria-cov-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        fs::create_dir_all(dir.join("coverage")).unwrap();
        fs::write(
            dir.join("coverage/coverage-summary.json"),
            r#"{"total":{"lines":{"pct":82.0}}}"#,
        )
        .unwrap();
        let report = collect_coverage_report(&dir);
        assert_eq!(report.percent, Some(82.0));
        assert_eq!(report.source, QualitySource::IstanbulSummary);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn coverage_missing_without_report() {
        let dir = std::env::temp_dir().join(format!(
            "poria-cov-empty-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        fs::create_dir_all(&dir).unwrap();
        let report = collect_coverage_report(&dir);
        assert!(report.percent.is_none());
        assert_eq!(report.source, QualitySource::Missing);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn security_from_saved_npm_audit() {
        let dir = std::env::temp_dir().join(format!(
            "poria-audit-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("npm-audit.json"),
            r#"{"metadata":{"vulnerabilities":{"high":1,"critical":0}}}"#,
        )
        .unwrap();
        let report = read_saved_audit(&dir).expect("audit");
        assert_eq!(report.pass, Some(false));
        assert_eq!(report.high, 1);
        fs::remove_dir_all(&dir).ok();
    }
}
