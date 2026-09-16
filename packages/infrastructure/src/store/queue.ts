import fs from "node:fs";
import path from "node:path";
import type Database from "better-sqlite3";

/**
 * Serial execution queue backed by SQLite.
 * Pipelines are dequeued by priority DESC, then enqueued_at ASC.
 */
export class PipelineQueue {
  private readonly enqueueStmt: Database.Statement;
  private readonly dequeueSelect: Database.Statement;
  private readonly dequeueDelete: Database.Statement;
  private readonly sizeStmt: Database.Statement;
  private readonly dequeueTx: Database.Transaction<() => string | null>;

  constructor(db: Database.Database) {
    this.enqueueStmt = db.prepare(`
      INSERT OR IGNORE INTO queue (pipeline_id, priority, enqueued_at)
      VALUES (?, ?, datetime('now'))
    `);

    this.dequeueSelect = db.prepare(
      "SELECT pipeline_id FROM queue ORDER BY priority DESC, enqueued_at ASC LIMIT 1",
    );

    this.dequeueDelete = db.prepare(
      "DELETE FROM queue WHERE pipeline_id = ?",
    );

    this.sizeStmt = db.prepare("SELECT COUNT(*) as count FROM queue");

    // Atomic dequeue: SELECT + DELETE in one transaction
    this.dequeueTx = db.transaction(() => {
      const row = this.dequeueSelect.get() as
        | { pipeline_id: string }
        | undefined;
      if (!row) return null;
      this.dequeueDelete.run(row.pipeline_id);
      return row.pipeline_id;
    });
  }

  /**
   * Enqueue a pipeline. Idempotent: duplicate pipeline_id is silently ignored.
   */
  enqueue(pipelineId: string, priority = 0): void {
    this.enqueueStmt.run(pipelineId, priority);
  }

  /**
   * Atomically dequeue the highest-priority, oldest pipeline.
   * Returns null if queue is empty.
   */
  dequeue(): string | null {
    return this.dequeueTx();
  }

  /**
   * Number of items currently in the queue.
   */
  size(): number {
    const row = this.sizeStmt.get() as { count: number };
    return row.count;
  }
}

/**
 * File-based single-instance lock for the pipeline worker.
 * Uses PID to detect and reclaim stale locks from dead processes.
 */
export class WorkerLock {
  private lockPath: string;

  constructor(lockDir: string) {
    this.lockPath = path.join(lockDir, "worker.lock");
  }

  /**
   * Attempt to acquire the worker lock.
   * Returns true if lock was acquired, false if another live process holds it.
   */
  acquireLock(): boolean {
    // Ensure directory exists
    const dir = path.dirname(this.lockPath);
    fs.mkdirSync(dir, { recursive: true });

    if (fs.existsSync(this.lockPath)) {
      // Check if the PID in the lock file is still alive
      try {
        const content = fs.readFileSync(this.lockPath, "utf-8").trim();
        const pid = parseInt(content, 10);
        if (!isNaN(pid) && isProcessAlive(pid)) {
          return false; // Another live worker holds the lock
        }
        // Dead process — reclaim lock
      } catch {
        // Corrupt lock file — reclaim
      }
    }

    // Write our PID
    fs.writeFileSync(this.lockPath, String(process.pid), "utf-8");
    return true;
  }

  /**
   * Release the worker lock.
   */
  releaseLock(): void {
    try {
      if (fs.existsSync(this.lockPath)) {
        fs.unlinkSync(this.lockPath);
      }
    } catch {
      // Best-effort cleanup
    }
  }

  /**
   * Check if the lock is currently held by a live process.
   */
  isLocked(): boolean {
    if (!fs.existsSync(this.lockPath)) return false;
    try {
      const content = fs.readFileSync(this.lockPath, "utf-8").trim();
      const pid = parseInt(content, 10);
      return !isNaN(pid) && isProcessAlive(pid);
    } catch {
      return false;
    }
  }
}

function isProcessAlive(pid: number): boolean {
  try {
    // signal 0 does not kill the process, just checks existence
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}
