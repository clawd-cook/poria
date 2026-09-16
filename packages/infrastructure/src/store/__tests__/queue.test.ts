import { describe, it, expect, beforeEach } from "vitest";
import { initDatabase } from "../schema.js";
import { PipelineQueue, WorkerLock } from "../queue.js";
import type Database from "better-sqlite3";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

describe("PipelineQueue", () => {
  let db: Database.Database;
  let queue: PipelineQueue;

  beforeEach(() => {
    db = initDatabase(":memory:");
    queue = new PipelineQueue(db);
  });

  it("should enqueue and dequeue a pipeline", () => {
    queue.enqueue("pl-1");
    expect(queue.size()).toBe(1);

    const id = queue.dequeue();
    expect(id).toBe("pl-1");
    expect(queue.size()).toBe(0);
  });

  it("enqueue should be idempotent (duplicate pipelineId ignored)", () => {
    queue.enqueue("pl-1");
    queue.enqueue("pl-1"); // Should not throw
    expect(queue.size()).toBe(1);
  });

  it("dequeue should return null when empty", () => {
    const id = queue.dequeue();
    expect(id).toBeNull();
  });

  it("should dequeue by priority DESC, then enqueued_at ASC", () => {
    queue.enqueue("pl-low", 0);
    queue.enqueue("pl-high", 10);
    queue.enqueue("pl-med", 5);

    expect(queue.dequeue()).toBe("pl-high");
    expect(queue.dequeue()).toBe("pl-med");
    expect(queue.dequeue()).toBe("pl-low");
    expect(queue.dequeue()).toBeNull();
  });

  it("should dequeue atomically (item removed after dequeue)", () => {
    queue.enqueue("pl-1");
    queue.enqueue("pl-2");

    const first = queue.dequeue();
    expect(first).toBe("pl-1");
    expect(queue.size()).toBe(1);

    const second = queue.dequeue();
    expect(second).toBe("pl-2");
    expect(queue.size()).toBe(0);
  });

  it("size should reflect current queue state", () => {
    expect(queue.size()).toBe(0);
    queue.enqueue("pl-1");
    queue.enqueue("pl-2");
    expect(queue.size()).toBe(2);
    queue.dequeue();
    expect(queue.size()).toBe(1);
  });
});

describe("WorkerLock", () => {
  let lockDir: string;
  let lock: WorkerLock;

  beforeEach(() => {
    lockDir = fs.mkdtempSync(path.join(os.tmpdir(), "poria-lock-test-"));
    lock = new WorkerLock(lockDir);
  });

  it("should acquire lock when no lock exists", () => {
    expect(lock.acquireLock()).toBe(true);
    expect(lock.isLocked()).toBe(true);
  });

  it("should fail to acquire when another live process holds it", () => {
    // Write current PID (which is alive) to simulate another holder
    fs.writeFileSync(path.join(lockDir, "worker.lock"), String(process.pid));

    // Same PID is alive, so acquireLock should fail
    const lock2 = new WorkerLock(lockDir);
    expect(lock2.acquireLock()).toBe(false);
  });

  it("should reclaim lock from dead process", () => {
    // Write a PID that definitely doesn't exist
    fs.writeFileSync(path.join(lockDir, "worker.lock"), "99999999");

    expect(lock.acquireLock()).toBe(true);
  });

  it("should release lock", () => {
    lock.acquireLock();
    expect(lock.isLocked()).toBe(true);

    lock.releaseLock();
    expect(lock.isLocked()).toBe(false);
  });

  it("isLocked should return false when no lock file", () => {
    expect(lock.isLocked()).toBe(false);
  });
});
