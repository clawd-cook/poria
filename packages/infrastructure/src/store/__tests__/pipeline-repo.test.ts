import { describe, it, expect, beforeEach } from "vitest";
import { initDatabase } from "../schema.js";
import { SqlitePipelineStore } from "../pipeline-repo.js";
import type { Pipeline, PipelineEvent } from "@poria/core";
import { pipelineStartedEvent, stageStartedEvent } from "@poria/core";
import type Database from "better-sqlite3";

function makePipeline(overrides?: Partial<Pipeline>): Pipeline {
  return {
    id: "pl-20260916-test0001",
    demandId: 4840029,
    demandCode: "JL3R4IV4",
    demandName: "Test Demand",
    status: "created",
    rawLink: "http://xingyun.jd.com/demands/view/JL3R4IV4/-1?demandId=4840029",
    operator: "testuser",
    hasRegressed: false,
    config: { gates: [], trdScope: [], repos: [] },
    stages: [],
    repos: [],
    createdAt: new Date("2026-09-16T00:00:00Z"),
    updatedAt: new Date("2026-09-16T00:00:00Z"),
    ...overrides,
  };
}

describe("SqlitePipelineStore", () => {
  let db: Database.Database;
  let store: SqlitePipelineStore;

  beforeEach(() => {
    db = initDatabase(":memory:");
    store = new SqlitePipelineStore(db);
  });

  describe("create + load (round-trip)", () => {
    it("should create a pipeline with 7 stages and load it back", () => {
      const pipeline = makePipeline();
      store.create(pipeline);

      const loaded = store.load(pipeline.id);
      expect(loaded).toBeDefined();
      expect(loaded!.id).toBe(pipeline.id);
      expect(loaded!.demandId).toBe(4840029);
      expect(loaded!.demandCode).toBe("JL3R4IV4");
      expect(loaded!.demandName).toBe("Test Demand");
      expect(loaded!.status).toBe("created");
      expect(loaded!.operator).toBe("testuser");
      expect(loaded!.hasRegressed).toBe(false);
      expect(loaded!.stages).toHaveLength(7);

      // Verify stage names are in order
      const stageNames = loaded!.stages.map((s) => s.name);
      expect(stageNames).toEqual([
        "init",
        "review_prd",
        "design",
        "workspace",
        "dev",
        "cr",
        "deploy",
      ]);

      // All stages start as pending
      for (const stage of loaded!.stages) {
        expect(stage.status).toBe("pending");
        expect(stage.retryCount).toBe(0);
        expect(stage.maxRetries).toBe(3);
      }
    });

    it("should return undefined for non-existent pipeline", () => {
      const loaded = store.load("pl-nonexistent");
      expect(loaded).toBeUndefined();
    });
  });

  describe("findByStatus", () => {
    it("should find pipelines by status", () => {
      store.create(makePipeline({ id: "pl-1", status: "created" }));
      store.create(makePipeline({ id: "pl-2", status: "created" }));
      store.create(makePipeline({ id: "pl-3", status: "running" }));

      // Need to update pl-3 status via saveStageTx
      const pl3 = store.load("pl-3")!;
      pl3.status = "running";
      store.saveStageTx(null, pl3, []);

      const created = store.findByStatus("created");
      // pl-1 and pl-2 are still created
      expect(created.length).toBeGreaterThanOrEqual(2);

      const running = store.findByStatus("running");
      expect(running).toHaveLength(1);
      expect(running[0]!.id).toBe("pl-3");
    });

    it("should return empty array when no pipelines match", () => {
      const result = store.findByStatus("blocked");
      expect(result).toHaveLength(0);
    });
  });

  describe("saveStageTx", () => {
    it("should atomically update stage + pipeline + events", () => {
      store.create(makePipeline());
      const pipeline = store.load("pl-20260916-test0001")!;
      const initStage = pipeline.stages[0]!;

      // Update stage status
      initStage.status = "running";
      initStage.startedAt = new Date();
      pipeline.status = "running";

      const events: PipelineEvent[] = [
        pipelineStartedEvent(pipeline.id),
        stageStartedEvent(pipeline.id, "init"),
      ];

      store.saveStageTx(initStage, pipeline, events);

      // Verify pipeline status updated
      const reloaded = store.load(pipeline.id)!;
      expect(reloaded.status).toBe("running");

      // Verify stage status updated
      expect(reloaded.stages[0]!.status).toBe("running");
      expect(reloaded.stages[0]!.startedAt).toBeDefined();

      // Verify events were inserted
      const eventRows = db
        .prepare("SELECT * FROM events WHERE pipeline_id = ?")
        .all(pipeline.id) as Array<{ kind: string }>;
      expect(eventRows).toHaveLength(2);
      expect(eventRows[0]!.kind).toBe("pipeline_started");
      expect(eventRows[1]!.kind).toBe("stage_started");
    });

    it("should work with stage = null (pipeline-only update)", () => {
      store.create(makePipeline());
      const pipeline = store.load("pl-20260916-test0001")!;
      pipeline.demandName = "Updated Name";

      store.saveStageTx(null, pipeline, []);

      const reloaded = store.load(pipeline.id)!;
      expect(reloaded.demandName).toBe("Updated Name");
    });

    it("should rollback all changes on error (atomicity)", () => {
      store.create(makePipeline());
      const pipeline = store.load("pl-20260916-test0001")!;
      const initStage = pipeline.stages[0]!;

      initStage.status = "running";
      pipeline.status = "running";

      // Create a malformed event that will cause the insert to fail
      // We'll test by checking that after a failed saveStageTx,
      // the data remains unchanged
      const originalPipeline = store.load(pipeline.id)!;
      expect(originalPipeline.status).toBe("created");

      // Force an error by providing an event with null pipelineId
      const badEvent = {
        pipelineId: null as unknown as string,
        kind: "pipeline_started",
        timestamp: new Date(),
      } as PipelineEvent;

      expect(() => store.saveStageTx(initStage, pipeline, [badEvent])).toThrow();

      // Verify nothing changed
      const afterError = store.load("pl-20260916-test0001")!;
      expect(afterError.status).toBe("created");
      expect(afterError.stages[0]!.status).toBe("pending");
    });
  });

  describe("JSON field serialization", () => {
    it("should round-trip config with nested objects", () => {
      const pipeline = makePipeline({
        config: {
          gates: [
            {
              id: "cr_score",
              name: "CR Score",
              enabled: true,
              threshold: "B+",
              onFail: "regress",
              gatePhase: "stage_exit",
              regressTo: "dev",
            },
          ],
          trdScope: ["src/**/*.ts"],
          repos: [],
        },
      });

      store.create(pipeline);
      const loaded = store.load(pipeline.id)!;
      expect(loaded.config.gates).toHaveLength(1);
      expect(loaded.config.gates[0]!.id).toBe("cr_score");
      expect(loaded.config.trdScope).toEqual(["src/**/*.ts"]);
    });

    it("should round-trip stage issue and rollback", () => {
      store.create(makePipeline());
      const pipeline = store.load("pl-20260916-test0001")!;
      const stage = pipeline.stages[0]!;

      stage.issue = { class: "compilation_error", message: "Build failed", retryable: true };
      stage.rollback = {
        stageIndex: 0,
        commands: [{ type: "delete_branch", params: { branch: "feature_test" } }],
      };
      stage.status = "failed";

      store.saveStageTx(stage, pipeline, []);

      const reloaded = store.load(pipeline.id)!;
      expect(reloaded.stages[0]!.issue!.class).toBe("compilation_error");
      expect(reloaded.stages[0]!.rollback!.commands).toHaveLength(1);
    });
  });
});
