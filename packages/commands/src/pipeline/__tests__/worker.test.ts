import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import type { Pipeline, Stage } from "@poria/core";
import { DEFAULT_GATES } from "@poria/core";
import { PipelineWorker } from "../worker.js";
import type { IWorkerDeps, IFileLock } from "../worker.js";
import type { PipelineExecutor } from "../executor.js";

function makePipeline(overrides?: Partial<Pipeline>): Pipeline {
  return {
    id: "pl-test",
    demandId: 1,
    demandCode: "D-001",
    status: "waiting_merge",
    rawLink: "",
    operator: "user1",
    hasRegressed: false,
    config: { gates: DEFAULT_GATES, trdScope: [], repos: [] },
    stages: [
      {
        pipelineId: "pl-test",
        name: "deploy",
        status: "completed",
        retryCount: 0,
        maxRetries: 3,
        output: { mrUrls: ["https://coding.jd.com/g/p/-/merge_requests/1"] },
        completedAt: new Date(),
      },
    ] as Stage[],
    repos: [],
    createdAt: new Date(),
    updatedAt: new Date(),
    ...overrides,
  };
}

describe("PipelineWorker", () => {
  let deps: IWorkerDeps;
  let lock: IFileLock;

  beforeEach(() => {
    vi.useFakeTimers();
    deps = {
      store: {
        load: vi.fn(),
        saveStageTx: vi.fn(),
        findByStatus: vi.fn().mockResolvedValue([]),
      },
      queue: { dequeue: vi.fn().mockReturnValue(null) },
      executor: { run: vi.fn().mockResolvedValue(undefined) } as unknown as PipelineExecutor,
      recovery: { recoverAll: vi.fn().mockResolvedValue(undefined) },
      codingChannel: { getMrStatus: vi.fn().mockResolvedValue("opened") },
      humanLoop: { escalate: vi.fn().mockResolvedValue(undefined) },
    };
    lock = { acquire: vi.fn(), release: vi.fn() };
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("acquires lock and runs recovery on start", async () => {
    const worker = new PipelineWorker(deps, lock);

    const startPromise = worker.start();
    // Let microtasks (recoverAll) settle
    await vi.advanceTimersByTimeAsync(0);

    expect(lock.acquire).toHaveBeenCalledOnce();
    expect(deps.recovery.recoverAll).toHaveBeenCalledOnce();

    worker.stop();
    // Advance past sleep intervals so loops exit
    await vi.advanceTimersByTimeAsync(60_000);
    await startPromise;
  });

  it("consumeQueue calls executor.run when queue has item", async () => {
    const worker = new PipelineWorker(deps, lock);

    let calls = 0;
    (deps.queue.dequeue as ReturnType<typeof vi.fn>).mockImplementation(() => {
      calls++;
      if (calls === 1) return "pl-123";
      worker.stop();
      return null;
    });

    const startPromise = worker.start();
    // Advance past sleep to trigger second dequeue iteration
    await vi.advanceTimersByTimeAsync(60_000);
    await startPromise;

    expect(deps.executor.run).toHaveBeenCalledWith("pl-123");
  });

  it("marks pipeline completed when all MRs merged", async () => {
    const pipeline = makePipeline();
    (deps.codingChannel.getMrStatus as ReturnType<typeof vi.fn>).mockResolvedValue("merged");

    const worker = new PipelineWorker(deps, lock);
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    await (worker as any).checkMergeStatus(pipeline);

    expect(pipeline.status).toBe("completed");
    expect(deps.store.saveStageTx).toHaveBeenCalled();
  });

  it("marks pipeline failed when MR is closed", async () => {
    const pipeline = makePipeline();
    (deps.codingChannel.getMrStatus as ReturnType<typeof vi.fn>).mockResolvedValue("closed");

    const worker = new PipelineWorker(deps, lock);
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    await (worker as any).checkMergeStatus(pipeline);

    expect(pipeline.status).toBe("failed");
    expect(deps.store.saveStageTx).toHaveBeenCalled();
  });

  it("fails pipeline when mrUrls empty", async () => {
    const pipeline = makePipeline();
    pipeline.stages[0]!.output = {};

    const worker = new PipelineWorker(deps, lock);
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    await (worker as any).checkMergeStatus(pipeline);

    expect(pipeline.status).toBe("failed");
  });

  it("escalates when MR is stale for over 24h", async () => {
    const pipeline = makePipeline();
    pipeline.stages[0]!.completedAt = new Date(Date.now() - 25 * 60 * 60 * 1000);
    (deps.codingChannel.getMrStatus as ReturnType<typeof vi.fn>).mockResolvedValue("opened");

    const worker = new PipelineWorker(deps, lock);
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    await (worker as any).checkMergeStatus(pipeline);

    expect(deps.humanLoop.escalate).toHaveBeenCalledOnce();
  });
});
