# Design: 全链路 + 阶段边界对话（方案 A）

## Architecture

```text
Profile(full) → PipelineExecutor
  for node in pipeline:
    run skill / mechanical step   # may use claude -p
    persist artifacts
    emit awaiting_advance + dialogue payload
    WAIT user AdvanceDecision via IPC
    apply continue | redo | skip | annotate
  deploy → archive
```

Frontend: DemandWorkbench **对话** tab becomes primary for gates; 确认 tab can merge or alias.

## Node map (full)

| # | id | Skill / action | Exit gate |
|---|-----|----------------|-----------|
| 0 | `init` | existing Init | → dialogue then continue |
| 1 | `clarify` | ReviewPrd-like → PRD_REVIEW + context notes | confirm-locked |
| 2 | `propose` | Design-like → TRD (+ optional tasks.md) | confirm-locked |
| 3 | `test_plan` | new thin skill → test-plan.md | confirm-locked |
| 4 | `implement` | Dev / gen-code | dialogue |
| 5 | `lint` | typecheck/lint commands in worktree | dialogue (fail stays) |
| 6 | `code_review` | Cr skill | dialogue + feedback |
| 7 | `test_cases` | thin → test-cases.md | dialogue |
| 8 | `run_autotest` | optional script or manual certify | confirm-locked |
| 9 | `handoff_qa` | thin handoff-report.md | confirm-locked |
| 10 | `deploy` | existing Deploy | confirm-locked |
| 11 | `archive` | mark done + summary event | confirm-locked |

Legacy mapping: `review_prd→clarify`, `design→propose`, `dev→implement`, `cr→code_review`, insert lint/test_*/handoff/archive.

## Dialogue contract

**System message fields**: `pipelineId`, `nodeId`, `summary`, `artifactPaths[]`, `allowedActions[]`, `gateLevel`.

**User AdvanceDecision**:

| action | Effect |
|--------|--------|
| `continue` | advance to next node |
| `redo` | re-queue same node |
| `skip` | skip if `gateLevel != locked` OR second confirm |
| `annotate` | store note for next node prompt; stay waiting |

IPC sketch: `human_loop_respond` extended or new `pipeline_advance({ pipelineId, action, note? })`.

## Compatibility

- SQLite pipelines with old stage names: migrate on read or dual-write alias table in core.
- Fixture mode may auto-continue **only** when `PORIA_PIPELINE_FIXTURE=1`（dev），production path never.

## Trade-offs

| Choice | Why |
|--------|-----|
| A not B | Ships full chain without rewriting agent loop to sessions |
| Thin test nodes | Avoid Playwright hard dep; docs + optional commands first |
| Merge dialogue into workbench | Reuse 2.0.4 shell |

## Risky files

- `crates/poria-core` stage order / state machine
- `crates/poria-commands` executor
- `crates/poria-skills` new nodes
- `src-tauri` commands + `DemandWorkbench` / store HITL
