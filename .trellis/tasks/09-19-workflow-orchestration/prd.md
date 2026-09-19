# Job/Step workflow engine

## Goal

Replace the hardcoded Init → ReviewPrd → Design → Dev → Cr → Deploy match with a GitHub Actions–style **Job graph + sequential Steps** workflow. Jobs remain the HITL / board / Xingyun writeback anchors. Steps are registered actions composed from YAML. The bundled `demand-to-mr` workflow must keep today’s demand → MR product behavior.

## Background

Desktop `execute_next_stage` switches on `StageEnum` and always runs the six `STAGE_ORDER` skills. Dev local verify is buried in `GenCodeSkill`; independent CR notes are buried in `DeploySkill`. The board step bar always paints six cells from `STAGE_ORDER`, not the pipeline’s persisted jobs.

## Decisions

- **D1** Orchestration is a Job DAG (`needs`) with serial Steps inside each Job. No matrix, no `${{ }}` expressions, no marketplace, no arbitrary `run: bash`.
- **D2** HITL, 看板 lanes, and Xingyun writeback stay on the Job (`stages` row). Step failure fails or blocks the whole Job via existing `ISSUE_POLICIES`.
- **D3** v1 Job ids are the existing six `StageEnum` values. YAML may omit a Job or add registered Steps. A seventh Job name is rejected at submit (no gate / writeback semantics).
- **D4** Default bundled workflow is `demand-to-mr`. `~/.poria/workflows/<id>.yml` may override the same id. Invalid `uses` or unknown Job ids are rejected at submit.
- **D5** Stamp workflow id + materialized jobs onto `pipeline.config` at submit. Pipelines missing those fields keep `STAGE_ORDER` (plus default extra steps) — no forced SQLite migration.
- **D6** One ready Job per `execute_next_stage` inside a pipeline (avoid double-writing the same worktree). Cross-pipeline parallelism is unchanged (`max_parallel_pipelines`).
- **D7** Backend repo stays out of `pipeline.repos`. `submit_pipeline` still requires `backendTrdUrl`.

## Requirements

- **R1** Parse and validate workflow YAML in `poria-core` (`WorkflowDocument`, `ready_jobs`, registered `uses`).
- **R2** Bundle `workflows/demand-to-mr.yml` with jobs matching today’s six stages; `dev` = `skill:gen-code` then `poria/dev-verify`; `deploy` = `skill:deploy` then `poria/post-cr-notes`.
- **R3** `submit_pipeline` materializes the workflow into `pipeline.config` and creates `stages` from that Job list (not a hardcoded six-row insert when jobs are present).
- **R4** Desktop `execute_next_stage` selects the next DAG-ready Job and runs its Steps through an Action registry in `src-tauri`. `poria-commands` must not depend on `poria-skills`.
- **R5** OutputGuard stays in `skill:gen-code`. Local verify and post-MR CR notes are registered builtin actions, not inlined in those skills.
- **R6** Fixture mode, HITL resume/skip/cancel, one CR→Dev regress, Deploy success → `waiting_merge` keep the same semantics.
- **R7** `StageProgress` renders `pipeline.stages` in persisted order. Known ids use `STAGE_LABELS`; unknown ids show the job id.

## Acceptance Criteria

- [ ] AC1: Bundled `demand-to-mr.yml` parses; `ready_jobs` is a linear chain equal to `STAGE_ORDER` when prior jobs are complete.
- [ ] AC2: Two jobs that `needs` the same upstream both become ready after that upstream completes; v1 still executes one Job per `execute_next_stage`.
- [ ] AC3: Submit stamps `workflow_id` + jobs; illegal `uses` / unknown job id is rejected; `~/.poria/workflows/demand-to-mr.yml` overrides the bundle.
- [ ] AC4: `execute_next_stage` has no six-way `match StageEnum` skill dispatch; each `uses` goes through the registry.
- [ ] AC5: Dev verify no longer runs inside `GenCodeSkill`; Deploy no longer posts CR notes inside `DeploySkill`. Job `output.steps[]` records id / uses / status / error.
- [ ] AC6: Legacy pipelines without `workflow_id` still run Init→Deploy including verify and CR notes.
- [ ] AC7: Board step bar follows materialized jobs, not a hardcoded six-cell `STAGE_ORDER.map`.
- [ ] AC8: `cargo test` covers parse/DAG/materialize and store create-from-jobs; `pnpm typecheck` passes after frontend changes.

## Out of scope

- Step-level HITL columns or a generic todo board.
- Free-string Job ids and historical DB migration of `StageEnum`.
- Visual workflow editor on Settings.
- Arbitrary YAML shell, matrix, or expressions.
- Intra-pipeline parallel Job execution on the same worktree.
