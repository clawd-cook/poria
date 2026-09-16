import type { Pipeline, PipelineEvent } from "@poria/core";
import {
  transitionPipeline,
  pipelineCompletedEvent,
  pipelineFailedEvent,
} from "@poria/core";
import type { IPipelineStore } from "./executor.js";
import type { PipelineExecutor } from "./executor.js";

export interface IWorkerDeps {
  store: IPipelineStore & { findByStatus(status: string): Promise<Pipeline[]> };
  queue: { dequeue(): string | null };
  executor: PipelineExecutor;
  recovery: { recoverAll(): Promise<void> };
  codingChannel: { getMrStatus(mrUrl: string): Promise<string> };
  humanLoop: { escalate(pipeline: Pipeline, message: string): Promise<void> };
}

export interface IFileLock {
  acquire(): void;
  release(): void;
}

const QUEUE_POLL_INTERVAL = 5_000;
const MR_POLL_INTERVAL = 60_000;
const MR_STALE_THRESHOLD = 24 * 60 * 60 * 1000;

export class PipelineWorker {
  private stopped = false;

  constructor(
    private readonly deps: IWorkerDeps,
    private readonly lock: IFileLock,
  ) {}

  async start(): Promise<void> {
    this.lock.acquire();
    await this.deps.recovery.recoverAll();
    this.stopped = false;
    await Promise.all([this.consumeQueue(), this.pollMergeRequests()]);
  }

  stop(): void {
    this.stopped = true;
    this.lock.release();
  }

  private async consumeQueue(): Promise<void> {
    while (!this.stopped) {
      const pipelineId = this.deps.queue.dequeue();
      if (pipelineId) {
        await this.deps.executor.run(pipelineId);
      } else {
        await sleep(QUEUE_POLL_INTERVAL);
      }
    }
  }

  private async pollMergeRequests(): Promise<void> {
    while (!this.stopped) {
      await sleep(MR_POLL_INTERVAL);
      if (this.stopped) break;

      const pipelines = await this.deps.store.findByStatus("waiting_merge");
      for (const pipeline of pipelines) {
        await this.checkMergeStatus(pipeline);
      }
    }
  }

  private async checkMergeStatus(pipeline: Pipeline): Promise<void> {
    const deployStage = pipeline.stages.find(s => s.name === "deploy");
    const mrUrls = deployStage?.output?.["mrUrls"] as string[] | undefined;

    if (!mrUrls || mrUrls.length === 0) {
      transitionPipeline(pipeline, "failed");
      const events: PipelineEvent[] = [pipelineFailedEvent(pipeline.id, "No MR URLs found in deploy output")];
      this.deps.store.saveStageTx(null, pipeline, events);
      return;
    }

    let allMerged = true;
    for (const url of mrUrls) {
      const status = await this.deps.codingChannel.getMrStatus(url);
      if (status === "closed") {
        transitionPipeline(pipeline, "failed");
        const events: PipelineEvent[] = [pipelineFailedEvent(pipeline.id, `MR closed: ${url}`)];
        this.deps.store.saveStageTx(null, pipeline, events);
        return;
      }
      if (status !== "merged") allMerged = false;
    }

    if (allMerged) {
      transitionPipeline(pipeline, "completed");
      const events: PipelineEvent[] = [pipelineCompletedEvent(pipeline.id)];
      this.deps.store.saveStageTx(null, pipeline, events);
      return;
    }

    const deployCompleted = deployStage?.completedAt;
    if (deployCompleted && Date.now() - deployCompleted.getTime() > MR_STALE_THRESHOLD) {
      await this.deps.humanLoop.escalate(pipeline, `MR(s) pending merge for over 24h: ${mrUrls.join(", ")}`);
    }
  }
}

function sleep(ms: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, ms));
}
