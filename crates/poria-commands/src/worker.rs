use poria_core::pipeline::{transition_pipeline, PipelineEvent};
use poria_core::types::{Pipeline, PipelineStatus, StageEnum};
use std::time::Duration;

use crate::executor::PipelineExecutor;
use crate::traits::{
    CodingChannel, CredentialGuard, FileLock, HumanLoop, MultiRepoOrchestrator, PipelineStore,
    Queue, Recovery, SkillLoader,
};

const QUEUE_POLL_INTERVAL: Duration = Duration::from_secs(5);
const MR_POLL_INTERVAL: Duration = Duration::from_secs(60);
const MR_STALE_THRESHOLD: Duration = Duration::from_secs(24 * 60 * 60);

/// Dependencies the worker needs beyond the executor itself.
pub struct WorkerDeps<S, L, C, M>
where
    S: PipelineStore,
    L: SkillLoader,
    C: CredentialGuard,
    M: MultiRepoOrchestrator,
{
    pub store: S,
    pub queue: Box<dyn Queue>,
    pub executor: PipelineExecutor<S, L, C, M>,
    pub recovery: Box<dyn Recovery>,
    pub coding_channel: Box<dyn CodingChannel>,
    pub human_loop: Box<dyn HumanLoop>,
}

/// The background pipeline worker that consumes a queue and polls MR status.
pub struct PipelineWorker<S, L, C, M = ()>
where
    S: PipelineStore,
    L: SkillLoader,
    C: CredentialGuard,
    M: MultiRepoOrchestrator,
{
    deps: WorkerDeps<S, L, C, M>,
    lock: Box<dyn FileLock>,
    stopped: bool,
}

impl<S, L, C, M> PipelineWorker<S, L, C, M>
where
    S: PipelineStore,
    L: SkillLoader,
    C: CredentialGuard,
    M: MultiRepoOrchestrator,
{
    pub fn new(deps: WorkerDeps<S, L, C, M>, lock: Box<dyn FileLock>) -> Self {
        Self {
            deps,
            lock,
            stopped: true,
        }
    }

    /// Start the worker loop. Acquires the file lock, recovers interrupted
    /// pipelines, then runs queue consumption and MR polling concurrently.
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.lock.acquire();
        self.deps.recovery.recover_all().await?;
        self.stopped = false;
        tokio::join!(self.consume_queue(), self.poll_merge_requests());
        Ok(())
    }

    /// Signal the worker to stop after the current iteration.
    pub fn stop(&mut self) {
        self.stopped = true;
        self.lock.release();
    }

    async fn consume_queue(&self) {
        if self.stopped {
            return;
        }
        loop {
            if let Some(pipeline_id) = self.deps.queue.dequeue() {
                let _ = self.deps.executor.run(&pipeline_id).await;
            } else {
                tokio::time::sleep(QUEUE_POLL_INTERVAL).await;
            }
        }
    }

    async fn poll_merge_requests(&self) {
        if self.stopped {
            return;
        }
        loop {
            tokio::time::sleep(MR_POLL_INTERVAL).await;
            if self.stopped {
                break;
            }

            if let Ok(pipelines) = self.deps.store.find_by_status("waiting_merge").await {
                for mut pipeline in pipelines {
                    self.check_merge_status(&mut pipeline).await;
                }
            }
        }
    }

    async fn check_merge_status(&self, pipeline: &mut Pipeline) {
        let mr_urls = poria_core::pipeline::collect_deploy_mr_urls(pipeline);

        if mr_urls.is_empty() {
            if transition_pipeline(&mut pipeline.status, PipelineStatus::Failed).is_ok() {
                self.deps.store.save_stage_tx(
                    None,
                    pipeline,
                    &[PipelineEvent::pipeline_failed(
                        &pipeline.id,
                        "No MR URLs found in deploy output",
                    )],
                );
            }
            return;
        }

        let mut all_merged = true;
        for url in &mr_urls {
            match self.deps.coding_channel.get_mr_status(url).await {
                Ok(status) if status == "closed" => {
                    if transition_pipeline(&mut pipeline.status, PipelineStatus::Failed).is_ok() {
                        self.deps.store.save_stage_tx(
                            None,
                            pipeline,
                            &[PipelineEvent::pipeline_failed(
                                &pipeline.id,
                                format!("MR closed: {}", url),
                            )],
                        );
                    }
                    return;
                }
                Ok(status) if status == "merged" => {}
                _ => {
                    all_merged = false;
                }
            }
        }

        if all_merged {
            if transition_pipeline(&mut pipeline.status, PipelineStatus::Completed).is_ok() {
                self.deps.store.save_stage_tx(
                    None,
                    pipeline,
                    &[PipelineEvent::pipeline_completed(&pipeline.id)],
                );
            }
            return;
        }

        // Check for stale MRs
        let deploy_stage = pipeline.stages.iter().find(|s| s.name == StageEnum::Deploy);
        if let Some(completed_at) = deploy_stage.and_then(|s| s.completed_at) {
            let elapsed = chrono::Utc::now() - completed_at;
            if elapsed.to_std().unwrap_or_default() > MR_STALE_THRESHOLD {
                let _ = self
                    .deps
                    .human_loop
                    .escalate(
                        pipeline,
                        &format!("MR(s) pending merge for over 24h: {}", mr_urls.join(", ")),
                    )
                    .await;
            }
        }
    }
}
