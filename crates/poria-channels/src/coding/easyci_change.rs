use super::{easyci_gql, JacpCredentials};

const QUERY_ISSUE_CHANGES: &str = r#"query queryIssue($key: String!, $code: String!) {
  devSpace(key: $key) {
    issueSpace {
      issue(code: $code) {
        changes {
          id
          branch
          repository {
            code
            gitUrl
            homeUrl
            __typename
          }
          __typename
        }
        __typename
      }
      __typename
    }
    __typename
  }
}"#;

const QUERY_BIND_DEPLOY_APPS: &str = r#"query queryBindDeployApps($key: String!, $repoCode: String!, $withBindApps: Boolean = true) {
  devSpace(key: $key) {
    id
    alreadyBindDeployApps(query: {code: $repoCode}) {
      name
      appKey
      systemId
      gitUrl
      deploySystem {
        id
        name
        env
        systemKey
        __typename
      }
      __typename
    }
    bindApps(query: {code: $repoCode, page: 1, size: 200}) @include(if: $withBindApps) {
      totalCount
      bindApps {
        app {
          name
          appKey
          systemId
          gitUrl
          deploySystem {
            id
            name
            env
            systemKey
            __typename
          }
          __typename
        }
        __typename
      }
      __typename
    }
    __typename
  }
}"#;

const MUTATION_CREATE_CHANGE: &str = r#"mutation createChange($input: CreateChangeInput!) {
  createChange(input: $input) {
    id
    __typename
  }
}"#;

/// Bound EasyCI change already attached to a Xingyun issue.
#[derive(Debug, Clone)]
pub struct BoundChange {
    pub id: Option<String>,
    pub branch: String,
    pub git_url: Option<String>,
    pub home_url: Option<String>,
    pub repo_code: Option<String>,
}

/// Deploy app that can be bound when creating a change.
#[derive(Debug, Clone)]
pub struct EasyciDeployApp {
    pub app_key: String,
    pub system_id: Option<String>,
    pub already_bound: bool,
}

/// Result of `queryBindDeployApps`.
#[derive(Debug, Clone)]
pub struct BindDeployAppsResult {
    pub dev_space_id: Option<String>,
    pub records: Vec<EasyciDeployApp>,
}

/// App pair required by `createChange`.
#[derive(Debug, Clone)]
pub struct BindApp {
    pub system_id: String,
    pub app_key: String,
}

/// Input for EasyCI `createChange`.
#[derive(Debug, Clone)]
pub struct CreateChangeInput {
    pub dev_space_id: String,
    pub name: Option<String>,
    pub code: String,
    pub issue_code: String,
    pub branch: String,
    pub branch_operate_type: &'static str,
    pub bind_apps: Vec<BindApp>,
}

pub(crate) fn json_text(value: Option<&serde_json::Value>) -> Option<String> {
    let value = value?;
    if let Some(text) = value.as_str() {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return None;
        }
        return Some(trimmed.to_string());
    }
    if let Some(n) = value.as_i64() {
        return Some(n.to_string());
    }
    if let Some(n) = value.as_u64() {
        return Some(n.to_string());
    }
    None
}

fn as_list(value: Option<&serde_json::Value>) -> Vec<serde_json::Value> {
    match value {
        Some(serde_json::Value::Array(items)) => items.clone(),
        Some(obj) if obj.is_object() => {
            for key in ["records", "list", "bindApps", "changes"] {
                if let Some(serde_json::Value::Array(items)) = obj.get(key) {
                    return items.clone();
                }
            }
            Vec::new()
        }
        _ => Vec::new(),
    }
}

fn as_bound_change(value: &serde_json::Value) -> Option<BoundChange> {
    let obj = value.as_object()?;
    let branch = json_text(obj.get("branch"))?;
    let repository = obj.get("repository").and_then(|v| v.as_object());
    Some(BoundChange {
        id: json_text(obj.get("id")),
        branch,
        git_url: repository.and_then(|repo| json_text(repo.get("gitUrl"))),
        home_url: repository.and_then(|repo| json_text(repo.get("homeUrl"))),
        repo_code: repository.and_then(|repo| json_text(repo.get("code"))),
    })
}

fn as_deploy_app(value: &serde_json::Value, already_bound: bool) -> Option<EasyciDeployApp> {
    let obj = value.as_object()?;
    let app_key = json_text(obj.get("appKey"))?;
    let deploy_system = obj.get("deploySystem").and_then(|v| v.as_object());
    let system_id = json_text(obj.get("systemId"))
        .or_else(|| deploy_system.and_then(|sys| json_text(sys.get("id"))));
    Some(EasyciDeployApp {
        app_key,
        system_id,
        already_bound,
    })
}

/// Prefer already-bound apps; otherwise all apps that have both systemId and appKey.
pub fn pick_bind_apps(records: &[EasyciDeployApp]) -> Vec<BindApp> {
    let bound: Vec<BindApp> = records
        .iter()
        .filter(|app| app.already_bound)
        .filter_map(bind_app_from)
        .collect();
    if !bound.is_empty() {
        return bound;
    }
    records.iter().filter_map(bind_app_from).collect()
}

fn bind_app_from(app: &EasyciDeployApp) -> Option<BindApp> {
    let system_id = app.system_id.as_deref()?.trim();
    if system_id.is_empty() {
        return None;
    }
    Some(BindApp {
        system_id: system_id.to_string(),
        app_key: app.app_key.clone(),
    })
}

/// Existing EasyCI changes for a Xingyun card.
pub async fn query_issue_changes(
    credentials: &JacpCredentials,
    space_key: &str,
    issue_code: &str,
) -> Result<Vec<BoundChange>, Box<dyn std::error::Error + Send + Sync>> {
    let key = space_key.trim();
    let code = issue_code.trim();
    if key.is_empty() {
        return Err("缺少空间 key".into());
    }
    if code.is_empty() {
        return Err("缺少卡片编码".into());
    }

    let data: serde_json::Value = easyci_gql(
        credentials,
        "绑定分支查询失败",
        "queryIssue",
        QUERY_ISSUE_CHANGES,
        serde_json::json!({
            "key": key,
            "code": code,
        }),
    )
    .await?;

    let changes = data
        .get("devSpace")
        .and_then(|v| v.get("issueSpace"))
        .and_then(|v| v.get("issue"))
        .and_then(|v| v.get("changes"));
    Ok(as_list(changes)
        .iter()
        .filter_map(as_bound_change)
        .collect())
}

/// Apps that can be bound when creating a change for `repo_code`.
pub async fn query_bind_deploy_apps(
    credentials: &JacpCredentials,
    space_key: &str,
    repo_code: &str,
) -> Result<BindDeployAppsResult, Box<dyn std::error::Error + Send + Sync>> {
    let key = space_key.trim();
    let code = repo_code.trim();
    if key.is_empty() {
        return Err("缺少空间 key".into());
    }
    if code.is_empty() {
        return Err("缺少仓库编码".into());
    }

    let data: serde_json::Value = easyci_gql(
        credentials,
        "绑定应用查询失败",
        "queryBindDeployApps",
        QUERY_BIND_DEPLOY_APPS,
        serde_json::json!({
            "withBindApps": true,
            "key": key,
            "repoCode": code,
        }),
    )
    .await?;

    let dev_space = data.get("devSpace");
    let mut merged: Vec<EasyciDeployApp> = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for app in as_list(dev_space.and_then(|v| v.get("alreadyBindDeployApps")))
        .iter()
        .filter_map(|item| as_deploy_app(item, true))
    {
        let id = format!(
            "{}::{}",
            app.app_key,
            app.system_id.as_deref().unwrap_or("")
        );
        if seen.insert(id) {
            merged.push(app);
        }
    }
    for row in as_list(
        dev_space
            .and_then(|v| v.get("bindApps"))
            .and_then(|v| v.get("bindApps")),
    ) {
        let nested = row.get("app").unwrap_or(&row);
        if let Some(app) = as_deploy_app(nested, false) {
            let id = format!(
                "{}::{}",
                app.app_key,
                app.system_id.as_deref().unwrap_or("")
            );
            if seen.insert(id) {
                merged.push(app);
            }
        }
    }

    Ok(BindDeployAppsResult {
        dev_space_id: json_text(dev_space.and_then(|v| v.get("id"))),
        records: merged,
    })
}

/// Create an EasyCI change that associates `branch` with the Xingyun issue.
pub async fn create_change(
    credentials: &JacpCredentials,
    input: &CreateChangeInput,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let bind_apps: Vec<serde_json::Value> = input
        .bind_apps
        .iter()
        .map(|app| {
            serde_json::json!({
                "systemId": app.system_id,
                "appKey": app.app_key,
            })
        })
        .collect();
    let variables = serde_json::json!({
        "input": {
            "devSpaceId": input.dev_space_id,
            "name": input.name.clone().unwrap_or_default(),
            "code": input.code,
            "issueCode": input.issue_code,
            "branch": input.branch,
            "branchOperateType": input.branch_operate_type,
            "bindApps": bind_apps,
        }
    });

    let data: serde_json::Value = easyci_gql(
        credentials,
        "关联分支失败",
        "createChange",
        MUTATION_CREATE_CHANGE,
        variables,
    )
    .await?;
    json_text(data.get("createChange").and_then(|v| v.get("id")))
        .ok_or_else(|| "关联分支失败：未返回变更 ID".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pick_bind_apps_prefers_already_bound() {
        let records = vec![
            EasyciDeployApp {
                app_key: "other".into(),
                system_id: Some("sys-2".into()),
                already_bound: false,
            },
            EasyciDeployApp {
                app_key: "bound".into(),
                system_id: Some("sys-1".into()),
                already_bound: true,
            },
        ];
        let picked = pick_bind_apps(&records);
        assert_eq!(picked.len(), 1);
        assert_eq!(picked[0].app_key, "bound");
        assert_eq!(picked[0].system_id, "sys-1");
    }

    #[test]
    fn pick_bind_apps_falls_back_to_all_with_system_id() {
        let records = vec![
            EasyciDeployApp {
                app_key: "missing".into(),
                system_id: None,
                already_bound: false,
            },
            EasyciDeployApp {
                app_key: "app-a".into(),
                system_id: Some("sys-a".into()),
                already_bound: false,
            },
        ];
        let picked = pick_bind_apps(&records);
        assert_eq!(picked.len(), 1);
        assert_eq!(picked[0].app_key, "app-a");
    }

    #[test]
    fn json_text_accepts_number_ids() {
        assert_eq!(json_text(Some(&serde_json::json!(42))), Some("42".into()));
        assert_eq!(
            json_text(Some(&serde_json::json!("  abc  "))),
            Some("abc".into())
        );
        assert_eq!(json_text(Some(&serde_json::json!(""))), None);
    }
}
