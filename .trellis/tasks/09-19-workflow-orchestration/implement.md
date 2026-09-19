# Job/Step workflow engine — implement

## Order

1. Trellis PRD / design / implement (this task).
2. `poria-core` `workflow` module: parse, validate, `ready_jobs`, materialize, tests. Workspace `serde_yaml`.
3. Bundle `workflows/demand-to-mr.yml`; Tauri `bundle.resources`; load + override; stamp on `submit_pipeline`; store create uses materialized stages.
4. Action registry + `execute_next_stage` Job/Step runner (delete six-way skill `match`).
5. Move Dev verify and post-CR notes into registered actions; record `output.steps[]`.
6. `StageProgress` from materialized jobs.

## Validation

- `cargo test -p poria-core -- workflow`
- `cargo test -p poria-skills -- gen_code deploy post_cr`
- `cargo test -p poria-infrastructure -- pipeline_repo`
- `cargo test -p poria-desktop --lib -- --test-threads=1` if desktop tests cover submit/stages
- `pnpm typecheck` after `src/` changes
- `pnpm exec oxfmt .` after `src/` changes
