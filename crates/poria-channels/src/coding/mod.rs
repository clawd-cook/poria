mod easyci_change;
mod git_url;
mod types;

pub use easyci_change::{
    create_change, pick_bind_apps, query_bind_deploy_apps, query_issue_changes, BindApp,
    BindDeployAppsResult, BoundChange, CreateChangeInput, EasyciDeployApp,
};
pub use git_url::{
    normalize_git_url, repo_name_from_git_url, repo_scope_and_name_from_git_url,
    repo_search_path_from_git_url, same_git_url,
};
pub use types::*;

use async_trait::async_trait;
use poria_core::contracts::{CapabilityMetadata, Channel, ChannelContext};

/// Creates a new Coding channel instance.
pub fn create_coding_channel() -> CodingChannel {
    CodingChannel {
        metadata: CapabilityMetadata {
            id: "channel:coding".into(),
            name: "Coding".into(),
            description: "EasyCI repository, branch, and MR operations".into(),
            version: "0.1.0".into(),
        },
    }
}

pub struct CodingChannel {
    metadata: CapabilityMetadata,
}

#[async_trait]
impl Channel for CodingChannel {
    fn metadata(&self) -> &CapabilityMetadata {
        &self.metadata
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        ctx: ChannelContext,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let input: CodingChannelInput = serde_json::from_value(input)?;
        let credentials = require_credentials(&ctx)?;

        let output = match input.action {
            CodingAction::SearchRepos => {
                let query = input.search_repos.unwrap_or_default();
                let repos = query_all_repos(
                    &credentials,
                    &query.name_like.unwrap_or_default(),
                    query.size.unwrap_or(20),
                )
                .await?;
                CodingChannelOutput::SearchRepos { repos }
            }
            CodingAction::ListBranches => {
                let params = input
                    .list_branches
                    .ok_or("listBranches requires listBranches input")?;
                let branches = query_branches(
                    &credentials,
                    &params.git_url,
                    &params.name_like.unwrap_or_default(),
                )
                .await?;
                CodingChannelOutput::ListBranches { branches }
            }
            CodingAction::CreateMergeRequest => {
                let params = input
                    .create_merge_request
                    .ok_or("createMergeRequest requires createMergeRequest input")?;
                let result = create_merge_request_live(&credentials, &params).await?;
                CodingChannelOutput::CreateMergeRequest { result }
            }
            CodingAction::GetMrStatus => {
                let params = input
                    .get_mr_status
                    .ok_or("getMrStatus requires getMrStatus input")?;
                let status =
                    get_mr_status_live(&credentials, &params.project_path, params.iid).await?;
                CodingChannelOutput::GetMrStatus { status }
            }
            CodingAction::FindMr => {
                let query = input.find_mr.ok_or("findMr requires findMr input")?;
                let mr = find_mr_live(&credentials, &query).await?;
                CodingChannelOutput::FindMr { mr }
            }
        };

        Ok(serde_json::to_value(output)?)
    }
}

fn require_credentials(
    ctx: &ChannelContext,
) -> Result<JacpCredentials, Box<dyn std::error::Error + Send + Sync>> {
    let creds: JacpCredentials = serde_json::from_value(ctx.credentials.clone())?;
    if creds.cookie.is_empty() {
        return Err("AuthRequired: please run poria auth login".into());
    }
    Ok(creds)
}

// ---------------------------------------------------------------------------
// Coding base URL
// ---------------------------------------------------------------------------

const DEFAULT_CODING_BASE: &str = "https://coding.jd.com";

fn coding_base_url() -> String {
    std::env::var("PORIA_CODING_BASE_URL")
        .ok()
        .and_then(|s| {
            let trimmed = s.trim().trim_end_matches('/').to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        })
        .unwrap_or_else(|| DEFAULT_CODING_BASE.to_string())
}

// ---------------------------------------------------------------------------
// EasyCI GraphQL
// ---------------------------------------------------------------------------

const DEFAULT_EASYCI_GQL_URL: &str = "https://easyci.jd.com/gql";

#[derive(serde::Deserialize)]
struct GqlResponse<T> {
    data: Option<T>,
    errors: Option<Vec<GqlError>>,
}

#[derive(serde::Deserialize)]
struct GqlError {
    message: Option<String>,
}

pub(crate) async fn easyci_gql<T: serde::de::DeserializeOwned>(
    credentials: &JacpCredentials,
    error_label: &str,
    operation_name: &str,
    query: &str,
    variables: serde_json::Value,
) -> Result<T, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "operationName": operation_name,
        "variables": variables,
        "query": query,
    });

    let response = client
        .post(DEFAULT_EASYCI_GQL_URL)
        .header("Content-Type", "application/json")
        .header("Cookie", &credentials.cookie)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("{}: {}", error_label, e))?;

    if response.status().as_u16() == 401 {
        return Err(poria_core::types::AUTH_EXPIRED_USER_MESSAGE.into());
    }
    if !response.status().is_success() {
        return Err(format!("{}: HTTP {}", error_label, response.status()).into());
    }

    let gql: GqlResponse<T> = response
        .json()
        .await
        .map_err(|e| format!("{}: {}", error_label, e))?;

    if let Some(errors) = &gql.errors {
        let msgs: Vec<String> = errors.iter().filter_map(|e| e.message.clone()).collect();
        if !msgs.is_empty() && gql.data.is_none() {
            return Err(format!("{}: {}", error_label, msgs.join("; ")).into());
        }
    }

    gql.data
        .ok_or_else(|| format!("{}: empty response", error_label).into())
}

// ---------------------------------------------------------------------------
// Repo queries
// ---------------------------------------------------------------------------

const QUERY_ALL_REPOS: &str = r#"query queryAllRepos($query: RepositoryQuery) {
  allRepositories(query: $query) {
    repositories {
      code
      defaultBranchName
      gitUrl
      homeUrl
      __typename
    }
    totalCount
    __typename
  }
}"#;

const QUERY_BRANCHES: &str = r#"query queryBranches($gitUrl: String!, $query: BranchQuery) {
  findRepository(gitUrl: $gitUrl) {
    branches(query: $query) {
      branches {
        name
        status
        __typename
      }
      totalCount
      __typename
    }
    __typename
  }
}"#;

#[derive(serde::Deserialize)]
struct AllReposData {
    #[serde(rename = "allRepositories")]
    all_repositories: Option<AllReposContainer>,
}

#[derive(serde::Deserialize)]
struct AllReposContainer {
    repositories: Option<Vec<serde_json::Value>>,
}

/// Query all repos from EasyCI.
pub async fn query_all_repos(
    credentials: &JacpCredentials,
    name_like: &str,
    size: i32,
) -> Result<Vec<CodingRepo>, Box<dyn std::error::Error + Send + Sync>> {
    let page_size = if size > 0 { size } else { 20 };
    let variables = serde_json::json!({
        "query": { "page": 1, "nameLike": name_like, "size": page_size }
    });

    let data: AllReposData = easyci_gql(
        credentials,
        "Repo search failed",
        "queryAllRepos",
        QUERY_ALL_REPOS,
        variables,
    )
    .await?;

    let repos = data
        .all_repositories
        .and_then(|c| c.repositories)
        .unwrap_or_default();

    Ok(repos
        .into_iter()
        .filter_map(|v| {
            let obj = v.as_object()?;
            let git_url = obj.get("gitUrl")?.as_str()?.trim().to_string();
            let code = obj.get("code")?.as_str()?.trim().to_string();
            if git_url.is_empty() || code.is_empty() {
                return None;
            }
            let home_url = obj
                .get("homeUrl")
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            let default_branch = obj
                .get("defaultBranchName")
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            let repo_label = home_url.clone().unwrap_or_else(|| git_url.clone());
            Some(CodingRepo {
                code,
                default_branch_name: default_branch,
                git_url,
                home_url,
                repo_label: Some(repo_label),
            })
        })
        .collect())
}

#[derive(serde::Deserialize)]
struct FindRepoData {
    #[serde(rename = "findRepository")]
    find_repository: Option<FindRepoBranches>,
}

#[derive(serde::Deserialize)]
struct FindRepoBranches {
    branches: Option<BranchesContainer>,
}

#[derive(serde::Deserialize)]
struct BranchesContainer {
    branches: Option<Vec<serde_json::Value>>,
}

/// Query branches for a repository from EasyCI.
pub async fn query_branches(
    credentials: &JacpCredentials,
    git_url: &str,
    name_like: &str,
) -> Result<Vec<CodingBranch>, Box<dyn std::error::Error + Send + Sync>> {
    let url = git_url.trim();
    if url.is_empty() {
        return Err("Missing repo URL".into());
    }
    let variables = serde_json::json!({
        "gitUrl": url,
        "query": { "page": 1, "nameLike": name_like, "size": 20 }
    });

    let data: FindRepoData = easyci_gql(
        credentials,
        "Branch search failed",
        "queryBranches",
        QUERY_BRANCHES,
        variables,
    )
    .await?;

    let branches = data
        .find_repository
        .and_then(|r| r.branches)
        .and_then(|b| b.branches)
        .unwrap_or_default();

    Ok(branches
        .into_iter()
        .filter_map(|v| {
            let obj = v.as_object()?;
            let name = obj.get("name")?.as_str()?.trim().to_string();
            if name.is_empty() {
                return None;
            }
            let status = obj
                .get("status")
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            Some(CodingBranch { name, status })
        })
        .collect())
}

// ---------------------------------------------------------------------------
// Merge request operations (GitLab API)
// ---------------------------------------------------------------------------

/// Extract project ID (path) from a git URL.
pub fn project_id_from_git_url(git_url: &str) -> Option<String> {
    repo_search_path_from_git_url(git_url)
}

/// Create a merge request via the Coding (GitLab) API.
pub async fn create_merge_request_live(
    credentials: &JacpCredentials,
    input: &CreateMrInput,
) -> Result<CreateMrResult, Box<dyn std::error::Error + Send + Sync>> {
    let base = coding_base_url();
    let project_id = input
        .project_id
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .ok_or("Missing project ID for merge request")?;

    let source_branch = input
        .source_branch
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .ok_or("Missing source branch")?;

    let target_branch = input
        .target_branch
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("master");

    let encoded_project = urlencoding_encode(project_id);
    let url = format!(
        "{}/api/v4/projects/{}/merge_requests",
        base, encoded_project
    );

    let body = serde_json::json!({
        "source_branch": source_branch,
        "target_branch": target_branch,
        "title": input.title,
        "description": input.description.as_deref().unwrap_or(""),
    });

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Cookie", &credentials.cookie)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Create merge request failed: {}", e))?;

    if response.status().as_u16() == 401 {
        return Err(poria_core::types::AUTH_EXPIRED_USER_MESSAGE.into());
    }
    if !response.status().is_success() {
        let status = response.status();
        let body_text = response.text().await.unwrap_or_default();
        return Err(format!(
            "Create merge request failed: HTTP {} {}",
            status,
            &body_text[..body_text.len().min(400)]
        )
        .into());
    }

    let data: serde_json::Value = response.json().await?;
    let web_url = data
        .get("web_url")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or("Create merge request failed: empty web_url")?;

    Ok(CreateMrResult {
        url: web_url,
        iid: data.get("iid").and_then(|v| v.as_i64()).map(|v| v as i32),
        source_branch: source_branch.to_string(),
        target_branch: target_branch.to_string(),
    })
}

/// Get MR status from GitLab API.
pub async fn get_mr_status_live(
    credentials: &JacpCredentials,
    project_path: &str,
    iid: i32,
) -> Result<MrStatus, Box<dyn std::error::Error + Send + Sync>> {
    let base = coding_base_url();
    let encoded_path = urlencoding_encode(project_path);
    let url = format!(
        "{}/api/v4/projects/{}/merge_requests/{}",
        base, encoded_path, iid
    );

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("Cookie", &credentials.cookie)
        .send()
        .await
        .map_err(|e| format!("Get MR status failed: {}", e))?;

    if response.status().as_u16() == 401 {
        return Err(poria_core::types::AUTH_EXPIRED_USER_MESSAGE.into());
    }
    if response.status().as_u16() == 404 {
        return Err(format!("MR not found: project={} iid={}", project_path, iid).into());
    }
    if !response.status().is_success() {
        return Err(format!("Get MR status failed: HTTP {}", response.status()).into());
    }

    let data: serde_json::Value = response.json().await?;
    let state = data
        .get("state")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();

    MrStatus::parse(state)
}

/// Find an existing MR matching the query.
pub async fn find_mr_live(
    credentials: &JacpCredentials,
    query: &FindMrQuery,
) -> Result<Option<MrInfo>, Box<dyn std::error::Error + Send + Sync>> {
    let base = coding_base_url();
    let encoded_path = urlencoding_encode(&query.project_path);
    let state_str = query.state.as_ref().map(|s| s.as_str()).unwrap_or("opened");

    let url = format!(
        "{}/api/v4/projects/{}/merge_requests?source_branch={}&target_branch={}&state={}",
        base,
        encoded_path,
        urlencoding_encode(&query.source_branch),
        urlencoding_encode(&query.target_branch),
        state_str,
    );

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("Cookie", &credentials.cookie)
        .send()
        .await
        .map_err(|e| format!("Find MR failed: {}", e))?;

    if response.status().as_u16() == 401 {
        return Err(poria_core::types::AUTH_EXPIRED_USER_MESSAGE.into());
    }
    if !response.status().is_success() {
        return Err(format!("Find MR failed: HTTP {}", response.status()).into());
    }

    let data: Vec<serde_json::Value> = response.json().await?;
    if data.is_empty() {
        return Ok(None);
    }

    let first = &data[0];
    let web_url = first
        .get("web_url")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let iid = first.get("iid").and_then(|v| v.as_i64()).map(|v| v as i32);

    match (web_url, iid) {
        (Some(url), Some(iid)) => {
            let state_raw = first
                .get("state")
                .and_then(|v| v.as_str())
                .unwrap_or("opened");
            let state = MrStatus::parse(state_raw).unwrap_or(MrStatus::Opened);
            Ok(Some(MrInfo {
                url,
                iid,
                source_branch: first
                    .get("source_branch")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&query.source_branch)
                    .to_string(),
                target_branch: first
                    .get("target_branch")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&query.target_branch)
                    .to_string(),
                state,
            }))
        }
        _ => Ok(None),
    }
}

fn urlencoding_encode(input: &str) -> String {
    let mut result = String::new();
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            _ => {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    result
}
