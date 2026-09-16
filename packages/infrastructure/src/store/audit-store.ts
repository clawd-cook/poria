import type Database from "better-sqlite3";

export type AuditAction =
  | "git_commit"
  | "git_push"
  | "mr_create"
  | "mr_merge"
  | "branch_delete";

export interface AuditEntry {
  pipelineId: string;
  stage?: string;
  action: AuditAction;
  operator: string;
  detail: Record<string, unknown>;
}

interface AuditRow {
  id: number;
  pipeline_id: string;
  stage: string | null;
  action: string;
  operator: string;
  detail: string | null;
  created_at: string;
}

function rowToEntry(row: AuditRow): AuditEntry & { id: number; createdAt: string } {
  return {
    id: row.id,
    pipelineId: row.pipeline_id,
    stage: row.stage ?? undefined,
    action: row.action as AuditAction,
    operator: row.operator,
    detail: row.detail ? (JSON.parse(row.detail) as Record<string, unknown>) : {},
    createdAt: row.created_at,
  };
}

export class AuditStore {
  private readonly insertStmt: Database.Statement;
  private readonly selectByPipeline: Database.Statement;

  constructor(db: Database.Database) {
    this.insertStmt = db.prepare(`
      INSERT INTO audit_log (pipeline_id, stage, action, operator, detail, created_at)
      VALUES (@pipeline_id, @stage, @action, @operator, @detail, @created_at)
    `);

    this.selectByPipeline = db.prepare(
      "SELECT * FROM audit_log WHERE pipeline_id = ? ORDER BY created_at ASC",
    );
  }

  /**
   * Record an audit entry.
   */
  record(entry: AuditEntry): void {
    this.insertStmt.run({
      pipeline_id: entry.pipelineId,
      stage: entry.stage ?? null,
      action: entry.action,
      operator: entry.operator,
      detail: JSON.stringify(entry.detail),
      created_at: new Date().toISOString(),
    });
  }

  /**
   * Query all audit entries for a pipeline, ordered by created_at ascending.
   */
  queryByPipeline(
    pipelineId: string,
  ): Array<AuditEntry & { id: number; createdAt: string }> {
    const rows = this.selectByPipeline.all(pipelineId) as AuditRow[];
    return rows.map(rowToEntry);
  }
}
