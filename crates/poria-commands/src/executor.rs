use chrono::Utc;
use poria_core::pipeline::{
    evaluate_gates, jme_notify_target, stamp_human_loop_notified, transition_pipeline,
    transition_stage, PipelineEvent, StageResult, DEFAULT_GATES,
};
use poria_core::types::{
    GatePhase, Pipeline, PipelineStatus, RollbackCommand, RollbackCommandType, RollbackInstruction,
    SkillInput, SkillOutput, Stage, StageEnum, StageStatus, STAGE_ORDER,
};

use crate::handle_error::{handle_stage_error, ErrorAction};
use crate::traits::{
    is_multi_repo_stage, stage_skill_id, CredentialGuard, HumanLoop, MultiRepoOrchestrator,
    PipelineStore, SkillLoader,
};

fn pipeline_gate_rules(pipeline: &Pipeline) -> &[poria_core::types::GateRule] {
    if pipeline.config.gates.is_empty() {
        &DEFAULT_GATES
    } else {
        &pipeline.config.gates
    }
}

/// Finds the first stage that is neither completed nor skipped.
fn find_current_stage(pipeline: &Pipeline) -> Option<usize> {
    pipeline
        .stages
        .iter()
        .position(|s| s.status != StageStatus::Completed && s.status != StageStatus::Skipped)
}

/// Builds rollback instructions for a completed stage based on its output.
fn build_rollback_instructions(stage: &Stage, _pipeline: &Pipeline) -> Option<RollbackInstruction> {
    let stage_index = STAGE_ORDER
        .iter()
        .position(|s| *s == stage.name)
        .unwrap_or(0) as i32;
    let mut commands: Vec<RollbackCommand> = Vec::new();

    if stage.name == StageEnum::Init {
        if let Some(output) = &stage.output {
            let frontend_path = output
                .get("worktreePath")
                .and_then(|v| v.as_str())
                .or_else(|| {
                    output
                        .get("repos")
                        .and_then(|repos| repos.as_array())
                        .and_then(|repos| repos.first())
                        .and_then(|repo| repo.get("worktreePath"))
                        .and_then(|v| v.as_str())
                });
            if let Some(path) = frontend_path {
                commands.push(RollbackCommand {
                    command_type: RollbackCommandType::RemoveWorktree,
                    params: [("path".into(), path.into())].into_iter().collect(),
                });
            }
            if let Some(path) = output.get("backendWorktreePath").and_then(|v| v.as_str()) {
                commands.push(RollbackCommand {
                    command_type: RollbackCommandType::RemoveWorktree,
                    params: [("path".into(), path.into())].into_iter().collect(),
                });
            }
            if let Some(path) = output.get("workspacePath").and_then(|v| v.as_str()) {
                commands.push(RollbackCommand {
                    command_type: RollbackCommandType::RemoveDirectory,
                    params: [("path".into(), path.into())].into_iter().collect(),
                });
            }
            if let Some(branch) = output
                .get("repos")
                .and_then(|repos| repos.as_array())
                .and_then(|repos| repos.first())
                .and_then(|repo| repo.get("branch"))
                .and_then(|v| v.as_str())
            {
                commands.push(RollbackCommand {
                    command_type: RollbackCommandType::DeleteBranch,
                    params: [("branch".into(), branch.into())].into_iter().collect(),
                });
            }
        }
    }

    if stage.name == StageEnum::Deploy {
        if let Some(output) = &stage.output {
            if let Some(mr_urls) = output.get("mrUrls").and_then(|v| v.as_array()) {
                for url_val in mr_urls {
                    if let Some(url) = url_val.as_str() {
                        commands.push(RollbackCommand {
                            command_type: RollbackCommandType::CloseMr,
                            params: [("mrUrl".into(), url.into())].into_iter().collect(),
                        });
                    }
                }
            }
        }
    }

    if commands.is_empty() {
        None
    } else {
        Some(RollbackInstruction {
            stage_index,
            commands,
        })
    }
}

/// Collects MR URLs from the deploy stage output.
fn collect_mr_urls(pipeline: &Pipeline) -> Vec<String> {
    poria_core::pipeline::collect_deploy_mr_urls(pipeline)
}

/// The main pipeline execution engine.
///
/// Iterates through stages in order, loading and running the corresponding skill
/// for each stage, handling gate evaluation for CR and deploy stages, and
/// delegating error handling to the issue-policy system.
pub struct PipelineExecutor<S, L, C, M = ()>
where
    S: PipelineStore,
    L: SkillLoader,
    C: CredentialGuard,
    M: MultiRepoOrchestrator,
{
    store: S,
    skill_loader: L,
    human_loop: Option<Box<dyn HumanLoop>>,
    credential_guard: C,
    multi_repo_orchestrator: Option<M>,
}

impl<S, L, C, M> PipelineExecutor<S, L, C, M>
where
    S: PipelineStore,
    L: SkillLoader,
    C: CredentialGuard,
    M: MultiRepoOrchestrator,
{
    pub fn new(
        store: S,
        skill_loader: L,
        credential_guard: C,
        human_loop: Option<Box<dyn HumanLoop>>,
        multi_repo_orchestrator: Option<M>,
    ) -> Self {
        Self {
            store,
            skill_loader,
            human_loop,
            credential_guard,
            multi_repo_orchestrator,
        }
    }

    /// Runs a pipeline to completion (or until it blocks/fails).
    pub async fn run(
        &self,
        pipeline_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut pipeline = self.store.load(pipeline_id).await?;

        if pipeline.status == PipelineStatus::Created {
            transition_pipeline(&mut pipeline.status, PipelineStatus::Running)?;
            self.store.save_stage_tx(
                None,
                &pipeline,
                &[PipelineEvent::pipeline_started(&pipeline.id)],
            );
        }

        while pipeline.status == PipelineStatus::Running {
            let stage_idx = match find_current_stage(&pipeline) {
                Some(idx) => idx,
                None => {
                    // All stages done -> waiting_merge
                    transition_pipeline(&mut pipeline.status, PipelineStatus::WaitingMerge)?;
                    let mr_urls = collect_mr_urls(&pipeline);
                    self.store.save_stage_tx(
                        None,
                        &pipeline,
                        &[PipelineEvent::pipeline_waiting_merge(&pipeline.id, mr_urls)],
                    );
                    return Ok(());
                }
            };

            // Retry / block / fail from ISSUE_POLICIES, not stage.max_retries
            match self.run_single_stage(&mut pipeline, stage_idx).await {
                Ok(StageOutcome::Continue) => continue,
                Ok(StageOutcome::Return) => return Ok(()),
                Err(e) => {
                    let error_message = e.to_string();
                    let result = handle_stage_error(
                        &mut pipeline.status,
                        &mut pipeline.stages[stage_idx],
                        &error_message,
                        None,
                        None,
                    )
                    .await;
                    let mut events: Vec<PipelineEvent> = Vec::new();

                    match result.action {
                        ErrorAction::Retry => {
                            let stage = &pipeline.stages[stage_idx];
                            events.push(PipelineEvent::stage_failed(
                                &pipeline.id,
                                stage.name,
                                &error_message,
                                stage.retry_count,
                            ));
                            let delay = crate::retry_delay_for_message(&error_message);
                            if !delay.is_zero() {
                                tokio::time::sleep(delay).await;
                            }
                        }
                        ErrorAction::Blocked => {
                            let target = jme_notify_target(&pipeline, result.issue_class.as_str());
                            stamp_human_loop_notified(
                                &mut pipeline.stages[stage_idx],
                                &target,
                                Utc::now(),
                            );
                            let stage = &pipeline.stages[stage_idx];
                            events.push(PipelineEvent::stage_blocked(
                                &pipeline.id,
                                stage.name,
                                result.issue_class,
                            ));
                        }
                        ErrorAction::Failed => {
                            events
                                .push(PipelineEvent::pipeline_failed(&pipeline.id, &error_message));
                        }
                    }

                    self.store
                        .save_stage_tx(Some(&pipeline.stages[stage_idx]), &pipeline, &events);

                    if result.action == ErrorAction::Blocked {
                        if let Some(hl) = self.human_loop.as_deref() {
                            let _ = hl
                                .notify(
                                    &pipeline,
                                    &pipeline.stages[stage_idx],
                                    result.issue_class.as_str(),
                                )
                                .await;
                        }
                    }

                    if result.action == ErrorAction::Blocked || result.action == ErrorAction::Failed
                    {
                        return Ok(());
                    }
                    // Retry -> continue loop
                }
            }
        }

        Ok(())
    }

    /// Execute one stage. Returns whether the main loop should continue or return.
    async fn run_single_stage(
        &self,
        pipeline: &mut Pipeline,
        stage_idx: usize,
    ) -> Result<StageOutcome, Box<dyn std::error::Error + Send + Sync>> {
        let credentials = self.credential_guard.ensure_valid().await?;

        // Transition stage to running
        transition_stage(&mut pipeline.stages[stage_idx].status, StageStatus::Running)?;
        pipeline.stages[stage_idx].started_at = Some(Utc::now());
        self.store.save_stage_tx(
            Some(&pipeline.stages[stage_idx]),
            pipeline,
            &[PipelineEvent::stage_started(
                &pipeline.id,
                pipeline.stages[stage_idx].name,
            )],
        );

        let stage_name = pipeline.stages[stage_idx].name;
        let skill_id = stage_skill_id(stage_name);
        let skill = self.skill_loader.load(skill_id).await?;

        let is_multi_repo = pipeline.repos.len() > 1 && is_multi_repo_stage(stage_name);
        let result: SkillOutput = if is_multi_repo {
            if let Some(ref orch) = self.multi_repo_orchestrator {
                orch.execute(
                    pipeline,
                    &pipeline.stages[stage_idx],
                    skill.as_ref(),
                    &credentials,
                )
                .await?
            } else {
                let ctx = poria_core::contracts::SkillContext {
                    pipeline_id: pipeline.id.clone(),
                    workdir: pipeline.stages[stage_idx]
                        .input
                        .as_ref()
                        .and_then(|i| i.get("workdir"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    credentials: credentials.clone(),
                };
                let input = SkillInput {
                    stage: pipeline.stages[stage_idx].clone(),
                    pipeline: pipeline.clone(),
                    extra: serde_json::Map::new(),
                };
                skill.execute(input, ctx).await?
            }
        } else {
            let ctx = poria_core::contracts::SkillContext {
                pipeline_id: pipeline.id.clone(),
                workdir: pipeline.stages[stage_idx]
                    .input
                    .as_ref()
                    .and_then(|i| i.get("workdir"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                credentials: credentials.clone(),
            };
            let input = SkillInput {
                stage: pipeline.stages[stage_idx].clone(),
                pipeline: pipeline.clone(),
                extra: serde_json::Map::new(),
            };
            skill.execute(input, ctx).await?
        };

        pipeline.stages[stage_idx].output = Some(result.output.clone());
        pipeline.stages[stage_idx].completed_at = Some(Utc::now());

        let rollback = build_rollback_instructions(&pipeline.stages[stage_idx], pipeline);
        if let Some(rb) = rollback {
            pipeline.stages[stage_idx].rollback = Some(rb);
        }

        let mut events: Vec<PipelineEvent> = vec![PipelineEvent::stage_completed(
            &pipeline.id,
            stage_name,
            result.output.clone(),
        )];

        // CR gate evaluation
        if stage_name == StageEnum::Cr {
            let action = self.handle_cr_gate(pipeline, stage_idx, &mut events);
            match action {
                GateAction::Regress => {
                    self.store
                        .save_stage_tx(Some(&pipeline.stages[stage_idx]), pipeline, &events);
                    return Ok(StageOutcome::Continue);
                }
                GateAction::Blocked => {
                    self.store
                        .save_stage_tx(Some(&pipeline.stages[stage_idx]), pipeline, &events);
                    return Ok(StageOutcome::Return);
                }
                GateAction::Pass => {}
            }
        }

        // Deploy gate evaluation
        if stage_name == StageEnum::Deploy {
            self.handle_deploy_gate(pipeline, stage_idx, &mut events);
            if pipeline.stages[stage_idx].status == StageStatus::Blocked {
                self.store
                    .save_stage_tx(Some(&pipeline.stages[stage_idx]), pipeline, &events);
                return Ok(StageOutcome::Return);
            }
        }

        transition_stage(
            &mut pipeline.stages[stage_idx].status,
            StageStatus::Completed,
        )?;
        self.store
            .save_stage_tx(Some(&pipeline.stages[stage_idx]), pipeline, &events);

        // Deploy complete -> waiting_merge
        if stage_name == StageEnum::Deploy {
            transition_pipeline(&mut pipeline.status, PipelineStatus::WaitingMerge)?;
            let mr_urls = collect_mr_urls(pipeline);
            self.store.save_stage_tx(
                None,
                pipeline,
                &[PipelineEvent::pipeline_waiting_merge(&pipeline.id, mr_urls)],
            );
            return Ok(StageOutcome::Return);
        }

        Ok(StageOutcome::Continue)
    }

    fn handle_cr_gate(
        &self,
        pipeline: &mut Pipeline,
        stage_idx: usize,
        events: &mut Vec<PipelineEvent>,
    ) -> GateAction {
        // Extract data we need from the stage up front to avoid holding an
        // immutable borrow across the later iter_mut().
        let stage_name = pipeline.stages[stage_idx].name;
        let stage_output = pipeline.stages[stage_idx].output.clone();
        let cr_result = StageResult {
            cr_score: stage_output
                .as_ref()
                .and_then(|o| o.get("crScore"))
                .and_then(|v| v.as_str())
                .map(String::from),
            security_pass: stage_output
                .as_ref()
                .and_then(|o| o.get("securityPass"))
                .and_then(|v| v.as_bool()),
            ..Default::default()
        };
        let findings = stage_output
            .as_ref()
            .and_then(|o| o.get("findings"))
            .cloned();
        let evaluation = evaluate_gates(
            &cr_result,
            pipeline_gate_rules(pipeline),
            GatePhase::StageExit,
        );
        events.push(PipelineEvent::gate_evaluated(
            &pipeline.id,
            stage_name,
            evaluation.details.clone(),
        ));

        if evaluation.all_pass {
            return GateAction::Pass;
        }

        if evaluation
            .blocking_failures
            .iter()
            .any(|detail| detail.rule_id == "security_scan")
        {
            pipeline.stages[stage_idx].status = StageStatus::Blocked;
            pipeline.status = PipelineStatus::Blocked;
            events.push(PipelineEvent::stage_blocked(
                &pipeline.id,
                pipeline.stages[stage_idx].name,
                poria_core::types::IssueClass::SecurityViolation,
            ));
            return GateAction::Blocked;
        }

        let cr_score_failed = evaluation
            .details
            .iter()
            .any(|detail| detail.rule_id == "cr_score" && !detail.pass);
        if cr_score_failed {
            if crate::try_regress_cr_to_dev(
                pipeline,
                cr_result.cr_score.as_deref(),
                findings.as_ref(),
            ) {
                events.push(PipelineEvent::stage_regressed(
                    &pipeline.id,
                    StageEnum::Cr,
                    StageEnum::Dev,
                    format!("CR score: {:?}", cr_result.cr_score),
                ));
                events.push(PipelineEvent::gate_regress_triggered(
                    &pipeline.id,
                    "cr_score",
                    StageEnum::Cr,
                    StageEnum::Dev,
                ));
                return GateAction::Regress;
            }
            pipeline.stages[stage_idx].status = StageStatus::Blocked;
            pipeline.status = PipelineStatus::Blocked;
            events.push(PipelineEvent::stage_blocked(
                &pipeline.id,
                pipeline.stages[stage_idx].name,
                poria_core::types::IssueClass::LowCrScore,
            ));
            return GateAction::Blocked;
        }

        // Non-regress blocking failure
        pipeline.stages[stage_idx].status = StageStatus::Blocked;
        pipeline.status = PipelineStatus::Blocked;
        GateAction::Blocked
    }

    fn handle_deploy_gate(
        &self,
        pipeline: &mut Pipeline,
        stage_idx: usize,
        events: &mut Vec<PipelineEvent>,
    ) {
        let stage_name = pipeline.stages[stage_idx].name;
        let stage_output = pipeline.stages[stage_idx].output.clone();
        let deploy_result = StageResult {
            ci_build_pass: stage_output
                .as_ref()
                .and_then(|o| o.get("ciBuildPass"))
                .and_then(|v| v.as_bool()),
            test_coverage: stage_output
                .as_ref()
                .and_then(|o| o.get("testCoverage"))
                .and_then(|v| v.as_f64()),
            diff_lines: stage_output
                .as_ref()
                .and_then(|o| o.get("diffLines"))
                .and_then(|v| v.as_i64())
                .map(|v| v as i32),
            has_conflict: stage_output
                .as_ref()
                .and_then(|o| o.get("hasConflict"))
                .and_then(|v| v.as_bool()),
            ..Default::default()
        };
        let evaluation = evaluate_gates(
            &deploy_result,
            pipeline_gate_rules(pipeline),
            GatePhase::Deploy,
        );
        events.push(PipelineEvent::gate_evaluated(
            &pipeline.id,
            stage_name,
            evaluation.details,
        ));

        if !evaluation.blocking_failures.is_empty() {
            pipeline.stages[stage_idx].status = StageStatus::Blocked;
            pipeline.status = PipelineStatus::Blocked;
            return;
        }

        if !evaluation.warn_failures.is_empty() {
            let warnings: Vec<serde_json::Value> = evaluation
                .warn_failures
                .iter()
                .map(|f| serde_json::Value::String(f.message.clone()))
                .collect();
            let mut output = pipeline.stages[stage_idx]
                .output
                .as_ref()
                .and_then(|v| v.as_object().cloned())
                .unwrap_or_default();
            output.insert("gateWarnings".into(), serde_json::Value::Array(warnings));
            pipeline.stages[stage_idx].output = Some(serde_json::Value::Object(output));
        }
    }
}

enum StageOutcome {
    Continue,
    Return,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GateAction {
    Pass,
    Regress,
    Blocked,
}

#[cfg(test)]
mod tests {
    use super::*;
    use poria_core::types::{PipelineConfig, StageStatus};
    use serde_json::json;

    fn init_stage_with_output(output: serde_json::Value) -> Stage {
        Stage {
            id: None,
            pipeline_id: "p1".into(),
            name: StageEnum::Init,
            status: StageStatus::Completed,
            skill_id: Some("skill:init".into()),
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
        }
    }

    fn dummy_pipeline() -> Pipeline {
        Pipeline {
            id: "p1".into(),
            demand_id: 1,
            demand_code: "R1".into(),
            demand_name: None,
            status: PipelineStatus::Running,
            raw_link: String::new(),
            operator: "tester".into(),
            has_regressed: false,
            config: PipelineConfig::default(),
            stages: vec![],
            repos: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn init_rollback_includes_frontend_and_backend_worktrees() {
        let stage = init_stage_with_output(json!({
            "workspacePath": "/tmp/.poria/workspaces/p1",
            "worktreePath": "/tmp/.poria/workspaces/p1/fe",
            "backendWorktreePath": "/tmp/.poria/workspaces/p1/be",
            "repos": [{
                "branch": "feature_R1",
                "worktreePath": "/tmp/.poria/workspaces/p1/fe"
            }]
        }));
        let rollback = build_rollback_instructions(&stage, &dummy_pipeline()).unwrap();
        let paths: Vec<_> = rollback
            .commands
            .iter()
            .filter(|cmd| cmd.command_type == RollbackCommandType::RemoveWorktree)
            .filter_map(|cmd| cmd.params.get("path").map(String::as_str))
            .collect();
        assert_eq!(
            paths,
            vec![
                "/tmp/.poria/workspaces/p1/fe",
                "/tmp/.poria/workspaces/p1/be"
            ]
        );
        assert!(rollback.commands.iter().any(|cmd| {
            cmd.command_type == RollbackCommandType::RemoveDirectory
                && cmd.params.get("path").map(String::as_str) == Some("/tmp/.poria/workspaces/p1")
        }));
        assert!(rollback.commands.iter().any(|cmd| {
            cmd.command_type == RollbackCommandType::DeleteBranch
                && cmd.params.get("branch").map(String::as_str) == Some("feature_R1")
        }));
    }
}
