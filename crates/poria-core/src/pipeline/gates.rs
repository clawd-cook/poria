use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types::{GateEvaluation, GateOnFail, GatePhase, GateResult, GateRule, StageEnum};

static CR_GRADE_ORDER: Lazy<HashMap<&'static str, u8>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("A+", 10);
    m.insert("A", 9);
    m.insert("A-", 8);
    m.insert("B+", 7);
    m.insert("B", 6);
    m.insert("B-", 5);
    m.insert("C+", 4);
    m.insert("C", 3);
    m.insert("C-", 2);
    m.insert("D", 1);
    m.insert("F", 0);
    m
});

pub fn cr_score_meets_threshold(actual: &str, threshold: &str) -> bool {
    match (CR_GRADE_ORDER.get(actual), CR_GRADE_ORDER.get(threshold)) {
        (Some(a), Some(t)) => a >= t,
        _ => false,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StageResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cr_score: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_coverage: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ci_build_pass: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security_pass: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff_lines: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_conflict: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prd_review_p0_done: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trd_exists: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_changes_exist: Option<bool>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

pub static DEFAULT_GATES: Lazy<Vec<GateRule>> = Lazy::new(|| {
    vec![
        GateRule {
            id: "ci_build".into(),
            name: "CI 构建".into(),
            enabled: true,
            threshold: serde_json::Value::Null,
            on_fail: GateOnFail::Block,
            gate_phase: GatePhase::Deploy,
            regress_to: None,
        },
        GateRule {
            id: "test_coverage".into(),
            name: "测试覆盖率".into(),
            enabled: true,
            threshold: serde_json::json!(80),
            on_fail: GateOnFail::Block,
            gate_phase: GatePhase::Deploy,
            regress_to: None,
        },
        GateRule {
            id: "security_scan".into(),
            name: "安全扫描".into(),
            enabled: true,
            threshold: serde_json::Value::Null,
            on_fail: GateOnFail::Block,
            gate_phase: GatePhase::StageExit,
            regress_to: None,
        },
        GateRule {
            id: "diff_size".into(),
            name: "变更量".into(),
            enabled: true,
            threshold: serde_json::json!(500),
            on_fail: GateOnFail::Warn,
            gate_phase: GatePhase::Deploy,
            regress_to: None,
        },
        GateRule {
            id: "merge_conflict".into(),
            name: "合并冲突".into(),
            enabled: true,
            threshold: serde_json::Value::Null,
            on_fail: GateOnFail::Block,
            gate_phase: GatePhase::Deploy,
            regress_to: None,
        },
        GateRule {
            id: "cr_score".into(),
            name: "CR 评分".into(),
            enabled: true,
            threshold: serde_json::json!("B+"),
            on_fail: GateOnFail::Regress,
            gate_phase: GatePhase::StageExit,
            regress_to: Some(StageEnum::Dev),
        },
        GateRule {
            id: "prd_review_p0".into(),
            name: "PRD 澄清 P0 完成".into(),
            enabled: true,
            threshold: serde_json::Value::Null,
            on_fail: GateOnFail::Block,
            gate_phase: GatePhase::StageEntry,
            regress_to: None,
        },
        GateRule {
            id: "trd_exists".into(),
            name: "TRD 文档存在".into(),
            enabled: true,
            threshold: serde_json::Value::Null,
            on_fail: GateOnFail::Block,
            gate_phase: GatePhase::StageEntry,
            regress_to: None,
        },
        GateRule {
            id: "code_changes_exist".into(),
            name: "代码变更存在".into(),
            enabled: true,
            threshold: serde_json::Value::Null,
            on_fail: GateOnFail::Block,
            gate_phase: GatePhase::StageEntry,
            regress_to: None,
        },
    ]
});

fn evaluate_one(rule: &GateRule, result: &StageResult) -> GateResult {
    match rule.id.as_str() {
        "ci_build" => gate_result(
            rule,
            result.ci_build_pass == Some(true),
            &serde_json::json!(result.ci_build_pass),
        ),
        "test_coverage" => {
            let threshold = rule.threshold.as_f64().unwrap_or(0.0);
            let actual = result.test_coverage.unwrap_or(0.0);
            gate_result(
                rule,
                actual >= threshold,
                &serde_json::json!(result.test_coverage),
            )
        }
        "security_scan" => gate_result(
            rule,
            result.security_pass == Some(true),
            &serde_json::json!(result.security_pass),
        ),
        "diff_size" => {
            let threshold = rule.threshold.as_i64().unwrap_or(0) as i32;
            let actual = result.diff_lines.unwrap_or(0);
            gate_result(
                rule,
                actual <= threshold,
                &serde_json::json!(result.diff_lines),
            )
        }
        "merge_conflict" => gate_result(
            rule,
            result.has_conflict != Some(true),
            &serde_json::json!(result.has_conflict),
        ),
        "cr_score" => {
            let threshold = rule.threshold.as_str().unwrap_or("");
            let actual = result.cr_score.as_deref().unwrap_or("");
            gate_result(
                rule,
                cr_score_meets_threshold(actual, threshold),
                &serde_json::json!(result.cr_score),
            )
        }
        "prd_review_p0" => gate_result(
            rule,
            result.prd_review_p0_done == Some(true),
            &serde_json::json!(result.prd_review_p0_done),
        ),
        "trd_exists" => gate_result(
            rule,
            result.trd_exists == Some(true),
            &serde_json::json!(result.trd_exists),
        ),
        "code_changes_exist" => gate_result(
            rule,
            result.code_changes_exist == Some(true),
            &serde_json::json!(result.code_changes_exist),
        ),
        _ => gate_result(rule, true, &serde_json::Value::Null),
    }
}

fn gate_result(rule: &GateRule, pass: bool, actual: &serde_json::Value) -> GateResult {
    let message = if pass {
        format!("{}: 通过", rule.name)
    } else {
        format!(
            "{}: 未通过 (actual={}, threshold={})",
            rule.name, actual, rule.threshold
        )
    };
    GateResult {
        rule_id: rule.id.clone(),
        pass,
        actual: actual.clone(),
        threshold: rule.threshold.clone(),
        message,
    }
}

pub fn evaluate_gates(
    result: &StageResult,
    rules: &[GateRule],
    phase: GatePhase,
) -> GateEvaluation {
    let applicable: Vec<_> = rules
        .iter()
        .filter(|r| r.enabled && r.gate_phase == phase)
        .collect();
    let details: Vec<GateResult> = applicable
        .iter()
        .map(|rule| evaluate_one(rule, result))
        .collect();

    let blocking_failures: Vec<GateResult> = details
        .iter()
        .filter(|r| {
            if r.pass {
                return false;
            }
            rules
                .iter()
                .find(|rule| rule.id == r.rule_id)
                .map(|rule| {
                    rule.on_fail == GateOnFail::Block || rule.on_fail == GateOnFail::Regress
                })
                .unwrap_or(false)
        })
        .cloned()
        .collect();

    let warn_failures: Vec<GateResult> = details
        .iter()
        .filter(|r| {
            if r.pass {
                return false;
            }
            rules
                .iter()
                .find(|rule| rule.id == r.rule_id)
                .map(|rule| rule.on_fail == GateOnFail::Warn)
                .unwrap_or(false)
        })
        .cloned()
        .collect();

    GateEvaluation {
        all_pass: blocking_failures.is_empty(),
        details,
        blocking_failures,
        warn_failures,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cr_score_meets_threshold() {
        assert!(cr_score_meets_threshold("A", "B+"));
        assert!(cr_score_meets_threshold("B+", "B+"));
        assert!(!cr_score_meets_threshold("B", "B+"));
        assert!(!cr_score_meets_threshold("C", "B+"));
        assert!(!cr_score_meets_threshold("invalid", "B+"));
    }

    #[test]
    fn test_evaluate_all_pass() {
        let result = StageResult {
            ci_build_pass: Some(true),
            test_coverage: Some(90.0),
            security_pass: Some(true),
            diff_lines: Some(100),
            has_conflict: Some(false),
            cr_score: Some("A".into()),
            ..Default::default()
        };
        let eval = evaluate_gates(&result, &DEFAULT_GATES, GatePhase::Deploy);
        assert!(eval.all_pass);
        assert!(eval.blocking_failures.is_empty());
    }

    #[test]
    fn test_evaluate_coverage_fail() {
        let result = StageResult {
            ci_build_pass: Some(true),
            test_coverage: Some(50.0),
            has_conflict: Some(false),
            diff_lines: Some(100),
            ..Default::default()
        };
        let eval = evaluate_gates(&result, &DEFAULT_GATES, GatePhase::Deploy);
        assert!(!eval.all_pass);
        assert_eq!(eval.blocking_failures.len(), 1);
        assert_eq!(eval.blocking_failures[0].rule_id, "test_coverage");
    }

    #[test]
    fn test_evaluate_diff_size_warn() {
        let result = StageResult {
            ci_build_pass: Some(true),
            test_coverage: Some(90.0),
            has_conflict: Some(false),
            diff_lines: Some(600),
            ..Default::default()
        };
        let eval = evaluate_gates(&result, &DEFAULT_GATES, GatePhase::Deploy);
        assert!(eval.all_pass);
        assert_eq!(eval.warn_failures.len(), 1);
        assert_eq!(eval.warn_failures[0].rule_id, "diff_size");
    }

    #[test]
    fn test_evaluate_cr_score_regress() {
        let result = StageResult {
            cr_score: Some("C".into()),
            security_pass: Some(true),
            ..Default::default()
        };
        let eval = evaluate_gates(&result, &DEFAULT_GATES, GatePhase::StageExit);
        assert!(!eval.all_pass);
        assert!(eval
            .blocking_failures
            .iter()
            .any(|r| r.rule_id == "cr_score"));
    }

    #[test]
    fn test_prd_review_p0_gate_pass() {
        let result = StageResult {
            prd_review_p0_done: Some(true),
            ..Default::default()
        };
        let eval = evaluate_gates(&result, &DEFAULT_GATES, GatePhase::StageEntry);
        let p0_result = eval.details.iter().find(|r| r.rule_id == "prd_review_p0");
        assert!(p0_result.is_some());
        assert!(p0_result.unwrap().pass);
    }

    #[test]
    fn test_prd_review_p0_gate_fail() {
        let result = StageResult {
            prd_review_p0_done: Some(false),
            ..Default::default()
        };
        let eval = evaluate_gates(&result, &DEFAULT_GATES, GatePhase::StageEntry);
        assert!(!eval.all_pass);
        assert!(eval
            .blocking_failures
            .iter()
            .any(|r| r.rule_id == "prd_review_p0"));
    }

    #[test]
    fn test_prd_review_p0_gate_missing() {
        let result = StageResult::default();
        let eval = evaluate_gates(&result, &DEFAULT_GATES, GatePhase::StageEntry);
        assert!(!eval.all_pass);
        assert!(eval
            .blocking_failures
            .iter()
            .any(|r| r.rule_id == "prd_review_p0"));
    }

    #[test]
    fn test_trd_exists_gate_pass() {
        let result = StageResult {
            trd_exists: Some(true),
            prd_review_p0_done: Some(true),
            code_changes_exist: Some(true),
            ..Default::default()
        };
        let eval = evaluate_gates(&result, &DEFAULT_GATES, GatePhase::StageEntry);
        assert!(eval.all_pass);
    }

    #[test]
    fn test_trd_exists_gate_fail() {
        let result = StageResult {
            trd_exists: Some(false),
            prd_review_p0_done: Some(true),
            code_changes_exist: Some(true),
            ..Default::default()
        };
        let eval = evaluate_gates(&result, &DEFAULT_GATES, GatePhase::StageEntry);
        assert!(!eval.all_pass);
        assert!(eval
            .blocking_failures
            .iter()
            .any(|r| r.rule_id == "trd_exists"));
    }

    #[test]
    fn test_code_changes_exist_gate_pass() {
        let result = StageResult {
            code_changes_exist: Some(true),
            prd_review_p0_done: Some(true),
            trd_exists: Some(true),
            ..Default::default()
        };
        let eval = evaluate_gates(&result, &DEFAULT_GATES, GatePhase::StageEntry);
        assert!(eval.all_pass);
    }

    #[test]
    fn test_code_changes_exist_gate_fail() {
        let result = StageResult {
            code_changes_exist: Some(false),
            prd_review_p0_done: Some(true),
            trd_exists: Some(true),
            ..Default::default()
        };
        let eval = evaluate_gates(&result, &DEFAULT_GATES, GatePhase::StageEntry);
        assert!(!eval.all_pass);
        assert!(eval
            .blocking_failures
            .iter()
            .any(|r| r.rule_id == "code_changes_exist"));
    }

    #[test]
    fn test_stage_entry_all_pass() {
        let result = StageResult {
            prd_review_p0_done: Some(true),
            trd_exists: Some(true),
            code_changes_exist: Some(true),
            ..Default::default()
        };
        let eval = evaluate_gates(&result, &DEFAULT_GATES, GatePhase::StageEntry);
        assert!(eval.all_pass);
        assert!(eval.blocking_failures.is_empty());
        assert_eq!(eval.details.len(), 3);
    }

    #[test]
    fn test_default_gates_count() {
        assert_eq!(DEFAULT_GATES.len(), 9);
        let entry_gates: Vec<_> = DEFAULT_GATES
            .iter()
            .filter(|g| g.gate_phase == GatePhase::StageEntry)
            .collect();
        assert_eq!(entry_gates.len(), 3);
    }
}
