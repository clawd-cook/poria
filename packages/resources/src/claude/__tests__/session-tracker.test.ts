import { describe, it, expect, beforeEach } from "vitest";
import { SessionTracker } from "../session-tracker.js";

describe("SessionTracker", () => {
  let tracker: SessionTracker;

  beforeEach(() => {
    tracker = new SessionTracker();
  });

  it("record + get round trip", () => {
    tracker.record("pl-001", "dev", "session-abc");
    expect(tracker.get("pl-001", "dev")).toBe("session-abc");
  });

  it("returns undefined for non-existent key", () => {
    expect(tracker.get("pl-nonexistent", "dev")).toBeUndefined();
  });

  it("overwrites existing record for same pipeline+stage", () => {
    tracker.record("pl-001", "dev", "session-old");
    tracker.record("pl-001", "dev", "session-new");
    expect(tracker.get("pl-001", "dev")).toBe("session-new");
  });

  it("tracks different stages independently", () => {
    tracker.record("pl-001", "dev", "session-dev");
    tracker.record("pl-001", "cr", "session-cr");

    expect(tracker.get("pl-001", "dev")).toBe("session-dev");
    expect(tracker.get("pl-001", "cr")).toBe("session-cr");
  });

  it("tracks different pipelines independently", () => {
    tracker.record("pl-001", "dev", "session-1");
    tracker.record("pl-002", "dev", "session-2");

    expect(tracker.get("pl-001", "dev")).toBe("session-1");
    expect(tracker.get("pl-002", "dev")).toBe("session-2");
  });

  it("remove deletes a session record", () => {
    tracker.record("pl-001", "dev", "session-abc");
    const deleted = tracker.remove("pl-001", "dev");

    expect(deleted).toBe(true);
    expect(tracker.get("pl-001", "dev")).toBeUndefined();
  });

  it("remove returns false for non-existent key", () => {
    expect(tracker.remove("pl-nonexistent", "dev")).toBe(false);
  });

  it("clear removes all sessions", () => {
    tracker.record("pl-001", "dev", "s1");
    tracker.record("pl-002", "cr", "s2");
    tracker.clear();

    expect(tracker.get("pl-001", "dev")).toBeUndefined();
    expect(tracker.get("pl-002", "cr")).toBeUndefined();
    expect(tracker.size).toBe(0);
  });

  it("size reflects number of tracked sessions", () => {
    expect(tracker.size).toBe(0);

    tracker.record("pl-001", "dev", "s1");
    expect(tracker.size).toBe(1);

    tracker.record("pl-001", "cr", "s2");
    expect(tracker.size).toBe(2);

    tracker.remove("pl-001", "dev");
    expect(tracker.size).toBe(1);
  });
});
