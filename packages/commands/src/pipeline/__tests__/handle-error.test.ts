import { describe, it, expect, vi } from "vitest";
import type { Pipeline, Stage } from "@poria/core";
import { IssueClass } from "@poria/core";
import { handleStageError } from "../handle-error.js";
import type { IHumanLoop } from "../handle-error.js";

function makeStage(overrides?: Partial<Stage>): Stage {
  return {
    pipelineId: "pl-test",
    name: "dev",
    status: "running",
    retryCount: 0,
    maxRetries: 3,
    ...overrides,
  };
}

function makePipeline(overrides?: Partial<Pipeline>): Pipeline {
  return {
    id: "pl-test",
    demandId: 1,
    demandCode: "D-001",
    status: "running",
    rawLink: "",
    operator: "user1",
    hasRegressed: false,
    config: { gates: [], trdScope: [], repos: [] },
    stages: [],
    repos: [],
    createdAt: new Date(),
    updatedAt: new Date(),
    ...overrides,
  };
}

describe("handleStageError", () => {
  it("retries when autoRetry > 0 and retryCount < autoRetry", async () => {
    const stage = makeStage({ retryCount: 0 });
    const pipeline = makePipeline();

    // compilation_error has autoRetry: 3
    const result = await handleStageError(pipeline, stage, new Error("compilation failed"));

    expect(result.action).toBe("retry");
    expect(result.issueClass).toBe(IssueClass.COMPILATION_ERROR);
    expect(stage.status).toBe("failed");
    expect(stage.retryCount).toBe(1);
    expect(stage.issue?.retryable).toBe(true);
    expect(pipeline.status).toBe("running");
  });

  it("blocks when retries exhausted and notifyRoles present", async () => {
    const stage = makeStage({ retryCount: 3 });
    const pipeline = makePipeline();
    const humanLoop: IHumanLoop = { notify: vi.fn().mockResolvedValue(undefined) };

    // compilation_error has autoRetry: 3, notifyRoles: ["developer"]
    const result = await handleStageError(pipeline, stage, new Error("compilation failed"), humanLoop);

    expect(result.action).toBe("blocked");
    expect(stage.status).toBe("blocked");
    expect(pipeline.status).toBe("blocked");
    expect(humanLoop.notify).toHaveBeenCalledOnce();
  });

  it("fails when no notifyRoles (LLM rate limit exhausted)", async () => {
    const stage = makeStage({ retryCount: 5 });
    const pipeline = makePipeline();

    // LLM_RATE_LIMIT has autoRetry: 5, notifyRoles: [] — after 5 retries, no roles → fail
    const result = await handleStageError(pipeline, stage, new Error("rate limit exceeded"));

    expect(result.action).toBe("failed");
    expect(stage.status).toBe("failed");
    expect(pipeline.status).toBe("failed");
    expect(stage.issue?.retryable).toBe(false);
  });

  it("blocks immediately for non-retryable errors with notifyRoles", async () => {
    const stage = makeStage();
    const pipeline = makePipeline();
    const humanLoop: IHumanLoop = { notify: vi.fn().mockResolvedValue(undefined) };

    // requirement_ambiguous has autoRetry: 0, notifyRoles: ["product"]
    const result = await handleStageError(pipeline, stage, new Error("ambiguous requirement"), humanLoop);

    expect(result.action).toBe("blocked");
    expect(result.issueClass).toBe(IssueClass.REQUIREMENT_AMBIG);
    expect(stage.status).toBe("blocked");
    expect(pipeline.status).toBe("blocked");
  });

  it("handles non-Error values", async () => {
    const stage = makeStage();
    const pipeline = makePipeline();

    const result = await handleStageError(pipeline, stage, "string error");

    expect(result.issueClass).toBe(IssueClass.UNKNOWN);
    expect(stage.issue?.message).toBe("string error");
  });
});
