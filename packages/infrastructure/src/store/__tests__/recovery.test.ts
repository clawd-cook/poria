import { describe, it, expect, beforeEach, vi } from "vitest";
import { initDatabase } from "../schema.js";
import { SqlitePipelineStore } from "../pipeline-repo.js";
import { PipelineRecovery } from "../recovery.js";
import type { IWorktreeCleaner, IHumanLoopNotifier } from "../recovery.js";
import type { Pipeline, Stage } from "@poria/core";
import type Database from "better-sqlite3";

function makePipeline(overrides?: Partial<Pipeline>): Pipeline {
  return {
    id: "pl-20260916-recov001",
    demandId: 100,
    demandCode: "TEST001",
    demandName: "Recovery Test",
    status: "created",
    rawLink: "http://xingyun.jd.com/demands/view/TEST001/-1?demandId=100",
    operator: "testuser",
    hasRegressed: false,
    config: { gates: [], trdScope: [], repos: [{ name: "main-repo", gitUrl: "git@test.com:group/repo.git", branch: "feature_test", baseBranch: "master", gitlabProjectPath: "group/repo" }] },
    stages: [],
    repos: [{ name: "main-repo", gitUrl: "git@test.com:group/repo.git", branch: "feature_test", baseBranch: "master", gitlabProjectPath: "group/repo" }],
    createdAt: new Date("2026-09-16T00:00:00Z"),
    updatedAt: new Date("2026-09-16T00:00:00Z"),
    ...overrides,
  };
}

describe("PipelineRecovery", () => {
  let db: Database.Database;
  let store: SqlitePipelineStore;
  let mockCleaner: IWorktreeCleaner;
  let mockHumanLoop: IHumanLoopNotifier;
  let recovery: PipelineRecovery;

  beforeEach(() => {
    db = initDatabase(":memory:");
    store = new SqlitePipelineStore(db);
    mockCleaner = { cleanDirtyState: vi.fn().mockResolvedValue(undefined) };
    mockHumanLoop = { renotify: vi.fn().mockResolvedValue(undefined) };
    recovery = new PipelineRecovery(store, mockCleaner, mockHumanLoop);
  });

  it("should recover a running pipeline with interrupted stage", async () => {
    // Create pipeline and manually set it to running with a running stage
    store.create(makePipeline({ status: "running" }));
    const pipeline = store.load("pl-20260916-recov001")!;

    // Mark pipeline as running
    pipeline.status = "running";
    // Simulate: init completed, review_prd running (interrupted)
    const initStage = pipeline.stages[0]!;
    initStage.status = "completed";
    store.saveStageTx(initStage, pipeline, []);

    const reviewStage = pipeline.stages[1]!;
    reviewStage.status = "running";
    store.saveStageTx(reviewStage, pipeline, []);

    // Now recover
    const result = await recovery.recoverAll();
    expect(result.runningRecovered).toBe(1);

    // Verify the interrupted stage is now failed with retryCount++
    const recovered = store.load("pl-20260916-recov001")!;
    const recoveredReview = recovered.stages[1]!;
    expect(recoveredReview.status).toBe("failed");
    expect(recoveredReview.retryCount).toBe(1);
  });

  it("should clean worktree for interrupted dev stage", async () => {
    store.create(makePipeline({ status: "running" }));
    const pipeline = store.load("pl-20260916-recov001")!;
    pipeline.status = "running";

    // Simulate dev stage running (interrupted)
    for (let i = 0; i < 4; i++) {
      pipeline.stages[i]!.status = "completed";
    }
    pipeline.stages[4]!.status = "running"; // dev

    store.saveStageTx(pipeline.stages[4]!, pipeline, []);
    // Also need to persist previous stages
    for (let i = 0; i < 4; i++) {
      store.saveStageTx(pipeline.stages[i]!, pipeline, []);
    }

    await recovery.recoverAll();

    // Worktree cleaner should have been called
    expect(mockCleaner.cleanDirtyState).toHaveBeenCalledWith(
      "main-repo",
      "pl-20260916-recov001",
    );
  });

  it("should renotify blocked pipelines", async () => {
    store.create(makePipeline({ id: "pl-blocked", status: "blocked" }));
    const pipeline = store.load("pl-blocked")!;
    pipeline.status = "blocked";

    const crStage = pipeline.stages[5]!; // cr
    crStage.status = "blocked";
    crStage.issue = { class: "low_cr_score", message: "Score too low", retryable: false };

    store.saveStageTx(crStage, pipeline, []);

    await recovery.recoverAll();

    expect(mockHumanLoop.renotify).toHaveBeenCalledTimes(1);
    const call = vi.mocked(mockHumanLoop.renotify).mock.calls[0]!;
    expect(call[0].id).toBe("pl-blocked");
    expect((call[1] as Stage).name).toBe("cr");
  });

  it("should handle no running or blocked pipelines gracefully", async () => {
    const result = await recovery.recoverAll();
    expect(result.runningRecovered).toBe(0);
    expect(result.blockedRenotified).toBe(0);
    expect(result.errors).toHaveLength(0);
  });
});
