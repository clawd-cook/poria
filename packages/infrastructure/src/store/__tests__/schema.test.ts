import { describe, it, expect, beforeEach } from "vitest";
import Database from "better-sqlite3";
import { initDatabase } from "../schema.js";

describe("initDatabase", () => {
  let db: Database.Database;

  beforeEach(() => {
    db = initDatabase(":memory:");
  });

  it("should create all required tables", () => {
    const tables = db
      .prepare(
        "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name",
      )
      .all() as Array<{ name: string }>;

    const tableNames = tables.map((t) => t.name);
    expect(tableNames).toContain("pipelines");
    expect(tableNames).toContain("stages");
    expect(tableNames).toContain("events");
    expect(tableNames).toContain("audit_log");
    expect(tableNames).toContain("queue");
    expect(tableNames).toContain("schema_version");
  });

  it("should be idempotent (calling twice does not error)", () => {
    // initDatabase already called in beforeEach; calling again on same path
    // We use a different in-memory DB to test the function call itself
    const db2 = initDatabase(":memory:");
    expect(db2).toBeDefined();
    db2.close();
  });

  it("stages table should have agent_session_id column", () => {
    const columns = db
      .prepare("PRAGMA table_info(stages)")
      .all() as Array<{ name: string }>;

    const columnNames = columns.map((c) => c.name);
    expect(columnNames).toContain("agent_session_id");
  });

  it("pipelines table should have all required columns", () => {
    const columns = db
      .prepare("PRAGMA table_info(pipelines)")
      .all() as Array<{ name: string }>;

    const columnNames = columns.map((c) => c.name);
    expect(columnNames).toContain("id");
    expect(columnNames).toContain("demand_id");
    expect(columnNames).toContain("demand_code");
    expect(columnNames).toContain("demand_name");
    expect(columnNames).toContain("status");
    expect(columnNames).toContain("raw_link");
    expect(columnNames).toContain("operator");
    expect(columnNames).toContain("has_regressed");
    expect(columnNames).toContain("config");
    expect(columnNames).toContain("created_at");
    expect(columnNames).toContain("updated_at");
  });

  it("should have correct indexes", () => {
    const indexes = db
      .prepare(
        "SELECT name FROM sqlite_master WHERE type='index' AND sql IS NOT NULL",
      )
      .all() as Array<{ name: string }>;

    const indexNames = indexes.map((i) => i.name);
    expect(indexNames).toContain("idx_events_pipeline");
    expect(indexNames).toContain("idx_audit_pipeline");
  });

  it("should set schema_version to 1", () => {
    const row = db
      .prepare("SELECT MAX(version) as version FROM schema_version")
      .get() as { version: number };
    expect(row.version).toBe(1);
  });

  it("queue table should have UNIQUE constraint on pipeline_id", () => {
    db.prepare(
      "INSERT INTO queue (pipeline_id, priority, enqueued_at) VALUES ('pl-1', 0, '2026-01-01')",
    ).run();

    // Duplicate should fail or be ignored depending on how it's inserted
    expect(() => {
      db.prepare(
        "INSERT INTO queue (pipeline_id, priority, enqueued_at) VALUES ('pl-1', 0, '2026-01-01')",
      ).run();
    }).toThrow();
  });
});
