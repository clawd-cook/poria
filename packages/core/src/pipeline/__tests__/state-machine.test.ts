import { describe, it, expect } from "vitest";
import {
  canPipelineTransition,
  canStageTransition,
  transitionPipeline,
  transitionStage,
  InvalidTransitionError,
} from "../state-machine.js";
import type { PipelineStatus, StageStatus } from "../../types/pipeline.js";

describe("Pipeline state machine", () => {
  describe("canPipelineTransition", () => {
    const valid: [PipelineStatus, PipelineStatus][] = [
      ["created", "running"],
      ["created", "cancelled"],
      ["running", "waiting_merge"],
      ["running", "blocked"],
      ["running", "failed"],
      ["running", "cancelled"],
      ["blocked", "running"],
      ["blocked", "cancelled"],
      ["waiting_merge", "completed"],
      ["waiting_merge", "failed"],
      ["waiting_merge", "cancelled"],
    ];

    it.each(valid)("%s → %s is valid", (from, to) => {
      expect(canPipelineTransition(from, to)).toBe(true);
    });

    const invalid: [PipelineStatus, PipelineStatus][] = [
      ["completed", "running"],
      ["completed", "failed"],
      ["completed", "cancelled"],
      ["cancelled", "running"],
      ["cancelled", "created"],
      ["created", "completed"],
      ["created", "failed"],
      ["created", "blocked"],
      ["created", "waiting_merge"],
      ["running", "created"],
      ["running", "completed"],
      ["blocked", "completed"],
      ["blocked", "failed"],
      ["blocked", "waiting_merge"],
      ["waiting_merge", "running"],
      ["waiting_merge", "blocked"],
      ["waiting_merge", "created"],
      ["failed", "running"],
      ["failed", "completed"],
      ["failed", "blocked"],
    ];

    it.each(invalid)("%s → %s is invalid", (from, to) => {
      expect(canPipelineTransition(from, to)).toBe(false);
    });
  });

  describe("transitionPipeline", () => {
    it("mutates status on valid transition", () => {
      const pipeline = { status: "created" as PipelineStatus };
      transitionPipeline(pipeline, "running");
      expect(pipeline.status).toBe("running");
    });

    it("supports chained transitions", () => {
      const pipeline = { status: "created" as PipelineStatus };
      transitionPipeline(pipeline, "running");
      transitionPipeline(pipeline, "waiting_merge");
      transitionPipeline(pipeline, "completed");
      expect(pipeline.status).toBe("completed");
    });

    it("throws InvalidTransitionError on invalid transition", () => {
      const pipeline = { status: "completed" as PipelineStatus };
      expect(() => transitionPipeline(pipeline, "running")).toThrow(InvalidTransitionError);
    });

    it("error contains correct fields", () => {
      const pipeline = { status: "created" as PipelineStatus };
      try {
        transitionPipeline(pipeline, "completed");
        expect.unreachable("should have thrown");
      } catch (e) {
        expect(e).toBeInstanceOf(InvalidTransitionError);
        const err = e as InvalidTransitionError;
        expect(err.entityType).toBe("pipeline");
        expect(err.from).toBe("created");
        expect(err.to).toBe("completed");
        expect(err.message).toContain("created");
        expect(err.message).toContain("completed");
      }
    });

    it("does not mutate status on invalid transition", () => {
      const pipeline = { status: "completed" as PipelineStatus };
      try { transitionPipeline(pipeline, "running"); } catch { /* expected */ }
      expect(pipeline.status).toBe("completed");
    });

    it("failed → cancelled is valid", () => {
      const pipeline = { status: "failed" as PipelineStatus };
      transitionPipeline(pipeline, "cancelled");
      expect(pipeline.status).toBe("cancelled");
    });
  });
});

describe("Stage state machine", () => {
  describe("canStageTransition", () => {
    const valid: [StageStatus, StageStatus][] = [
      ["pending", "running"],
      ["pending", "skipped"],
      ["running", "completed"],
      ["running", "failed"],
      ["running", "blocked"],
      ["failed", "running"],
      ["failed", "blocked"],
      ["blocked", "running"],
    ];

    it.each(valid)("%s → %s is valid", (from, to) => {
      expect(canStageTransition(from, to)).toBe(true);
    });

    const invalid: [StageStatus, StageStatus][] = [
      ["completed", "running"],
      ["completed", "pending"],
      ["completed", "failed"],
      ["skipped", "running"],
      ["skipped", "pending"],
      ["skipped", "completed"],
      ["pending", "completed"],
      ["pending", "failed"],
      ["pending", "blocked"],
      ["running", "pending"],
      ["running", "skipped"],
      ["blocked", "completed"],
      ["blocked", "failed"],
      ["blocked", "pending"],
      ["failed", "completed"],
      ["failed", "pending"],
      ["failed", "skipped"],
    ];

    it.each(invalid)("%s → %s is invalid", (from, to) => {
      expect(canStageTransition(from, to)).toBe(false);
    });
  });

  describe("transitionStage", () => {
    it("mutates status on valid transition", () => {
      const stage = { status: "pending" as StageStatus };
      transitionStage(stage, "running");
      expect(stage.status).toBe("running");
    });

    it("supports full lifecycle: pending → running → completed", () => {
      const stage = { status: "pending" as StageStatus };
      transitionStage(stage, "running");
      transitionStage(stage, "completed");
      expect(stage.status).toBe("completed");
    });

    it("supports retry: running → failed → running → completed", () => {
      const stage = { status: "pending" as StageStatus };
      transitionStage(stage, "running");
      transitionStage(stage, "failed");
      transitionStage(stage, "running");
      transitionStage(stage, "completed");
      expect(stage.status).toBe("completed");
    });

    it("supports blocked → resumed: running → blocked → running", () => {
      const stage = { status: "pending" as StageStatus };
      transitionStage(stage, "running");
      transitionStage(stage, "blocked");
      transitionStage(stage, "running");
      expect(stage.status).toBe("running");
    });

    it("supports cancel skip: pending → skipped", () => {
      const stage = { status: "pending" as StageStatus };
      transitionStage(stage, "skipped");
      expect(stage.status).toBe("skipped");
    });

    it("throws InvalidTransitionError on invalid transition", () => {
      const stage = { status: "completed" as StageStatus };
      expect(() => transitionStage(stage, "running")).toThrow(InvalidTransitionError);
    });

    it("error contains correct fields for stage", () => {
      const stage = { status: "skipped" as StageStatus };
      try {
        transitionStage(stage, "running");
        expect.unreachable("should have thrown");
      } catch (e) {
        expect(e).toBeInstanceOf(InvalidTransitionError);
        const err = e as InvalidTransitionError;
        expect(err.entityType).toBe("stage");
        expect(err.from).toBe("skipped");
        expect(err.to).toBe("running");
      }
    });

    it("does not mutate status on invalid transition", () => {
      const stage = { status: "skipped" as StageStatus };
      try { transitionStage(stage, "pending"); } catch { /* expected */ }
      expect(stage.status).toBe("skipped");
    });
  });
});
