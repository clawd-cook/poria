use serde::{Deserialize, Serialize};

/// Where a quality number came from. Agent self-report is never a valid source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualitySource {
    GitlabPipeline,
    GitlabCommitStatus,
    IstanbulSummary,
    Lcov,
    NpmAudit,
    CargoAudit,
    Missing,
}

impl QualitySource {
    pub fn as_str(&self) -> &'static str {
        match self {
            QualitySource::GitlabPipeline => "gitlab_pipeline",
            QualitySource::GitlabCommitStatus => "gitlab_commit_status",
            QualitySource::IstanbulSummary => "istanbul_summary",
            QualitySource::Lcov => "lcov",
            QualitySource::NpmAudit => "npm_audit",
            QualitySource::CargoAudit => "cargo_audit",
            QualitySource::Missing => "missing",
        }
    }

    pub fn is_measured(&self) -> bool {
        !matches!(self, QualitySource::Missing)
    }
}

/// CI build from Coding/GitLab (or EasyCI) — never from the model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CiBuildReport {
    pub pass: Option<bool>,
    pub source: QualitySource,
    pub status: Option<String>,
    pub web_url: Option<String>,
}

impl CiBuildReport {
    pub fn missing(reason: impl Into<String>) -> Self {
        Self {
            pass: None,
            source: QualitySource::Missing,
            status: Some(reason.into()),
            web_url: None,
        }
    }
}

/// Line coverage percent from a report file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoverageReport {
    pub percent: Option<f64>,
    pub source: QualitySource,
    pub path: Option<String>,
}

impl CoverageReport {
    pub fn missing() -> Self {
        Self {
            percent: None,
            source: QualitySource::Missing,
            path: None,
        }
    }
}

/// Security scan from npm/pnpm/cargo audit JSON.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityScanReport {
    pub pass: Option<bool>,
    pub source: QualitySource,
    pub high: u32,
    pub critical: u32,
    pub detail: Option<String>,
}

impl SecurityScanReport {
    pub fn missing(reason: impl Into<String>) -> Self {
        Self {
            pass: None,
            source: QualitySource::Missing,
            high: 0,
            critical: 0,
            detail: Some(reason.into()),
        }
    }
}

/// Parse GitLab `/merge_requests/:iid/pipelines` JSON (array, newest first).
pub fn parse_gitlab_pipelines(json: &serde_json::Value) -> CiBuildReport {
    let Some(items) = json.as_array() else {
        return CiBuildReport::missing("CI pipelines response is not an array");
    };
    let Some(latest) = items.first() else {
        return CiBuildReport::missing("no CI pipeline on merge request");
    };
    let status = latest
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    let web_url = latest
        .get("web_url")
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned);
    let pass = match status.as_str() {
        "success" => Some(true),
        "failed" | "canceled" | "cancelled" => Some(false),
        _ => None,
    };
    CiBuildReport {
        pass,
        source: QualitySource::GitlabPipeline,
        status: Some(status),
        web_url,
    }
}

/// Parse GitLab commit statuses JSON array.
pub fn parse_gitlab_commit_statuses(json: &serde_json::Value) -> CiBuildReport {
    let Some(items) = json.as_array() else {
        return CiBuildReport::missing("commit statuses response is not an array");
    };
    if items.is_empty() {
        return CiBuildReport::missing("no commit check statuses");
    }
    let mut saw_pending = false;
    let mut saw_failure = false;
    let mut saw_success = false;
    let mut web_url = None;
    for item in items {
        let status = item
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if web_url.is_none() {
            web_url = item
                .get("target_url")
                .and_then(|v| v.as_str())
                .map(ToOwned::to_owned);
        }
        match status.as_str() {
            "success" => saw_success = true,
            "failed" | "failure" | "canceled" | "cancelled" => saw_failure = true,
            "pending" | "running" | "created" | "manual" => saw_pending = true,
            _ => {}
        }
    }
    let pass = if saw_failure {
        Some(false)
    } else if saw_pending {
        None
    } else if saw_success {
        Some(true)
    } else {
        None
    };
    CiBuildReport {
        pass,
        source: QualitySource::GitlabCommitStatus,
        status: Some(if saw_failure {
            "failed".into()
        } else if saw_pending {
            "pending".into()
        } else if saw_success {
            "success".into()
        } else {
            "unknown".into()
        }),
        web_url,
    }
}

/// Istanbul / nyc `coverage-summary.json`.
pub fn parse_istanbul_summary(json: &str) -> CoverageReport {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(json) else {
        return CoverageReport::missing();
    };
    let pct = value
        .pointer("/total/lines/pct")
        .and_then(|v| v.as_f64())
        .or_else(|| {
            value
                .pointer("/total/statements/pct")
                .and_then(|v| v.as_f64())
        });
    CoverageReport {
        percent: pct,
        source: if pct.is_some() {
            QualitySource::IstanbulSummary
        } else {
            QualitySource::Missing
        },
        path: None,
    }
}

/// Minimal LCOV: sum `LF` / `LH` across records.
pub fn parse_lcov(text: &str) -> CoverageReport {
    let mut found = 0u64;
    let mut hit = 0u64;
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("LF:") {
            found = found.saturating_add(value.trim().parse().unwrap_or(0));
        }
        if let Some(value) = line.strip_prefix("LH:") {
            hit = hit.saturating_add(value.trim().parse().unwrap_or(0));
        }
    }
    if found == 0 {
        return CoverageReport::missing();
    }
    CoverageReport {
        percent: Some((hit as f64) * 100.0 / found as f64),
        source: QualitySource::Lcov,
        path: None,
    }
}

/// npm / pnpm `audit --json`. High or critical → fail.
pub fn parse_npm_audit(json: &str) -> SecurityScanReport {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(json) else {
        return SecurityScanReport::missing("npm audit JSON invalid");
    };
    let vulns = value
        .pointer("/metadata/vulnerabilities")
        .or_else(|| value.pointer("/data/vulnerabilities"));
    let high = json_u32(vulns, "high");
    let critical = json_u32(vulns, "critical");
    if vulns.is_none()
        && value.get("advisories").is_none()
        && value.get("vulnerabilities").is_none()
    {
        return SecurityScanReport::missing("npm audit JSON missing vulnerability counts");
    }
    let extra_high = count_advisory_severity(&value, "high");
    let extra_crit = count_advisory_severity(&value, "critical");
    let high = high.max(extra_high);
    let critical = critical.max(extra_crit);
    SecurityScanReport {
        pass: Some(high == 0 && critical == 0),
        source: QualitySource::NpmAudit,
        high,
        critical,
        detail: Some(format!("high={high} critical={critical}")),
    }
}

/// `cargo audit --json` warning list.
pub fn parse_cargo_audit(json: &str) -> SecurityScanReport {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(json) else {
        return SecurityScanReport::missing("cargo audit JSON invalid");
    };
    let vulns = value
        .pointer("/vulnerabilities/list")
        .and_then(|v| v.as_array())
        .or_else(|| value.get("vulnerabilities").and_then(|v| v.as_array()));
    let Some(list) = vulns else {
        if value.get("vulnerabilities").is_some() {
            return SecurityScanReport {
                pass: Some(true),
                source: QualitySource::CargoAudit,
                high: 0,
                critical: 0,
                detail: Some("high=0 critical=0".into()),
            };
        }
        return SecurityScanReport::missing("cargo audit JSON missing vulnerabilities");
    };
    let mut high = 0u32;
    let mut critical = 0u32;
    for item in list {
        let severity = item
            .pointer("/advisory/cvss")
            .and_then(|v| v.as_str())
            .or_else(|| item.pointer("/advisory/severity").and_then(|v| v.as_str()))
            .unwrap_or("")
            .to_ascii_lowercase();
        if severity.contains("critical") {
            critical += 1;
        } else {
            high += 1;
        }
    }
    SecurityScanReport {
        pass: Some(high == 0 && critical == 0),
        source: QualitySource::CargoAudit,
        high,
        critical,
        detail: Some(format!("high={high} critical={critical}")),
    }
}

fn json_u32(parent: Option<&serde_json::Value>, key: &str) -> u32 {
    parent
        .and_then(|v| v.get(key))
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32
}

fn count_advisory_severity(root: &serde_json::Value, severity: &str) -> u32 {
    let Some(advisories) = root.get("advisories").and_then(|v| v.as_object()) else {
        return 0;
    };
    advisories
        .values()
        .filter(|item| {
            item.get("severity")
                .and_then(|v| v.as_str())
                .map(|s| s.eq_ignore_ascii_case(severity))
                .unwrap_or(false)
        })
        .count() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gitlab_pipeline_success() {
        let json = serde_json::json!([{"status":"success","web_url":"https://ci.example/1"}]);
        let report = parse_gitlab_pipelines(&json);
        assert_eq!(report.pass, Some(true));
        assert_eq!(report.source, QualitySource::GitlabPipeline);
    }

    #[test]
    fn gitlab_pipeline_missing_is_not_pass() {
        let report = parse_gitlab_pipelines(&serde_json::json!([]));
        assert_eq!(report.pass, None);
        assert_eq!(report.source, QualitySource::Missing);
    }

    #[test]
    fn gitlab_pipeline_running_is_not_pass() {
        let json = serde_json::json!([{"status":"running"}]);
        let report = parse_gitlab_pipelines(&json);
        assert_eq!(report.pass, None);
        assert_eq!(report.source, QualitySource::GitlabPipeline);
    }

    #[test]
    fn istanbul_reads_line_pct() {
        let report = parse_istanbul_summary(r#"{"total":{"lines":{"pct":81.5}}}"#);
        assert_eq!(report.percent, Some(81.5));
        assert_eq!(report.source, QualitySource::IstanbulSummary);
    }

    #[test]
    fn lcov_computes_percent() {
        let report = parse_lcov("SF:src/a.ts\nLF:10\nLH:8\nend_of_record\n");
        assert_eq!(report.percent, Some(80.0));
        assert_eq!(report.source, QualitySource::Lcov);
    }

    #[test]
    fn npm_audit_blocks_high() {
        let report = parse_npm_audit(
            r#"{"metadata":{"vulnerabilities":{"info":0,"low":1,"moderate":0,"high":2,"critical":0}}}"#,
        );
        assert_eq!(report.pass, Some(false));
        assert_eq!(report.high, 2);
        assert_eq!(report.source, QualitySource::NpmAudit);
    }

    #[test]
    fn npm_audit_passes_without_high() {
        let report =
            parse_npm_audit(r#"{"metadata":{"vulnerabilities":{"high":0,"critical":0,"low":3}}}"#);
        assert_eq!(report.pass, Some(true));
    }

    #[test]
    fn npm_audit_invalid_is_missing() {
        let report = parse_npm_audit("not-json");
        assert_eq!(report.pass, None);
        assert_eq!(report.source, QualitySource::Missing);
    }

    #[test]
    fn cargo_audit_empty_list_passes() {
        let report = parse_cargo_audit(r#"{"vulnerabilities":{"list":[]}}"#);
        assert_eq!(report.pass, Some(true));
        assert_eq!(report.source, QualitySource::CargoAudit);
    }

    #[test]
    fn gitlab_commit_status_pending_is_not_pass() {
        let json = serde_json::json!([{"status":"pending","target_url":"https://ci.example/2"}]);
        let report = parse_gitlab_commit_statuses(&json);
        assert_eq!(report.pass, None);
        assert_eq!(report.source, QualitySource::GitlabCommitStatus);
    }
}
