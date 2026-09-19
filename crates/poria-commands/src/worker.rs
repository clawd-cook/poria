use poria_core::pipeline::{transition_pipeline, PipelineEvent};
use poria_core::types::{Pipeline, PipelineStatus, StageEnum};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinSet;

use crate::traits::{
    CodingChannel, FileLock, HumanLoop, PipelineRun, PipelineStore, Queue, Recovery,
};

const QUEUE_POLL_INTERVAL: Duration = Duration::from_secs(5);
const MR_POLL_INTERVAL: Duration = Duration::from_secs(60);
const MR_STALE_THRESHOLD: Duration = Duration::from_secs(24 * 60 * 60);

/// Dependencies the worker needs beyond the executor itself.
pub struct WorkerDeps<S>
where
    S: PipelineStore,
{
    pub store: S,
    pub queue: Box<dyn Queue>,
    pub runner: Arc<dyn PipelineRun>,
    pub recovery: Box<dyn Recovery>,
    pub coding_channel: Option<Box<dyn CodingChannel>>,
    pub human_loop: Option<Box<dyn HumanLoop>>,
    /// Live cap of in-flight pipelines. Blocked / failed / waiting_merge must not hold a slot.
    pub max_parallel: Arc<dyn Fn() -> usize + Send + Sync>,
}

/// The background pipeline worker that consumes a queue and polls MR status.
pub struct PipelineWorker<S>
where
    S: PipelineStore,
{
    deps: WorkerDeps<S>,
    lock: Box<dyn FileLock>,
    stopped: Arc<AtomicBool>,
    poll_merge_requests: bool,
}

impl<S> PipelineWorker<S>
where
    S: PipelineStore,
{
    pub fn new(deps: WorkerDeps<S>, lock: Box<dyn FileLock>, stopped: Arc<AtomicBool>) -> Self {
        Self {
            deps,
            lock,
            stopped,
            poll_merge_requests: true,
        }
    }

    /// Desktop already polls WaitingMerge + HITL; skip the worker MR loop.
    pub fn without_mr_poll(mut self) -> Self {
        self.poll_merge_requests = false;
        self
    }

    pub fn stopper(&self) -> Arc<AtomicBool> {
        self.stopped.clone()
    }

    /// Start the worker loop. Acquires the file lock, recovers interrupted
    /// pipelines, then runs queue consumption and optional MR polling.
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self.lock.acquire() {
            return Err("Another Poria worker is already running".into());
        }
        let result = self.run_loops().await;
        self.lock.release();
        result
    }

    async fn run_loops(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.deps.recovery.recover_all().await?;
        self.stopped.store(false, Ordering::SeqCst);
        if self.poll_merge_requests {
            tokio::join!(self.consume_queue(), self.poll_merge_requests_loop());
        } else {
            self.consume_queue().await;
        }
        Ok(())
    }

    /// Signal the worker to stop after the current iteration.
    pub fn stop(&self) {
        self.stopped.store(true, Ordering::SeqCst);
    }

    async fn consume_queue(&self) {
        let in_flight = Arc::new(AtomicUsize::new(0));
        let mut tasks: JoinSet<()> = JoinSet::new();
        loop {
            if self.stopped.load(Ordering::SeqCst) {
                break;
            }
            while tasks.try_join_next().is_some() {}

            let max_parallel = (self.deps.max_parallel)().max(1);
            if in_flight.load(Ordering::SeqCst) >= max_parallel {
                let _ = tasks.join_next().await;
                continue;
            }

            if let Some(pipeline_id) = self.deps.queue.dequeue() {
                let runner = Arc::clone(&self.deps.runner);
                let in_flight = Arc::clone(&in_flight);
                in_flight.fetch_add(1, Ordering::SeqCst);
                tasks.spawn(async move {
                    let _ = runner.run_pipeline(&pipeline_id).await;
                    in_flight.fetch_sub(1, Ordering::SeqCst);
                });
            } else {
                tokio::time::sleep(QUEUE_POLL_INTERVAL).await;
            }
        }
        while tasks.join_next().await.is_some() {}
    }

    async fn poll_merge_requests_loop(&self) {
        let Some(coding) = self.deps.coding_channel.as_ref() else {
            return;
        };
        loop {
            tokio::time::sleep(MR_POLL_INTERVAL).await;
            if self.stopped.load(Ordering::SeqCst) {
                break;
            }

            if let Ok(pipelines) = self.deps.store.find_by_status("waiting_merge").await {
                for mut pipeline in pipelines {
                    self.check_merge_status(coding.as_ref(), &mut pipeline)
                        .await;
                }
            }
        }
    }

    async fn check_merge_status(&self, coding: &dyn CodingChannel, pipeline: &mut Pipeline) {
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
            match coding.get_mr_status(url).await {
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

        let deploy_stage = pipeline.stages.iter().find(|s| s.name == StageEnum::Deploy);
        if let Some(completed_at) = deploy_stage.and_then(|s| s.completed_at) {
            let elapsed = chrono::Utc::now() - completed_at;
            if elapsed.to_std().unwrap_or_default() > MR_STALE_THRESHOLD {
                if let Some(human_loop) = self.deps.human_loop.as_deref() {
                    let _ = human_loop
                        .escalate(
                            pipeline,
                            &format!("MR(s) pending merge for over 24h: {}", mr_urls.join(", ")),
                        )
                        .await;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use poria_core::pipeline::PipelineEvent;
    use poria_core::types::{Pipeline, Stage};
    use std::sync::Mutex;

    struct MemLock {
        deny: bool,
    }

    impl FileLock for MemLock {
        fn acquire(&self) -> bool {
            !self.deny
        }
        fn release(&self) {}
    }

    struct MemQueue {
        items: Mutex<Vec<String>>,
    }

    impl Queue for MemQueue {
        fn dequeue(&self) -> Option<String> {
            let mut items = self.items.lock().unwrap();
            if items.is_empty() {
                None
            } else {
                Some(items.remove(0))
            }
        }
    }

    struct RecordingRunner {
        ran: Mutex<Vec<String>>,
    }

    #[async_trait]
    impl PipelineRun for RecordingRunner {
        async fn run_pipeline(
            &self,
            pipeline_id: &str,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            self.ran.lock().unwrap().push(pipeline_id.to_string());
            Ok(())
        }
    }

    struct FlagRecovery {
        called: Mutex<bool>,
    }

    #[async_trait]
    impl Recovery for FlagRecovery {
        async fn recover_all(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            *self.called.lock().unwrap() = true;
            Ok(())
        }
    }

    struct EmptyStore;

    #[async_trait]
    impl PipelineStore for EmptyStore {
        async fn load(
            &self,
            _pipeline_id: &str,
        ) -> Result<Pipeline, Box<dyn std::error::Error + Send + Sync>> {
            Err("not found".into())
        }
        fn save_stage_tx(
            &self,
            _stage: Option<&Stage>,
            _pipeline: &Pipeline,
            _events: &[PipelineEvent],
        ) {
        }
        async fn find_by_status(
            &self,
            _status: &str,
        ) -> Result<Vec<Pipeline>, Box<dyn std::error::Error + Send + Sync>> {
            Ok(Vec::new())
        }
    }

    #[tokio::test]
    async fn start_fails_when_lock_held() {
        let worker = PipelineWorker::new(
            WorkerDeps {
                store: EmptyStore,
                queue: Box::new(MemQueue {
                    items: Mutex::new(vec![]),
                }),
                runner: Arc::new(RecordingRunner {
                    ran: Mutex::new(Vec::new()),
                }),
                recovery: Box::new(FlagRecovery {
                    called: Mutex::new(false),
                }),
                coding_channel: None,
                human_loop: None,
                max_parallel: Arc::new(|| 2),
            },
            Box::new(MemLock { deny: true }),
            Arc::new(AtomicBool::new(true)),
        )
        .without_mr_poll();
        let err = worker.start().await.unwrap_err().to_string();
        assert!(err.contains("already running"));
    }

    #[tokio::test]
    async fn recover_then_drain_runs_queued_ids() {
        let runner: Arc<dyn PipelineRun> = Arc::new(RecordingRunner {
            ran: Mutex::new(Vec::new()),
        });
        let recovery = FlagRecovery {
            called: Mutex::new(false),
        };
        let worker = PipelineWorker::new(
            WorkerDeps {
                store: EmptyStore,
                queue: Box::new(MemQueue {
                    items: Mutex::new(vec!["p1".into(), "p2".into()]),
                }),
                runner,
                recovery: Box::new(recovery),
                coding_channel: None,
                human_loop: None,
                max_parallel: Arc::new(|| 2),
            },
            Box::new(MemLock { deny: false }),
            Arc::new(AtomicBool::new(true)),
        )
        .without_mr_poll();

        worker.deps.recovery.recover_all().await.unwrap();
        let mut ran = Vec::new();
        while let Some(id) = worker.deps.queue.dequeue() {
            worker.deps.runner.run_pipeline(&id).await.unwrap();
            ran.push(id);
        }
        assert_eq!(ran, vec!["p1".to_string(), "p2".to_string()]);
    }

    struct SlowRunner {
        started: Arc<Mutex<Vec<String>>>,
    }

    #[async_trait]
    impl PipelineRun for SlowRunner {
        async fn run_pipeline(
            &self,
            pipeline_id: &str,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            self.started.lock().unwrap().push(pipeline_id.to_string());
            tokio::time::sleep(Duration::from_millis(80)).await;
            Ok(())
        }
    }

    #[tokio::test]
    async fn consume_queue_runs_two_pipelines_in_parallel() {
        let started = Arc::new(Mutex::new(Vec::new()));
        let stopped = Arc::new(AtomicBool::new(false));
        let worker = PipelineWorker::new(
            WorkerDeps {
                store: EmptyStore,
                queue: Box::new(MemQueue {
                    items: Mutex::new(vec!["p1".into(), "p2".into()]),
                }),
                runner: Arc::new(SlowRunner {
                    started: Arc::clone(&started),
                }),
                recovery: Box::new(FlagRecovery {
                    called: Mutex::new(false),
                }),
                coding_channel: None,
                human_loop: None,
                max_parallel: Arc::new(|| 2),
            },
            Box::new(MemLock { deny: false }),
            Arc::clone(&stopped),
        )
        .without_mr_poll();

        let handle = tokio::spawn(async move { worker.start().await });
        let deadline = tokio::time::Instant::now() + Duration::from_millis(500);
        let mut in_flight = 0;
        while tokio::time::Instant::now() < deadline {
            in_flight = started.lock().unwrap().len();
            if in_flight >= 2 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        stopped.store(true, Ordering::SeqCst);
        handle.await.unwrap().unwrap();
        assert!(
            in_flight >= 2,
            "expected two pipelines in flight, got {in_flight}"
        );
    }
}
