import { describe, it, expect, beforeEach, afterEach } from "vitest";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { initDatabase } from "../schema.js";
import { SqlitePipelineStore } from "../pipeline-repo.js";
import { EventArchiver, EventReplayService } from "../archiver.js";
import { pipelineStartedEvent, stageStartedEvent } from "@poria/core";
import type { Pipeline } from "@poria/core";
import type Database from "better-sqlite3";

function makePipeline(id: string, status = "completed", createdAt = "2026-07-01T00:00:00Z"): Pipeline {
  return {
    id,
    demandId: 100,
    demandCode: "TEST001",
    demandName: "Archive Test",
    status: status as Pipeline["status"],
    rawLink: "http://xingyun.jd.com/demands/view/TEST001/-1?demandId=100",
    operator: "testuser",
    hasRegressed: false,
    config: { gates: [], trdScope: [], repos: [] },
    stages: [],
    repos: [],
    createdAt: new Date(createdAt),
    updatedAt: new Date(createdAt),
  };
}

describe("EventArchiver", () => {
  let db: Database.Database;
  let store: SqlitePipelineStore;
  let archiveDir: string;
  let archiver: EventArchiver;

  beforeEach(() => {
    db = initDatabase(":memory:");
    store = new SqlitePipelineStore(db);
    archiveDir = fs.mkdtempSync(path.join(os.tmpdir(), "poria-archive-test-"));
    archiver = new EventArchiver(db, archiveDir);
  });

  afterEach(() => {
    fs.rmSync(archiveDir, { recursive: true, force: true });
  });

  it("should archive events for completed pipelines past retention", async () => {
    // Create a completed pipeline with old updated_at
    store.create(makePipeline("pl-old", "completed", "2026-07-01T00:00:00Z"));
    const pipeline = store.load("pl-old")!;
    pipeline.status = "completed";

    // Insert events
    store.saveStageTx(null, pipeline, [
      pipelineStartedEvent(pipeline.id),
      stageStartedEvent(pipeline.id, "init"),
    ]);

    // Force updated_at to be old enough
    db.prepare("UPDATE pipelines SET updated_at = '2026-07-01T00:00:00Z' WHERE id = ?")
      .run("pl-old");

    // Archive with 0 retention days (everything is past retention)
    const result = await archiver.archive(0);
    expect(result.archivedCount).toBe(1);

    // JSONL file should exist
    const jsonlPath = path.join(archiveDir, "2026-07", "events-pl-old.jsonl");
    expect(fs.existsSync(jsonlPath)).toBe(true);

    // SQLite events should be deleted
    const remaining = db
      .prepare("SELECT COUNT(*) as count FROM events WHERE pipeline_id = ?")
      .get("pl-old") as { count: number };
    expect(remaining.count).toBe(0);
  });

  it("should not archive running pipelines", async () => {
    store.create(makePipeline("pl-running", "running"));
    const pipeline = store.load("pl-running")!;
    pipeline.status = "running";
    store.saveStageTx(null, pipeline, [pipelineStartedEvent(pipeline.id)]);

    db.prepare("UPDATE pipelines SET updated_at = '2026-01-01T00:00:00Z' WHERE id = ?")
      .run("pl-running");

    const result = await archiver.archive(0);
    expect(result.archivedCount).toBe(0);
  });
});

describe("EventReplayService", () => {
  let db: Database.Database;
  let store: SqlitePipelineStore;
  let archiveDir: string;
  let replay: EventReplayService;

  beforeEach(() => {
    db = initDatabase(":memory:");
    store = new SqlitePipelineStore(db);
    archiveDir = fs.mkdtempSync(path.join(os.tmpdir(), "poria-replay-test-"));
    replay = new EventReplayService(db, archiveDir);
  });

  afterEach(() => {
    fs.rmSync(archiveDir, { recursive: true, force: true });
  });

  it("should replay from SQLite when events exist", async () => {
    store.create(makePipeline("pl-live"));
    const pipeline = store.load("pl-live")!;
    store.saveStageTx(null, pipeline, [
      pipelineStartedEvent(pipeline.id),
    ]);

    const events = await replay.replayAll("pl-live");
    expect(events).toHaveLength(1);
    expect(events[0]!.kind).toBe("pipeline_started");
  });

  it("should replay from archived JSONL when SQLite has no events", async () => {
    // Manually create a JSONL archive file
    const monthDir = path.join(archiveDir, "2026-07");
    fs.mkdirSync(monthDir, { recursive: true });

    const event = {
      seq: 1,
      pipeline_id: "pl-archived",
      kind: "pipeline_started",
      payload: JSON.stringify({
        pipelineId: "pl-archived",
        kind: "pipeline_started",
        timestamp: "2026-07-01T00:00:00Z",
      }),
      created_at: "2026-07-01T00:00:00Z",
    };

    fs.writeFileSync(
      path.join(monthDir, "events-pl-archived.jsonl"),
      JSON.stringify(event) + "\n",
      "utf-8",
    );

    const events = await replay.replayAll("pl-archived");
    expect(events).toHaveLength(1);
    expect(events[0]!.kind).toBe("pipeline_started");
  });

  it("should return empty array when no events anywhere", async () => {
    const events = await replay.replayAll("nonexistent");
    expect(events).toHaveLength(0);
  });
});
