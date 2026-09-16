use serde::{Deserialize, Serialize};

use super::pipeline_types::StageEnum;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GatePhase {
    StageEntry,
    StageExit,
    Deploy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateOnFail {
    Block,
    Warn,
    Regress,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateRule {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub threshold: serde_json::Value,
    pub on_fail: GateOnFail,
    pub gate_phase: GatePhase,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub regress_to: Option<StageEnum>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateResult {
    pub rule_id: String,
    pub pass: bool,
    pub actual: serde_json::Value,
    pub threshold: serde_json::Value,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateEvaluation {
    pub all_pass: bool,
    pub details: Vec<GateResult>,
    pub blocking_failures: Vec<GateResult>,
    pub warn_failures: Vec<GateResult>,
}
