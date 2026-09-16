use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserVO {
    pub erp: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub org_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub org_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardAttachment {
    pub tag_name: String,
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DemandMetadata {
    pub demand_id: i64,
    pub demand_code: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub demand_project_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processor: Option<UserVO>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proposer: Option<UserVO>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receiver: Option<UserVO>,
    pub prd_url: String,
    pub attachments: Vec<CardAttachment>,
    pub raw_link: String,
}
