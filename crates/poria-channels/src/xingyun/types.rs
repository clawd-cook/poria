use serde::{Deserialize, Serialize};

/// Credentials for JACP API access (cookie-based SSO).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JacpCredentials {
    pub cookie: String,
    pub username: String,
}

/// Card attachment from a Xingyun demand card.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardAttachment {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag_id: Option<i64>,
    #[serde(rename = "tagName")]
    pub tag_name: String,
    pub name: String,
    pub url: String,
}

/// User value object for demand detail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserVO {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub erp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "orgId", skip_serializing_if = "Option::is_none")]
    pub org_id: Option<String>,
    #[serde(rename = "orgName", skip_serializing_if = "Option::is_none")]
    pub org_name: Option<String>,
}

/// Full demand detail from the JACP API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DemandDetail {
    pub id: i64,
    #[serde(rename = "demandCode", default)]
    pub demand_code: String,
    #[serde(default)]
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<i32>,
    #[serde(rename = "demandDesc", skip_serializing_if = "Option::is_none")]
    pub demand_desc: Option<String>,
    #[serde(rename = "demandDescLink", skip_serializing_if = "Option::is_none")]
    pub demand_desc_link: Option<String>,
    #[serde(rename = "projectId", skip_serializing_if = "Option::is_none")]
    pub project_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processor: Option<UserVO>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proposer: Option<UserVO>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receiver: Option<UserVO>,
}

/// Query for related-to-me demand list.
///
/// Default (`accepted_by_me` false) omits `receiver` so JACP returns demands
/// related to the current ERP via cookie / `optErp`. When true, `receiver` is
/// the current ERP (assigned-to-me / 由我受理).
#[derive(Debug, Clone, Default)]
pub struct DemandListQuery {
    pub accepted_by_me: bool,
    pub current: i64,
    pub keyword: Option<String>,
    pub page_size: i64,
}

impl DemandListQuery {
    pub fn normalized(self) -> Self {
        let keyword = self.keyword.and_then(|value| {
            let trimmed = value.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        });
        Self {
            accepted_by_me: self.accepted_by_me,
            current: if self.current > 0 { self.current } else { 1 },
            keyword,
            page_size: if self.page_size > 0 {
                self.page_size
            } else {
                20
            },
        }
    }
}

/// One row in the related-to-me demand table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DemandListItem {
    pub id: i64,
    pub demand_code: String,
    pub name: String,
    pub status: Option<i32>,
    pub status_label: String,
    pub receiver_erp: Option<String>,
    pub receiver_name: Option<String>,
}

/// Paginated related-to-me demand list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DemandPage {
    pub records: Vec<DemandListItem>,
    pub total: i64,
    pub current: i64,
    pub page_size: i64,
}

/// Best-effort JoySpace PRD prefill for the start-pipeline wizard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DemandPrdPreview {
    pub demand_id: i64,
    pub demand_code: String,
    pub demand_name: String,
    pub url: Option<String>,
}

/// Result of a demand action (communicate / accept).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DemandActionResult {
    #[serde(rename = "demandId", skip_serializing_if = "Option::is_none")]
    pub demand_id: Option<i64>,
    #[serde(rename = "demandStatusCode", skip_serializing_if = "Option::is_none")]
    pub demand_status_code: Option<i32>,
    #[serde(rename = "taskOwner", skip_serializing_if = "Option::is_none")]
    pub task_owner: Option<String>,
    #[serde(rename = "taskId", skip_serializing_if = "Option::is_none")]
    pub task_id: Option<i64>,
    #[serde(rename = "taskName", skip_serializing_if = "Option::is_none")]
    pub task_name: Option<String>,
}

/// Action discriminator for XingyunChannelInput.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum XingyunAction {
    GetDemand,
    ListDemands,
    ListCardAttachments,
    ResolvePrdLink,
    BindBranch,
    Communicate,
    Accept,
}

/// Input for the Xingyun channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XingyunChannelInput {
    pub action: XingyunAction,
    #[serde(rename = "demandId", skip_serializing_if = "Option::is_none")]
    pub demand_id: Option<i64>,
    #[serde(rename = "demandCode", skip_serializing_if = "Option::is_none")]
    pub demand_code: Option<String>,
    #[serde(rename = "gitUrl", skip_serializing_if = "Option::is_none")]
    pub git_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(rename = "baseBranch", skip_serializing_if = "Option::is_none")]
    pub base_branch: Option<String>,
    #[serde(rename = "createLocal", skip_serializing_if = "Option::is_none")]
    pub create_local: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keyword: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current: Option<i64>,
    #[serde(rename = "pageSize", skip_serializing_if = "Option::is_none")]
    pub page_size: Option<i64>,
    #[serde(rename = "acceptedByMe", skip_serializing_if = "Option::is_none")]
    pub accepted_by_me: Option<bool>,
}

/// Output from the Xingyun channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
pub enum XingyunChannelOutput {
    #[serde(rename = "getDemand")]
    GetDemand { demand: Box<DemandDetail> },
    #[serde(rename = "listDemands")]
    ListDemands {
        records: Vec<DemandListItem>,
        total: i64,
        current: i64,
        page_size: i64,
    },
    #[serde(rename = "listCardAttachments")]
    ListCardAttachments { attachments: Vec<CardAttachment> },
    #[serde(rename = "resolvePrdLink")]
    ResolvePrdLink {
        url: String,
        attachment: CardAttachment,
    },
    #[serde(rename = "bindBranch")]
    BindBranch {
        branch: String,
        #[serde(rename = "changeId")]
        change_id: String,
        #[serde(rename = "baseBranch")]
        base_branch: String,
    },
    #[serde(rename = "communicate")]
    Communicate {
        #[serde(rename = "demandId")]
        demand_id: i64,
        #[serde(rename = "demandStatusCode", skip_serializing_if = "Option::is_none")]
        demand_status_code: Option<i32>,
    },
    #[serde(rename = "accept")]
    Accept {
        #[serde(rename = "demandId")]
        demand_id: i64,
        #[serde(rename = "demandStatusCode", skip_serializing_if = "Option::is_none")]
        demand_status_code: Option<i32>,
    },
}
