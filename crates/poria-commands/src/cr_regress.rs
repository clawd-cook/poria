use poria_core::types::{Pipeline, StageEnum, StageStatus};

/// Reset Dev + CR to pending at most once. Returns false if already used.
pub fn try_regress_cr_to_dev(
    pipeline: &mut Pipeline,
    cr_score: Option<&str>,
    findings: Option<&serde_json::Value>,
) -> bool {
    if pipeline.has_regressed {
        return false;
    }
    pipeline.has_regressed = true;
    pipeline.status = poria_core::types::PipelineStatus::Running;
    for stage in pipeline.stages.iter_mut() {
        if stage.name == StageEnum::Dev {
            stage.status = StageStatus::Pending;
            stage.issue = None;
            let cr_feedback = serde_json::json!({
                "crScore": cr_score,
                "findings": findings,
            });
            let mut input = stage
                .input
                .as_ref()
                .and_then(|v| v.as_object().cloned())
                .unwrap_or_default();
            input.insert("crFeedback".into(), cr_feedback);
            stage.input = Some(serde_json::Value::Object(input));
        }
        if stage.name == StageEnum::Cr {
            stage.status = StageStatus::Pending;
            stage.issue = None;
            stage.completed_at = None;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use poria_core::types::{PipelineConfig, PipelineStatus};

    fn pipeline() -> Pipeline {
        Pipeline {
            id: "pl-cr".into(),
            demand_id: 1,
            demand_code: "D-1".into(),
            demand_name: None,
            status: PipelineStatus::Running,
            raw_link: String::new(),
            operator: "dev".into(),
            has_regressed: false,
            config: PipelineConfig::default(),
            stages: vec![
                poria_core::types::Stage {
                    id: None,
                    pipeline_id: "pl-cr".into(),
                    name: StageEnum::Dev,
                    status: StageStatus::Completed,
                    skill_id: None,
                    retry_count: 0,
                    max_retries: 3,
                    input: None,
                    output: None,
                    gate_results: None,
                    issue: None,
                    rollback: None,
                    agent_session_id: None,
                    started_at: None,
                    completed_at: None,
                },
                poria_core::types::Stage {
                    id: None,
                    pipeline_id: "pl-cr".into(),
                    name: StageEnum::Cr,
                    status: StageStatus::Running,
                    skill_id: None,
                    retry_count: 0,
                    max_retries: 3,
                    input: None,
                    output: None,
                    gate_results: None,
                    issue: None,
                    rollback: None,
                    agent_session_id: None,
                    started_at: None,
                    completed_at: None,
                },
            ],
            repos: vec![],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn regresses_once_then_refuses() {
        let mut p = pipeline();
        assert!(try_regress_cr_to_dev(&mut p, Some("C"), None));
        assert!(p.has_regressed);
        assert_eq!(p.stages[0].status, StageStatus::Pending);
        assert_eq!(p.stages[1].status, StageStatus::Pending);
        assert!(!try_regress_cr_to_dev(&mut p, Some("C"), None));
    }
}
