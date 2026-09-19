use poria_channels::coding::{post_mr_note_live, JacpCredentials};
use poria_core::types::{Pipeline, StageEnum};

use crate::cr_findings::cr_stage_mr_notes;
use crate::fixture::is_fixture_mode;

fn cr_stage_output(pipeline: &Pipeline) -> Option<&serde_json::Value> {
    pipeline
        .stages
        .iter()
        .find(|stage| stage.name == StageEnum::Cr)
        .and_then(|stage| stage.output.as_ref())
}

/// Post independent CR P0/P1 notes to the Coding MR created by Deploy.
///
/// Auth failures fail the Job. Other post errors are warnings and do not fail Deploy.
pub async fn post_cr_blocking_notes(
    creds: &JacpCredentials,
    project_path: &str,
    iid: i32,
    pipeline: &Pipeline,
) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    if is_fixture_mode() {
        return Ok(0);
    }
    let notes = cr_stage_mr_notes(cr_stage_output(pipeline));
    let mut posted = 0;
    for note in notes {
        match post_mr_note_live(creds, project_path, iid, &note).await {
            Ok(()) => posted += 1,
            Err(err) => {
                let message = err.to_string();
                if message.contains(poria_core::types::AUTH_EXPIRED_USER_MESSAGE)
                    || message.contains("请先登录")
                {
                    return Err(err);
                }
                tracing::warn!(
                    pipeline_id = %pipeline.id,
                    error = %message,
                    "failed to post independent CR note; deploy continues"
                );
            }
        }
    }
    Ok(posted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use poria_core::types::{PipelineConfig, PipelineStatus};

    fn empty_pipeline() -> Pipeline {
        Pipeline {
            id: "p1".into(),
            demand_id: 1,
            demand_code: "R1".into(),
            demand_name: None,
            status: PipelineStatus::Running,
            raw_link: String::new(),
            operator: "t".into(),
            has_regressed: false,
            config: PipelineConfig::default(),
            stages: vec![],
            repos: vec![],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[tokio::test]
    async fn no_cr_output_posts_zero_notes() {
        let creds = JacpCredentials {
            cookie: "c".into(),
            username: "u".into(),
        };
        let posted = post_cr_blocking_notes(&creds, "g/r", 1, &empty_pipeline())
            .await
            .unwrap();
        assert_eq!(posted, 0);
    }
}
