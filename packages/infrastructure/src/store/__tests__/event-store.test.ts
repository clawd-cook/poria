import { describe, it, expect, beforeEach } from "vitest";
import { initDatabase } from "../schema.js";
import { SqlitePipelineStore } from "../pipeline-repo.js";
import { EventStore } from "../event-store.js";
import { pipelineStartedEvent, stageStartedEvent, stageCompletedEvent } from "@poria/core";
import type { Pipeline } from "@poria/core";
import type Database from "better-sqlite3";

function makePipeline(id = "pl-20260916-evt00001"): Pipeline {
  return {
    id,
    demandId: 100,
    demandCode: "TEST001",
    demandName: "Event Test",
    status: "created",
    rawLink: "http://xingyun.jd.com/demands/view/TEST001/-1?demandId=100",
    operator: "testuser",
    hasRegressed: false,
    config: { gates: [], trdScope: [], repos: [] },
    stages: [],
    repos: [],
    createdAt: new Date("2026-09-16T00:00:00Z"),
    updatedAt: new Date("2026-09-16T00:00:00Z"),
  };
}

describe("EventStore", () => {
  let db: Database.Database;
  let pipelineStore: SqlitePipelineStore;
  let eventStore: EventStore;

  beforeEach(() => {
    db = initDatabase(":memory:");
    pipelineStore = new SqlitePipelineStore(db);
    eventStore = new EventStore(db);
  });

  it("should return events in seq order via queryByPipeline", () => {
    pipelineStore.create(makePipeline());
    const pipeline = pipelineStore.load("pl-20260916-evt00001")!;

    // Insert events via saveStageTx
    pipelineStore.saveStageTx(null, pipeline, [
      pipelineStartedEvent(pipeline.id),
      stageStartedEvent(pipeline.id, "init"),
    ]);

    const events = eventStore.queryByPipeline(pipeline.id);
    expect(events).toHaveLength(2);
    expect(events[0]!.kind).toBe("pipeline_started");
    expect(events[1]!.kind).toBe("stage_started");
  });

  it("should filter events by kind via queryByKind", () => {
    pipelineStore.create(makePipeline());
    const pipeline = pipelineStore.load("pl-20260916-evt00001")!;

    pipelineStore.saveStageTx(null, pipeline, [
      pipelineStartedEvent(pipeline.id),
      stageStartedEvent(pipeline.id, "init"),
      stageCompletedEvent(pipeline.id, "init", { projectDir: "/tmp" }),
      stageStartedEvent(pipeline.id, "review_prd"),
    ]);

    const stageEvents = eventStore.queryByKind(pipeline.id, "stage_started");
    expect(stageEvents).toHaveLength(2);
    expect(stageEvents[0]!.kind).toBe("stage_started");
  });

  it("should return empty array for pipeline with no events", () => {
    const events = eventStore.queryByPipeline("nonexistent");
    expect(events).toHaveLength(0);
  });

  it("should restore Date objects from event timestamps", () => {
    pipelineStore.create(makePipeline());
    const pipeline = pipelineStore.load("pl-20260916-evt00001")!;

    pipelineStore.saveStageTx(null, pipeline, [
      pipelineStartedEvent(pipeline.id),
    ]);

    const events = eventStore.queryByPipeline(pipeline.id);
    expect(events[0]!.timestamp).toBeInstanceOf(Date);
  });
});
