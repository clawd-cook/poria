use serde::{Deserialize, Serialize};

/// Credentials for JACP API access (cookie-based SSO).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JacpCredentials {
    pub cookie: String,
    pub username: String,
}

/// Coding repository info from EasyCI GQL API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodingRepo {
    pub code: String,
    #[serde(rename = "defaultBranchName", skip_serializing_if = "Option::is_none")]
    pub default_branch_name: Option<String>,
    #[serde(rename = "gitUrl")]
    pub git_url: String,
    #[serde(rename = "homeUrl", skip_serializing_if = "Option::is_none")]
    pub home_url: Option<String>,
    #[serde(rename = "repoLabel", skip_serializing_if = "Option::is_none")]
    pub repo_label: Option<String>,
}

/// Branch info from EasyCI GQL API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodingBranch {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

/// MR status from GitLab API.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MrStatus {
    Opened,
    Closed,
    Merged,
    Locked,
}

impl MrStatus {
    pub fn parse(s: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        match s.trim() {
            "opened" => Ok(MrStatus::Opened),
            "closed" => Ok(MrStatus::Closed),
            "merged" => Ok(MrStatus::Merged),
            "locked" => Ok(MrStatus::Locked),
            other => Err(format!("Unknown MR state: {}", other).into()),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            MrStatus::Opened => "opened",
            MrStatus::Closed => "closed",
            MrStatus::Merged => "merged",
            MrStatus::Locked => "locked",
        }
    }
}

/// MR info returned from findMr.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MrInfo {
    pub url: String,
    pub iid: i32,
    #[serde(rename = "sourceBranch")]
    pub source_branch: String,
    #[serde(rename = "targetBranch")]
    pub target_branch: String,
    pub state: MrStatus,
}

/// Query parameters for finding an existing MR.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindMrQuery {
    #[serde(rename = "projectPath")]
    pub project_path: String,
    #[serde(rename = "sourceBranch")]
    pub source_branch: String,
    #[serde(rename = "targetBranch")]
    pub target_branch: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<MrStatus>,
}

/// MR creation input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMrInput {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "sourceBranch", skip_serializing_if = "Option::is_none")]
    pub source_branch: Option<String>,
    #[serde(rename = "targetBranch", skip_serializing_if = "Option::is_none")]
    pub target_branch: Option<String>,
    #[serde(rename = "projectId", skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
}

/// MR creation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMrResult {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iid: Option<i32>,
    #[serde(rename = "sourceBranch")]
    pub source_branch: String,
    #[serde(rename = "targetBranch")]
    pub target_branch: String,
}

/// Action discriminator for CodingChannelInput.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CodingAction {
    SearchRepos,
    ListBranches,
    CreateMergeRequest,
    GetMrStatus,
    FindMr,
}

/// Input for the Coding channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodingChannelInput {
    pub action: CodingAction,
    #[serde(rename = "searchRepos", skip_serializing_if = "Option::is_none")]
    pub search_repos: Option<SearchReposInput>,
    #[serde(rename = "listBranches", skip_serializing_if = "Option::is_none")]
    pub list_branches: Option<ListBranchesInput>,
    #[serde(rename = "createMergeRequest", skip_serializing_if = "Option::is_none")]
    pub create_merge_request: Option<CreateMrInput>,
    #[serde(rename = "getMrStatus", skip_serializing_if = "Option::is_none")]
    pub get_mr_status: Option<GetMrStatusInput>,
    #[serde(rename = "findMr", skip_serializing_if = "Option::is_none")]
    pub find_mr: Option<FindMrQuery>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchReposInput {
    #[serde(rename = "nameLike", skip_serializing_if = "Option::is_none")]
    pub name_like: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListBranchesInput {
    #[serde(rename = "gitUrl")]
    pub git_url: String,
    #[serde(rename = "nameLike", skip_serializing_if = "Option::is_none")]
    pub name_like: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetMrStatusInput {
    #[serde(rename = "projectPath")]
    pub project_path: String,
    pub iid: i32,
}

/// Output from the Coding channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
pub enum CodingChannelOutput {
    #[serde(rename = "searchRepos")]
    SearchRepos { repos: Vec<CodingRepo> },
    #[serde(rename = "listBranches")]
    ListBranches { branches: Vec<CodingBranch> },
    #[serde(rename = "createMergeRequest")]
    CreateMergeRequest { result: CreateMrResult },
    #[serde(rename = "getMrStatus")]
    GetMrStatus { status: MrStatus },
    #[serde(rename = "findMr")]
    FindMr { mr: Option<MrInfo> },
}
