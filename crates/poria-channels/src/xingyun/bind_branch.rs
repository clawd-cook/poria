use super::types::JacpCredentials;
use super::{get_demand_by_id, jacp_fetch, same_git_url, urlencoding};
use crate::coding::{
    create_change, pick_bind_apps, query_all_repos, query_bind_deploy_apps, query_issue_changes,
    repo_name_from_git_url, BoundChange, CreateChangeInput, JacpCredentials as CodingCredentials,
};

/// Input for associating a local feature branch with a Xingyun card.
pub struct BindBranchInput {
    pub demand_id: Option<i64>,
    pub demand_code: Option<String>,
    pub git_url: String,
    pub branch: String,
    pub base_branch: Option<String>,
}

/// Result of a successful bind (new or reused).
#[derive(Debug, Clone)]
pub struct BindBranchResult {
    pub branch: String,
    pub change_id: String,
    pub base_branch: String,
    pub already_bound: bool,
    pub repo_code: String,
}

pub(crate) fn json_i64(value: Option<&serde_json::Value>) -> Option<i64> {
    let value = value?;
    if let Some(n) = value.as_i64() {
        return Some(n);
    }
    if let Some(n) = value.as_u64() {
        return i64::try_from(n).ok();
    }
    value.as_str()?.trim().parse().ok()
}

pub(crate) fn card_space_id(card: &serde_json::Value) -> Option<i64> {
    let obj = card.as_object()?;
    if let Some(nested) = obj.get("space").and_then(|v| v.as_object()) {
        if let Some(id) = json_i64(nested.get("id")) {
            return Some(id);
        }
    }
    json_i64(obj.get("spaceId"))
}

pub(crate) fn space_key(space: &serde_json::Value) -> Option<String> {
    space
        .as_object()
        .and_then(|obj| obj.get("key"))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
}

pub(crate) fn matching_change<'a>(
    changes: &'a [BoundChange],
    git_url: &str,
    branch: &str,
) -> Option<&'a BoundChange> {
    changes.iter().find(|change| {
        if change.branch != branch {
            return false;
        }
        match change.git_url.as_deref().map(str::trim) {
            None | Some("") => true,
            Some(url) => same_git_url(Some(url), Some(git_url)),
        }
    })
}

fn coding_creds(credentials: &JacpCredentials) -> CodingCredentials {
    CodingCredentials {
        cookie: credentials.cookie.clone(),
        username: credentials.username.clone(),
    }
}

async fn fetch_card(
    credentials: &JacpCredentials,
    demand_code: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    let path = format!(
        "/openapi/v3/cards/code/{}",
        urlencoding::encode(demand_code)
    );
    jacp_fetch(
        credentials,
        &path,
        reqwest::Method::GET,
        None,
        "空间卡片详情查询失败",
    )
    .await
}

async fn fetch_space_key(
    credentials: &JacpCredentials,
    space_id: i64,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let path = format!("/openapi/v3/spaces/{space_id}");
    let data = jacp_fetch(
        credentials,
        &path,
        reqwest::Method::GET,
        None,
        "空间详情查询失败",
    )
    .await?;
    space_key(&data).ok_or_else(|| format!("空间 {space_id} 缺少 key").into())
}

async fn resolve_repo_code(
    credentials: &CodingCredentials,
    git_url: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let name = repo_name_from_git_url(git_url).unwrap_or_default();
    if name.is_empty() {
        return Err("无法从 gitUrl 解析仓库名".into());
    }
    let repos = query_all_repos(credentials, &name, 20).await?;
    repos
        .into_iter()
        .find(|repo| same_git_url(Some(repo.git_url.as_str()), Some(git_url)))
        .map(|repo| repo.code)
        .ok_or_else(|| format!("未在 EasyCI 找到仓库: {git_url}").into())
}

/// Bind an existing remote `branch` to the Xingyun card via EasyCI `createChange`.
///
/// Callers must `git push` first: `branchOperateType` only accepts `SELECT`, and
/// SELECT fails if EasyCI cannot find the remote branch. Idempotent when the
/// issue already has this repo+branch.
pub async fn bind_branch(
    credentials: &JacpCredentials,
    input: BindBranchInput,
) -> Result<BindBranchResult, Box<dyn std::error::Error + Send + Sync>> {
    let git_url = input.git_url.trim().to_string();
    let branch = input.branch.trim().to_string();
    if git_url.is_empty() {
        return Err("bindBranch requires gitUrl".into());
    }
    if branch.is_empty() {
        return Err("bindBranch requires branch".into());
    }

    let mut demand_code = input
        .demand_code
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let mut fallback_space_id = None;
    if demand_code.is_none() || fallback_space_id.is_none() {
        if let Some(demand_id) = input.demand_id {
            let demand = get_demand_by_id(credentials, demand_id).await?;
            if demand_code.is_none() {
                let code = demand.demand_code.trim();
                if !code.is_empty() {
                    demand_code = Some(code.to_string());
                }
            }
            fallback_space_id = demand.project_id;
        }
    }
    let demand_code = demand_code.ok_or("bindBranch requires demandCode or demandId")?;

    let space_id = match fetch_card(credentials, &demand_code).await {
        Ok(card) => card_space_id(&card).or(fallback_space_id),
        Err(_) => fallback_space_id,
    }
    .ok_or("无法解析行云空间 ID")?;
    let space_key = fetch_space_key(credentials, space_id).await?;

    let coding = coding_creds(credentials);
    let repo_code = resolve_repo_code(&coding, &git_url).await?;
    let existing = query_issue_changes(&coding, &space_key, &demand_code).await?;
    if let Some(found) = matching_change(&existing, &git_url, &branch) {
        let change_id = found.id.clone().ok_or("已关联分支缺少 changeId")?;
        return Ok(BindBranchResult {
            branch,
            change_id,
            base_branch: input.base_branch.unwrap_or_default(),
            already_bound: true,
            repo_code,
        });
    }

    let apps = query_bind_deploy_apps(&coding, &space_key, &repo_code).await?;
    let bind_apps = pick_bind_apps(&apps.records);
    if bind_apps.is_empty() {
        return Err(format!("仓库 {repo_code} 未绑定部署应用，无法关联行云").into());
    }
    let dev_space_id = apps
        .dev_space_id
        .filter(|id| !id.is_empty())
        .ok_or("缺少 EasyCI 空间 ID")?;

    let change_id = match create_change(
        &coding,
        &CreateChangeInput {
            dev_space_id,
            name: Some(String::new()),
            code: repo_code.clone(),
            issue_code: demand_code.clone(),
            branch: branch.clone(),
            branch_operate_type: "SELECT",
            bind_apps,
        },
    )
    .await
    {
        Ok(id) => id,
        Err(err) => {
            let retried = query_issue_changes(&coding, &space_key, &demand_code).await?;
            matching_change(&retried, &git_url, &branch)
                .and_then(|change| change.id.clone())
                .ok_or(err)?
        }
    };

    Ok(BindBranchResult {
        branch,
        change_id,
        base_branch: input.base_branch.unwrap_or_default(),
        already_bound: false,
        repo_code,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coding::BoundChange;

    #[test]
    fn card_space_id_from_nested_space() {
        let card = serde_json::json!({ "space": { "id": 99 }, "spaceId": 1 });
        assert_eq!(card_space_id(&card), Some(99));
    }

    #[test]
    fn card_space_id_from_space_id() {
        let card = serde_json::json!({ "spaceId": "17" });
        assert_eq!(card_space_id(&card), Some(17));
    }

    #[test]
    fn space_key_reads_key_field() {
        assert_eq!(
            space_key(&serde_json::json!({ "key": " o2o ", "id": 1 })),
            Some("o2o".into())
        );
        assert_eq!(space_key(&serde_json::json!({ "id": 1 })), None);
    }

    #[test]
    fn matching_change_requires_same_branch() {
        let changes = vec![BoundChange {
            id: Some("chg-1".into()),
            branch: "feature_R1".into(),
            git_url: Some("git@coding.jd.com:group/repo.git".into()),
            home_url: None,
            repo_code: None,
        }];
        assert!(
            matching_change(&changes, "https://coding.jd.com/group/repo", "feature_R1").is_some()
        );
        assert!(matching_change(&changes, "https://coding.jd.com/group/repo", "other").is_none());
    }
}
