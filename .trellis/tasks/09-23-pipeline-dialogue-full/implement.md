# Implement plan: 09-23-pipeline-dialogue-full

**Locked**: full chain + stage-boundary dialogue (A). Do not `task.py start` until user approves.

## Checklist

### P0 — Core stop-and-talk

1. Extend `STAGE_ORDER` / profile in `poria-core` + TS types; migration aliases.
2. Executor: after each stage success → `awaiting_advance` + emit dialogue event (no auto next).
3. IPC `pipeline_advance` + wire HumanLoop / workbench 对话 actions.
4. Tests: state_machine / stage_order / no silent chain.
5. `cargo test -p poria-core` + `pnpm typecheck`.

### P1 — Full nodes (thin)

6. Skills stubs: `test_plan`, `test_cases`, `handoff_qa`, `archive` (markdown artifacts under projects or workspace).
7. `lint` node: run existing verify commands; fail → dialogue.
8. Map clarify/propose/implement/code_review/deploy to existing skills.
9. confirm-locked enforcement on listed gates.

### P2 — Workbench UX

10. DemandWorkbench：对话切面展示摘要气泡 + 操作按钮/输入.
11. Settings: never enable prod autopilot; fixture-only note in docs.
12. Update AGENTS.md / ARCHITECTURE.md stage table.
13. Desktop hand test AC1–AC5.

## Validation

```bash
nvm use 24.20.0
cargo test -p poria-core -- stage_order
cargo test -p poria-commands --
pnpm typecheck
# cargo tauri dev — stop/chat/continue
```

## Before start

- [x] Scope full + dialogue A
- [x] design / implement written
- [x] User review approve → `task.py start`
- [x] P0+P1 implement + check (residual: R6, desktop AC5)
