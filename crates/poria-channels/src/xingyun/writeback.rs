use poria_core::types::PipelineStatus;

use super::{jacp_fetch, JacpCredentials};

/// Progress posted back to Xingyun using existing JACP communicate + remark.
#[derive(Debug, Clone)]
pub struct PipelineWriteback {
    pub demand_id: i64,
    pub demand_code: String,
    pub pipeline_id: String,
    pub status: PipelineStatus,
    pub mr_urls: Vec<String>,
}

pub fn writeback_remark(writeback: &PipelineWriteback) -> String {
    let status = serde_json::to_value(writeback.status)
        .ok()
        .and_then(|value| value.as_str().map(ToOwned::to_owned))
        .unwrap_or_else(|| format!("{:?}", writeback.status));
    let mrs = if writeback.mr_urls.is_empty() {
        "-".into()
    } else {
        writeback.mr_urls.join(" ")
    };
    format!(
        "【Poria】需求 {} 流水线 {} 状态 {}。MR: {}",
        writeback.demand_code, writeback.pipeline_id, status, mrs
    )
}

fn is_xingyun_fixture() -> bool {
    std::env::var("PORIA_XINGYUN_FIXTURE")
        .ok()
        .filter(|v| v == "1")
        .is_some()
        || std::env::var("PORIA_PIPELINE_FIXTURE")
            .ok()
            .filter(|v| v == "1")
            .is_some()
}

/// Write pipeline status and MR URLs onto the demand via communicate + remark.
/// Extra JSON fields are included so JACP can persist them if the action accepts them.
pub async fn writeback_pipeline_progress(
    credentials: &JacpCredentials,
    writeback: &PipelineWriteback,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let remark = writeback_remark(writeback);
    if is_xingyun_fixture() {
        return Ok(format!("fixture writeback: {remark}"));
    }
    if writeback.demand_id <= 0 {
        return Err("writeback requires demandId".into());
    }
    let status = serde_json::to_value(writeback.status)
        .ok()
        .and_then(|value| value.as_str().map(ToOwned::to_owned))
        .unwrap_or_default();
    let body = serde_json::json!({
        "demandId": writeback.demand_id,
        "remark": remark,
        "pipelineId": writeback.pipeline_id,
        "pipelineStatus": status,
        "mrUrls": writeback.mr_urls,
    });
    jacp_fetch(
        credentials,
        "/openapi/v3/demands/actions/communicate",
        reqwest::Method::POST,
        Some(body),
        "Writeback demand progress failed",
    )
    .await?;
    Ok(remark)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remark_includes_status_and_mr() {
        let text = writeback_remark(&PipelineWriteback {
            demand_id: 9,
            demand_code: "D1".into(),
            pipeline_id: "pl-1".into(),
            status: PipelineStatus::WaitingMerge,
            mr_urls: vec!["https://coding.jd.com/g/r/-/merge_requests/3".into()],
        });
        assert!(text.contains("waiting_merge"));
        assert!(text.contains("merge_requests/3"));
        assert!(text.contains("D1"));
    }
}
