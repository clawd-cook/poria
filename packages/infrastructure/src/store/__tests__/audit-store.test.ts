import { describe, it, expect, beforeEach } from "vitest";
import { initDatabase } from "../schema.js";
import { AuditStore } from "../audit-store.js";
import type Database from "better-sqlite3";

describe("AuditStore", () => {
  let db: Database.Database;
  let auditStore: AuditStore;

  beforeEach(() => {
    db = initDatabase(":memory:");
    auditStore = new AuditStore(db);
  });

  it("should record and query audit entries", () => {
    auditStore.record({
      pipelineId: "pl-test",
      stage: "deploy",
      action: "mr_create",
      operator: "testuser",
      detail: { mrUrl: "https://coding.jd.com/mr/1" },
    });

    auditStore.record({
      pipelineId: "pl-test",
      stage: "deploy",
      action: "git_push",
      operator: "testuser",
      detail: { branch: "feature_test" },
    });

    const entries = auditStore.queryByPipeline("pl-test");
    expect(entries).toHaveLength(2);
    expect(entries[0]!.action).toBe("mr_create");
    expect(entries[0]!.detail).toEqual({ mrUrl: "https://coding.jd.com/mr/1" });
    expect(entries[1]!.action).toBe("git_push");
  });

  it("should record entries without stage", () => {
    auditStore.record({
      pipelineId: "pl-test",
      action: "git_commit",
      operator: "testuser",
      detail: { hash: "abc123" },
    });

    const entries = auditStore.queryByPipeline("pl-test");
    expect(entries).toHaveLength(1);
    expect(entries[0]!.stage).toBeUndefined();
  });

  it("should return empty array for unknown pipeline", () => {
    const entries = auditStore.queryByPipeline("nonexistent");
    expect(entries).toHaveLength(0);
  });

  it("should include createdAt timestamp", () => {
    auditStore.record({
      pipelineId: "pl-test",
      action: "branch_delete",
      operator: "testuser",
      detail: {},
    });

    const entries = auditStore.queryByPipeline("pl-test");
    expect(entries[0]!.createdAt).toBeDefined();
    expect(typeof entries[0]!.createdAt).toBe("string");
  });
});
