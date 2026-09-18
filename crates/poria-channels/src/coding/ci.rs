use super::{coding_base_url, urlencoding_encode, JacpCredentials};
use poria_core::pipeline::{parse_gitlab_commit_statuses, parse_gitlab_pipelines, CiBuildReport};
use poria_core::types::AUTH_EXPIRED_USER_MESSAGE;

/// Query Coding/GitLab CI for an MR. Missing or non-success is not a pass.
pub async fn query_mr_ci_status(
    credentials: &JacpCredentials,
    project_path: &str,
    iid: i32,
) -> Result<CiBuildReport, Box<dyn std::error::Error + Send + Sync>> {
    let pipelines = gitlab_get_json(
        credentials,
        &format!(
            "{}/api/v4/projects/{}/merge_requests/{}/pipelines",
            coding_base_url(),
            urlencoding_encode(project_path),
            iid
        ),
        "Query MR pipelines",
    )
    .await?;
    let report = parse_gitlab_pipelines(&pipelines);
    if report.pass.is_some() || report.source != poria_core::pipeline::QualitySource::Missing {
        return Ok(report);
    }

    let mr = gitlab_get_json(
        credentials,
        &format!(
            "{}/api/v4/projects/{}/merge_requests/{}",
            coding_base_url(),
            urlencoding_encode(project_path),
            iid
        ),
        "Query MR for SHA",
    )
    .await?;
    let Some(sha) = mr
        .get("sha")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    else {
        return Ok(report);
    };
    let statuses = gitlab_get_json(
        credentials,
        &format!(
            "{}/api/v4/projects/{}/repository/commits/{}/statuses",
            coding_base_url(),
            urlencoding_encode(project_path),
            urlencoding_encode(sha)
        ),
        "Query commit statuses",
    )
    .await?;
    Ok(parse_gitlab_commit_statuses(&statuses))
}

async fn gitlab_get_json(
    credentials: &JacpCredentials,
    url: &str,
    error_label: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .header("Cookie", &credentials.cookie)
        .send()
        .await
        .map_err(|e| format!("{error_label} failed: {e}"))?;
    if response.status().as_u16() == 401 {
        return Err(AUTH_EXPIRED_USER_MESSAGE.into());
    }
    if !response.status().is_success() {
        return Err(format!("{error_label} failed: HTTP {}", response.status()).into());
    }
    Ok(response.json().await?)
}
