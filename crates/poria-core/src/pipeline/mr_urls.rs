use crate::types::{Pipeline, StageEnum};

/// Parse GitLab/Coding MR iid from a `.../merge_requests/<iid>` URL.
pub fn parse_mr_iid_from_url(url: &str) -> Option<i32> {
    let rest = url.split("/merge_requests/").nth(1)?;
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

/// Deploy output may use `mrUrls` (array) or `mrUrl` (single).
pub fn collect_deploy_mr_urls(pipeline: &Pipeline) -> Vec<String> {
    let Some(output) = pipeline
        .stages
        .iter()
        .find(|stage| stage.name == StageEnum::Deploy)
        .and_then(|stage| stage.output.as_ref())
    else {
        return Vec::new();
    };
    let mut urls = Vec::new();
    if let Some(items) = output.get("mrUrls").and_then(|v| v.as_array()) {
        for item in items {
            if let Some(url) = item.as_str().map(str::trim).filter(|s| !s.is_empty()) {
                urls.push(url.to_string());
            }
        }
    }
    if urls.is_empty() {
        if let Some(url) = output
            .get("mrUrl")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            urls.push(url.to_string());
        }
    }
    urls
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{PipelineConfig, PipelineStatus, Stage, StageStatus};
    use chrono::Utc;

    fn pipeline_with_output(output: serde_json::Value) -> Pipeline {
        Pipeline {
            id: "pl-mr".into(),
            demand_id: 1,
            demand_code: "D-1".into(),
            demand_name: None,
            status: PipelineStatus::WaitingMerge,
            raw_link: String::new(),
            operator: "dev".into(),
            has_regressed: false,
            config: PipelineConfig::default(),
            stages: vec![Stage {
                id: None,
                pipeline_id: "pl-mr".into(),
                name: StageEnum::Deploy,
                status: StageStatus::Completed,
                skill_id: None,
                retry_count: 0,
                max_retries: 3,
                input: None,
                output: Some(output),
                gate_results: None,
                issue: None,
                rollback: None,
                agent_session_id: None,
                started_at: None,
                completed_at: None,
            }],
            repos: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn reads_singular_mr_url() {
        let pipeline = pipeline_with_output(serde_json::json!({
            "mrUrl": "https://coding.jd.com/g/r/-/merge_requests/9"
        }));
        assert_eq!(
            collect_deploy_mr_urls(&pipeline),
            vec!["https://coding.jd.com/g/r/-/merge_requests/9"]
        );
    }

    #[test]
    fn prefers_mr_urls_array() {
        let pipeline = pipeline_with_output(serde_json::json!({
            "mrUrl": "https://coding.jd.com/g/r/-/merge_requests/1",
            "mrUrls": ["https://coding.jd.com/g/r/-/merge_requests/2"]
        }));
        assert_eq!(
            collect_deploy_mr_urls(&pipeline),
            vec!["https://coding.jd.com/g/r/-/merge_requests/2"]
        );
    }

    #[test]
    fn empty_when_no_mr_fields() {
        assert!(collect_deploy_mr_urls(&pipeline_with_output(serde_json::json!({}))).is_empty());
    }

    #[test]
    fn parse_mr_iid_from_coding_url() {
        assert_eq!(
            parse_mr_iid_from_url("https://coding.jd.com/g/r/-/merge_requests/42"),
            Some(42)
        );
        assert_eq!(
            parse_mr_iid_from_url("https://coding.jd.com/g/r/-/merge_requests/7#note_1"),
            Some(7)
        );
        assert_eq!(parse_mr_iid_from_url("https://example.com/no-mr"), None);
    }
}
