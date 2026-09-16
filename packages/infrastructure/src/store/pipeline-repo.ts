import type Database from "better-sqlite3";
import type {
  Pipeline,
  PipelineConfig,
  PipelineStatus,
  Stage,
  StageEnum,
  StageStatus,
  StageIssue,
  PipelineEvent,
} from "@poria/core";
import type { RollbackInstruction } from "@poria/core";
import { STAGE_ORDER } from "@poria/core";

// ── Row types for SQLite ──

interface PipelineRow {
  id: string;
  demand_id: number;
  demand_code: string;
  demand_name: string | null;
  status: string;
  raw_link: string;
  operator: string;
  has_regressed: number;
  config: string | null;
  created_at: string;
  updated_at: string;
}

interface StageRow {
  id: number;
  pipeline_id: string;
  name: string;
  status: string;
  skill_id: string | null;
  retry_count: number;
  max_retries: number;
  input: string | null;
  output: string | null;
  gate_results: string | null;
  issue: string | null;
  rollback: string | null;
  agent_session_id: string | null;
  started_at: string | null;
  completed_at: string | null;
}

// ── Serialization helpers ──

function toJson(value: unknown): string | null {
  if (value == null) return null;
  return JSON.stringify(value);
}

function fromJson<T>(value: string | null): T | undefined {
  if (value == null) return undefined;
  return JSON.parse(value) as T;
}

function toIso(date: Date | undefined): string | null {
  if (date == null) return null;
  return date.toISOString();
}

function fromIso(value: string | null): Date | undefined {
  if (value == null) return undefined;
  return new Date(value);
}

function rowToStage(row: StageRow): Stage {
  return {
    id: row.id,
    pipelineId: row.pipeline_id,
    name: row.name as StageEnum,
    status: row.status as StageStatus,
    skillId: row.skill_id ?? undefined,
    retryCount: row.retry_count,
    maxRetries: row.max_retries,
    input: fromJson<Record<string, unknown>>(row.input),
    output: fromJson<Record<string, unknown>>(row.output),
    gateResults: fromJson<Record<string, unknown>>(row.gate_results),
    issue: fromJson<StageIssue>(row.issue),
    rollback: fromJson<RollbackInstruction>(row.rollback),
    agentSessionId: row.agent_session_id ?? undefined,
    startedAt: fromIso(row.started_at),
    completedAt: fromIso(row.completed_at),
  };
}

function rowToPipeline(pRow: PipelineRow, stageRows: StageRow[]): Pipeline {
  const config = fromJson<PipelineConfig>(pRow.config) ?? {
    gates: [],
    trdScope: [],
    repos: [],
  };

  return {
    id: pRow.id,
    demandId: pRow.demand_id,
    demandCode: pRow.demand_code,
    demandName: pRow.demand_name ?? undefined,
    status: pRow.status as PipelineStatus,
    rawLink: pRow.raw_link,
    operator: pRow.operator,
    hasRegressed: pRow.has_regressed === 1,
    config,
    stages: stageRows.map(rowToStage),
    repos: config.repos,
    createdAt: new Date(pRow.created_at),
    updatedAt: new Date(pRow.updated_at),
  };
}

// ── Store ──

export class SqlitePipelineStore {
  private readonly insertPipeline: Database.Statement;
  private readonly insertStage: Database.Statement;
  private readonly selectPipeline: Database.Statement;
  private readonly selectStages: Database.Statement;
  private readonly selectByStatus: Database.Statement;
  private readonly updatePipelineStmt: Database.Statement;
  private readonly updateStageStmt: Database.Statement;
  private readonly insertEvent: Database.Statement;

  constructor(private readonly db: Database.Database) {
    this.insertPipeline = db.prepare(`
      INSERT INTO pipelines (id, demand_id, demand_code, demand_name, status, raw_link, operator, has_regressed, config, created_at, updated_at)
      VALUES (@id, @demand_id, @demand_code, @demand_name, @status, @raw_link, @operator, @has_regressed, @config, @created_at, @updated_at)
    `);

    this.insertStage = db.prepare(`
      INSERT INTO stages (pipeline_id, name, status, skill_id, retry_count, max_retries, input, output, gate_results, issue, rollback, agent_session_id, started_at, completed_at)
      VALUES (@pipeline_id, @name, @status, @skill_id, @retry_count, @max_retries, @input, @output, @gate_results, @issue, @rollback, @agent_session_id, @started_at, @completed_at)
    `);

    this.selectPipeline = db.prepare("SELECT * FROM pipelines WHERE id = ?");
    this.selectStages = db.prepare(
      "SELECT * FROM stages WHERE pipeline_id = ? ORDER BY id ASC",
    );
    this.selectByStatus = db.prepare("SELECT * FROM pipelines WHERE status = ?");

    this.updatePipelineStmt = db.prepare(`
      UPDATE pipelines SET
        status = @status,
        has_regressed = @has_regressed,
        config = @config,
        demand_name = @demand_name,
        updated_at = @updated_at
      WHERE id = @id
    `);

    this.updateStageStmt = db.prepare(`
      UPDATE stages SET
        status = @status,
        skill_id = @skill_id,
        retry_count = @retry_count,
        max_retries = @max_retries,
        input = @input,
        output = @output,
        gate_results = @gate_results,
        issue = @issue,
        rollback = @rollback,
        agent_session_id = @agent_session_id,
        started_at = @started_at,
        completed_at = @completed_at
      WHERE id = @id
    `);

    this.insertEvent = db.prepare(`
      INSERT INTO events (pipeline_id, kind, payload, created_at)
      VALUES (@pipeline_id, @kind, @payload, @created_at)
    `);
  }

  /**
   * Create a new Pipeline with its initial 7 stages.
   */
  create(pipeline: Pipeline): void {
    const now = new Date().toISOString();

    this.db.transaction(() => {
      this.insertPipeline.run({
        id: pipeline.id,
        demand_id: pipeline.demandId,
        demand_code: pipeline.demandCode,
        demand_name: pipeline.demandName ?? null,
        status: pipeline.status,
        raw_link: pipeline.rawLink,
        operator: pipeline.operator,
        has_regressed: pipeline.hasRegressed ? 1 : 0,
        config: toJson(pipeline.config),
        created_at: pipeline.createdAt.toISOString(),
        updated_at: now,
      });

      // Insert 7 initial stages in order
      for (const stageName of STAGE_ORDER) {
        const existing = pipeline.stages.find((s) => s.name === stageName);
        this.insertStage.run({
          pipeline_id: pipeline.id,
          name: stageName,
          status: existing?.status ?? "pending",
          skill_id: existing?.skillId ?? null,
          retry_count: existing?.retryCount ?? 0,
          max_retries: existing?.maxRetries ?? 3,
          input: toJson(existing?.input),
          output: toJson(existing?.output),
          gate_results: toJson(existing?.gateResults),
          issue: toJson(existing?.issue),
          rollback: toJson(existing?.rollback),
          agent_session_id: existing?.agentSessionId ?? null,
          started_at: toIso(existing?.startedAt),
          completed_at: toIso(existing?.completedAt),
        });
      }
    })();
  }

  /**
   * Load a Pipeline and all its stages.
   */
  load(pipelineId: string): Pipeline | undefined {
    const pRow = this.selectPipeline.get(pipelineId) as PipelineRow | undefined;
    if (!pRow) return undefined;

    const stageRows = this.selectStages.all(pipelineId) as StageRow[];
    return rowToPipeline(pRow, stageRows);
  }

  /**
   * Find all pipelines with the given status.
   */
  findByStatus(status: PipelineStatus): Pipeline[] {
    const pRows = this.selectByStatus.all(status) as PipelineRow[];
    return pRows.map((pRow) => {
      const stageRows = this.selectStages.all(pRow.id) as StageRow[];
      return rowToPipeline(pRow, stageRows);
    });
  }

  /**
   * Atomic transaction: update stage (optional) + pipeline + append events.
   * This is the ONLY write path for stage/pipeline/event mutations.
   */
  saveStageTx(
    stage: Stage | null,
    pipeline: Pipeline,
    events: PipelineEvent[],
  ): void {
    const now = new Date().toISOString();

    this.db.transaction(() => {
      if (stage) {
        this.updateStageStmt.run({
          id: stage.id,
          status: stage.status,
          skill_id: stage.skillId ?? null,
          retry_count: stage.retryCount,
          max_retries: stage.maxRetries,
          input: toJson(stage.input),
          output: toJson(stage.output),
          gate_results: toJson(stage.gateResults),
          issue: toJson(stage.issue),
          rollback: toJson(stage.rollback),
          agent_session_id: stage.agentSessionId ?? null,
          started_at: toIso(stage.startedAt),
          completed_at: toIso(stage.completedAt),
        });
      }

      this.updatePipelineStmt.run({
        id: pipeline.id,
        status: pipeline.status,
        has_regressed: pipeline.hasRegressed ? 1 : 0,
        config: toJson(pipeline.config),
        demand_name: pipeline.demandName ?? null,
        updated_at: now,
      });

      for (const event of events) {
        this.insertEvent.run({
          pipeline_id: event.pipelineId,
          kind: event.kind,
          payload: JSON.stringify(event),
          created_at: event.timestamp.toISOString(),
        });
      }
    })();
  }
}
