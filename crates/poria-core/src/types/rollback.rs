use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RollbackCommandType {
    DeleteBranch,
    CloseMr,
    RevertCommit,
    RemoveWorktree,
    RemoveDirectory,
    RevertMr,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackCommand {
    #[serde(rename = "type")]
    pub command_type: RollbackCommandType,
    pub params: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackInstruction {
    pub stage_index: i32,
    pub commands: Vec<RollbackCommand>,
}
