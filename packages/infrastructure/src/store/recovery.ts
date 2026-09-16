import type { Pipeline, Stage, StageEnum, PipelineEvent } from "@poria/core";
import type { SqlitePipelineStore } from "./pipeline-repo.js";
import { worktreeCleanedEvent, stageFailedEvent } from "@poria/core";

/**
 * Dependency-injection interface for worktree cleanup.
 * Avoids direct dependency on resources/worktree package.
 */
export interface IWorktreeCleaner {
  cleanDirtyState(repoName: string, pipelineId: string): Promise<void>;
}

/**
 * Dependency-injection interface for human-loop notifications.
 * Avoids direct dependency on channels/jme package.
 */
export interface IHumanLoopNotifier {
  renotify(pipeline: Pipeline, stage: Stage): Promise<void>;
}

/**
 * Recovers interrupted pipelines on system restart.
 *
 * 1. Finds all RUNNING pipelines
 * 2. Marks interrupted stages as failed + increments retryCount
 * 3. Cleans dirty worktree state for dev/cr stages
 * 4. Re-sends notifications for BLOCKED pipelines
 */
export class PipelineRecovery {
  constructor(
    private readonly store: SqlitePipelineStore,
    private readonly worktreeCleaner: IWorktreeCleaner | null,
    private readonly humanLoop: IHumanLoopNotifier | null,
  ) {}

  async recoverAll(): Promise<RecoveryResult> {
    const result: RecoveryResult = {
      runningRecovered: 0,
      blockedRenotified: 0,
      errors: [],
    };

    // 1. Recover RUNNING pipelines
    const running = this.store.findByStatus("running");

    for (const pipeline of running) {
      try {
        await this.recoverPipeline(pipeline);
        result.runningRecovered++;
      } catch (err) {
        result.errors.push({
          pipelineId: pipeline.id,
          error: err instanceof Error ? err.message : String(err),
        });
      }
    }

    // 2. Re-notify BLOCKED pipelines
    const blocked = this.store.findByStatus("blocked");

    for (const p of blocked) {
      const blockedStage = p.stages.find((s) => s.status === "blocked");
      if (blockedStage?.issue && this.humanLoop) {
        try {
          await this.humanLoop.renotify(p, blockedStage);
          result.blockedRenotified++;
        } catch (err) {
          result.errors.push({
            pipelineId: p.id,
            error: err instanceof Error ? err.message : String(err),
          });
        }
      }
    }

    return result;
  }

  private async recoverPipeline(pipeline: Pipeline): Promise<void> {
    // Find last completed stage to determine resume point
    // (The executor will use this when re-running the pipeline)
    const lastCompletedIdx = pipeline.stages.findLastIndex(
      (s) => s.status === "completed",
    );

    // Find the interrupted running stage
    const interrupted = pipeline.stages.find((s) => s.status === "running");
    if (!interrupted) return;

    const events: PipelineEvent[] = [];

    // Clean dirty worktree state for dev/cr stages
    const dirtyStages: StageEnum[] = ["dev", "cr"];
    if (dirtyStages.includes(interrupted.name) && this.worktreeCleaner) {
      for (const repo of pipeline.repos) {
        try {
          await this.worktreeCleaner.cleanDirtyState(repo.name, pipeline.id);
          events.push(
            worktreeCleanedEvent(
              pipeline.id,
              repo.name,
              `recovery: cleaned uncommitted changes (resume from stage ${lastCompletedIdx + 1})`,
            ),
          );
        } catch {
          // Best-effort cleanup; continue with other repos
        }
      }
    }

    // Mark interrupted stage as failed + increment retry count
    interrupted.status = "failed";
    interrupted.retryCount++;
    events.push(
      stageFailedEvent(
        pipeline.id,
        interrupted.name,
        "Interrupted during execution (recovery)",
        interrupted.retryCount,
      ),
    );

    // Atomic write
    this.store.saveStageTx(interrupted, pipeline, events);
  }
}

export interface RecoveryResult {
  runningRecovered: number;
  blockedRenotified: number;
  errors: Array<{ pipelineId: string; error: string }>;
}
