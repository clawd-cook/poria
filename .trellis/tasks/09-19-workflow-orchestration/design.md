# Job/Step workflow engine — design

## Data flow

```
workflows/demand-to-mr.yml
  → parse/validate (poria-core)
  → optional ~/.poria/workflows override (poria-infrastructure)
  → materialize into PipelineConfig.jobs (submit_pipeline)
  → SQLite stages rows (one Job = one Stage)
  → execute_next_stage: ready_jobs DAG → sequential uses
  → ActionRegistry (src-tauri)
  → Skill or builtin
  → Stage.output.steps[] + HITL on the Job
  → StageProgress reads pipeline.stages order
```

## Types (`poria-core::workflow`)

- `WorkflowDocument { name, jobs: Vec<WorkflowJob> }` — YAML mapping order preserved.
- `WorkflowJob { id, needs, steps }`; `WorkflowStep { uses, id? }`.
- `MaterializedJob` / `MaterializedStep` stored on `PipelineConfig`.
- Allowed job ids = `STAGE_ORDER` snake_case. Allowed `uses` = registered action ids.
- `ready_jobs(doc, completed_ids)`: job is ready when every `needs` id is in `completed` (Completed or Skipped) and the job itself is not.
- Cycles, unknown needs, empty steps, unknown job id, unknown `uses` fail validation.

## Default workflow

Linear chain Init → … → Deploy. Extra steps only on `dev` and `deploy`. Builtin ids:

- `poria/dev-verify` — `run_frontend_verify` (fixture skips)
- `poria/post-cr-notes` — post P0/P1 MR notes (fixture skips; 401 fails Job; other post errors warn)

## Execution

Desktop is the only IPC runner. `execute_next_stage`:

1. Build a `WorkflowDocument` from `config.jobs`, else fallback to the bundled default (legacy).
2. Pick the first ready job in document order that exists on `pipeline.stages`.
3. Mark Job running (`skill_id` = first step `uses`).
4. Run each step via registry; append `output.steps[]`.
5. Step error → Job Failed/Blocked (`fail_or_block_stage` / Init cleanup unchanged).
6. After all steps: existing Job hooks (CR regress once, Deploy gates + `waiting_merge`).

Single-pipeline: one Job per call. Worker cross-pipeline parallelism unchanged.

## Registry

Assembled in `src-tauri` (`workflow_actions`). Maps:

| uses | implementation |
|------|----------------|
| `skill:init` … `skill:deploy` | existing skill + command-layer setup |
| `poria/dev-verify` | `poria_skills::run_frontend_verify` |
| `poria/post-cr-notes` | extracted from `DeploySkill` |

`poria-commands` stays free of `poria-skills`.

## Persistence

`SqlitePipelineStore::create` inserts `pipeline.stages` in order. Empty stages still fall back to `STAGE_ORDER` for existing tests / legacy creates.

## Frontend

`StageProgress` uses `stages` as given. `stageLabel(name)` = `STAGE_LABELS[name] ?? name`.
