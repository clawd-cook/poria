use chrono::Utc;
use poria_core::pipeline::{evaluate_gates, transition_pipeline, transition_stage, PipelineEvent, StageResult};
use poria_core::types::{
    GateOnFail, GatePhase, Pipeline, PipelineStatus, RollbackCommand,
    RollbackCommandType, RollbackInstruction, SkillInput, SkillOutput, Stage, StageEnum,
    StageStatus, STAGE_ORDER,
};

use crate::handle_error::{handle_stage_error, ErrorAction};
use crate::traits::{
    is_multi_repo_stage, stage_skill_id, CredentialGuard, HumanLoop, MultiRepoOrchestrator,
    PipelineStore, SkillLoader,
};

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

    if stage.name == StageEnum::Workspace {
        if let Some(output) = &stage.output {
            if let Some(path) = output.get("worktreePath").and_then(|v| v.as_str()) {
                commands.push(RollbackCommand {
                    command_type: RollbackCommandType::RemoveWorktree,
                    params: [("path".into(), path.into())].into_iter().collect(),
                });
            }
            if let Some(branch) = output.get("branch").and_then(|v| v.as_str()) {
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
    pipeline
        .stages
        .iter()
        .find(|s| s.name == StageEnum::Deploy)
        .and_then(|s| s.output.as_ref())
        .and_then(|o| o.get("mrUrls"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
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
    pub async fn run(&self, pipeline_id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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

            // Check retry exhaustion
            if pipeline.stages[stage_idx].status == StageStatus::Failed
                && pipeline.stages[stage_idx].retry_count
                    >= pipeline.stages[stage_idx].max_retries
            {
                transition_pipeline(&mut pipeline.status, PipelineStatus::Failed)?;
                let stage_name = pipeline.stages[stage_idx].name;
                self.store.save_stage_tx(
                    Some(&pipeline.stages[stage_idx]),
                    &pipeline,
                    &[PipelineEvent::pipeline_failed(
                        &pipeline.id,
                        format!("Stage {:?} exhausted retries", stage_name),
                    )],
                );
                return Ok(());
            }

            match self.run_single_stage(&mut pipeline, stage_idx).await {
                Ok(StageOutcome::Continue) => continue,
                Ok(StageOutcome::Return) => return Ok(()),
                Err(e) => {
                    let error_message = e.to_string();
                    let hl_ref = self.human_loop.as_deref();
                    let result = handle_stage_error(
                        &mut pipeline.status,
                        &mut pipeline.stages[stage_idx],
                        &error_message,
                        hl_ref,
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
                        }
                        ErrorAction::Blocked => {
                            let stage = &pipeline.stages[stage_idx];
                            events.push(PipelineEvent::stage_blocked(
                                &pipeline.id,
                                stage.name,
                                result.issue_class,
                            ));
                        }
                        ErrorAction::Failed => {
                            events.push(PipelineEvent::pipeline_failed(
                                &pipeline.id,
                                &error_message,
                            ));
                        }
                    }

                    self.store.save_stage_tx(
                        Some(&pipeline.stages[stage_idx]),
                        &pipeline,
                        &events,
                    );

                    if result.action == ErrorAction::Blocked
                        || result.action == ErrorAction::Failed
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
        transition_stage(
            &mut pipeline.stages[stage_idx].status,
            StageStatus::Running,
        )?;
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

        let is_multi_repo =
            pipeline.repos.len() > 1 && is_multi_repo_stage(stage_name);
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
                    self.store.save_stage_tx(
                        Some(&pipeline.stages[stage_idx]),
                        pipeline,
                        &events,
                    );
                    return Ok(StageOutcome::Continue);
                }
                GateAction::Blocked => {
                    self.store.save_stage_tx(
                        Some(&pipeline.stages[stage_idx]),
                        pipeline,
                        &events,
                    );
                    return Ok(StageOutcome::Return);
                }
                GateAction::Pass => {}
            }
        }

        // Deploy gate evaluation
        if stage_name == StageEnum::Deploy {
            self.handle_deploy_gate(pipeline, stage_idx, &mut events);
            if pipeline.stages[stage_idx].status == StageStatus::Blocked {
                self.store.save_stage_tx(
                    Some(&pipeline.stages[stage_idx]),
                    pipeline,
                    &events,
                );
                return Ok(StageOutcome::Return);
            }
        }

        transition_stage(
            &mut pipeline.stages[stage_idx].status,
            StageStatus::Completed,
        )?;
        self.store.save_stage_tx(
            Some(&pipeline.stages[stage_idx]),
            pipeline,
            &events,
        );

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
        let evaluation =
            evaluate_gates(&cr_result, &pipeline.config.gates, GatePhase::StageExit);
        events.push(PipelineEvent::gate_evaluated(
            &pipeline.id,
            stage_name,
            evaluation.details.clone(),
        ));

        if evaluation.all_pass {
            return GateAction::Pass;
        }

        // Look for a regress rule that failed
        let regress_rule_data = pipeline.config.gates.iter().find(|r| {
            r.enabled
                && r.on_fail == GateOnFail::Regress
                && evaluation
                    .details
                    .iter()
                    .any(|d| d.rule_id == r.id && !d.pass)
        }).map(|r| (r.id.clone(), r.regress_to));

        if let Some((rule_id, regress_to_opt)) = regress_rule_data {
            if !pipeline.has_regressed {
                pipeline.has_regressed = true;
                let regress_to = regress_to_opt.unwrap_or(StageEnum::Dev);

                // Reset dev + cr stages to pending
                for s in pipeline.stages.iter_mut() {
                    if s.name == StageEnum::Dev {
                        s.status = StageStatus::Pending;
                        let cr_feedback = serde_json::json!({
                            "crScore": cr_result.cr_score,
                            "findings": findings,
                        });
                        let mut input = s
                            .input
                            .as_ref()
                            .and_then(|v| v.as_object().cloned())
                            .unwrap_or_default();
                        input.insert("crFeedback".into(), cr_feedback);
                        s.input = Some(serde_json::Value::Object(input));
                    }
                    if s.name == StageEnum::Cr {
                        s.status = StageStatus::Pending;
                    }
                }

                events.push(PipelineEvent::stage_regressed(
                    &pipeline.id,
                    StageEnum::Cr,
                    regress_to,
                    format!("CR score: {:?}", cr_result.cr_score),
                ));
                events.push(PipelineEvent::gate_regress_triggered(
                    &pipeline.id,
                    &rule_id,
                    StageEnum::Cr,
                    regress_to,
                ));
                return GateAction::Regress;
            }

            // Already regressed once -> block
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
        let evaluation =
            evaluate_gates(&deploy_result, &pipeline.config.gates, GatePhase::Deploy);
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
