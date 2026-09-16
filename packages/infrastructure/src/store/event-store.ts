import type Database from "better-sqlite3";
import type { PipelineEvent } from "@poria/core";

interface EventRow {
  seq: number;
  pipeline_id: string;
  kind: string;
  payload: string;
  created_at: string;
}

function rowToEvent(row: EventRow): PipelineEvent {
  const parsed = JSON.parse(row.payload) as Record<string, unknown>;
  // Restore Date objects from ISO strings
  if (typeof parsed["timestamp"] === "string") {
    parsed["timestamp"] = new Date(parsed["timestamp"] as string);
  }
  return parsed as unknown as PipelineEvent;
}

/**
 * Read-only event store. Writes happen via SqlitePipelineStore.saveStageTx().
 */
export class EventStore {
  private readonly selectByPipeline: Database.Statement;
  private readonly selectByKind: Database.Statement;

  constructor(db: Database.Database) {
    this.selectByPipeline = db.prepare(
      "SELECT * FROM events WHERE pipeline_id = ? ORDER BY seq ASC",
    );
    this.selectByKind = db.prepare(
      "SELECT * FROM events WHERE pipeline_id = ? AND kind = ? ORDER BY seq ASC",
    );
  }

  /**
   * Query all events for a pipeline, ordered by seq ascending.
   */
  queryByPipeline(pipelineId: string): PipelineEvent[] {
    const rows = this.selectByPipeline.all(pipelineId) as EventRow[];
    return rows.map(rowToEvent);
  }

  /**
   * Query events for a pipeline filtered by event kind.
   */
  queryByKind(pipelineId: string, kind: string): PipelineEvent[] {
    const rows = this.selectByKind.all(pipelineId, kind) as EventRow[];
    return rows.map(rowToEvent);
  }
}
