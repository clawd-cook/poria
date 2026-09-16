import fs from "node:fs";
import path from "node:path";
import readline from "node:readline";
import type Database from "better-sqlite3";
import type { PipelineEvent } from "@poria/core";

interface ArchivedPipelineRow {
  id: string;
  created_at: string;
}

interface EventRow {
  seq: number;
  pipeline_id: string;
  kind: string;
  payload: string;
  created_at: string;
}

export interface ArchiveResult {
  archivedCount: number;
}

/**
 * Archives events from completed pipelines to JSONL files
 * and removes them from SQLite to keep the database lean.
 */
export class EventArchiver {
  constructor(
    private readonly db: Database.Database,
    private readonly archiveBaseDir: string,
  ) {}

  /**
   * Archive events for pipelines that have been in a terminal state
   * (completed/cancelled/failed) longer than retentionDays.
   */
  async archive(retentionDays = 30): Promise<ArchiveResult> {
    const cutoff = new Date(
      Date.now() - retentionDays * 86400_000,
    ).toISOString();

    // Find all terminal pipelines past retention
    const archived = this.db
      .prepare(
        `SELECT id, created_at FROM pipelines
         WHERE status IN ('completed', 'cancelled', 'failed')
         AND updated_at < ?`,
      )
      .all(cutoff) as ArchivedPipelineRow[];

    for (const { id, created_at } of archived) {
      const events = this.db
        .prepare("SELECT * FROM events WHERE pipeline_id = ? ORDER BY seq ASC")
        .all(id) as EventRow[];

      if (events.length === 0) continue;

      // Month from pipeline.created_at (e.g., "2026-09")
      const month = created_at.slice(0, 7);
      const archiveDir = path.join(this.archiveBaseDir, month);
      fs.mkdirSync(archiveDir, { recursive: true });

      const archivePath = path.join(archiveDir, `events-${id}.jsonl`);
      const lines = events.map((e) => JSON.stringify(e));
      fs.writeFileSync(archivePath, lines.join("\n") + "\n", "utf-8");
    }

    // Delete archived events from SQLite (keep pipeline/stage metadata)
    if (archived.length > 0) {
      this.db
        .prepare(
          `DELETE FROM events WHERE pipeline_id IN (
             SELECT id FROM pipelines
             WHERE status IN ('completed','cancelled','failed')
             AND updated_at < ?
           )`,
        )
        .run(cutoff);
    }

    return { archivedCount: archived.length };
  }
}

/**
 * Replays events from either live SQLite or archived JSONL files.
 */
export class EventReplayService {
  constructor(
    private readonly db: Database.Database,
    private readonly archiveBaseDir: string,
  ) {}

  /**
   * Replay all events for a pipeline. Checks SQLite first,
   * falls back to archived JSONL files.
   */
  async replayAll(pipelineId: string): Promise<PipelineEvent[]> {
    // 1. Check live SQLite events table
    const liveEvents = this.db
      .prepare(
        "SELECT * FROM events WHERE pipeline_id = ? ORDER BY seq ASC",
      )
      .all(pipelineId) as EventRow[];

    if (liveEvents.length > 0) {
      return liveEvents.map(rowToEvent);
    }

    // 2. Fall back to archived JSONL
    const archivePath = await this.findArchiveFile(pipelineId);
    if (archivePath) {
      return this.readJsonl(archivePath);
    }

    return [];
  }

  /**
   * Scan archive directories for the JSONL file belonging to a pipeline.
   */
  async findArchiveFile(pipelineId: string): Promise<string | null> {
    if (!fs.existsSync(this.archiveBaseDir)) return null;

    const months = fs.readdirSync(this.archiveBaseDir);
    for (const month of months) {
      const candidate = path.join(
        this.archiveBaseDir,
        month,
        `events-${pipelineId}.jsonl`,
      );
      if (fs.existsSync(candidate)) {
        return candidate;
      }
    }

    return null;
  }

  /**
   * Read a JSONL file and parse each line as a PipelineEvent.
   */
  private async readJsonl(filePath: string): Promise<PipelineEvent[]> {
    const events: PipelineEvent[] = [];
    const fileStream = fs.createReadStream(filePath, "utf-8");
    const rl = readline.createInterface({
      input: fileStream,
      crlfDelay: Infinity,
    });

    for await (const line of rl) {
      const trimmed = line.trim();
      if (!trimmed) continue;
      const row = JSON.parse(trimmed) as EventRow;
      events.push(rowToEvent(row));
    }

    return events;
  }
}

function rowToEvent(row: EventRow): PipelineEvent {
  const parsed = JSON.parse(row.payload) as Record<string, unknown>;
  if (typeof parsed["timestamp"] === "string") {
    parsed["timestamp"] = new Date(parsed["timestamp"] as string);
  }
  return parsed as unknown as PipelineEvent;
}
