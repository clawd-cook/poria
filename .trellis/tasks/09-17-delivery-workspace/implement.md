# Implement: Delivery workspace

Parent task. Implementation happens on children. Do not `task.py start` this parent unless doing integration review only.

## Child map

| Child slug | Owns | Depends on |
| --- | --- | --- |
| `desktop-ia` | R1, R2, R6 — four tabs, header auth, home kanban + existing detail, settings page | SSO UI already in AuthStatus |
| `repo-registry` | R5 — register, clone to `~/.poria/repos/<scope>/<repo>`, grouped list | `desktop-ia` nav slot |
| `demand-list` | R3 — Xingyun assigned-to-me list + search/page | `desktop-ia`; `09-17-sso-login` for cookie |
| `start-pipeline` | R4 — wizard + structured `submit_pipeline` + backend_context | `repo-registry` + `demand-list` |

Ordering is written here, not implied by the tree. After all four pass check, parent integration: walk 登记仓库 → 登录 → 拉需求 → 开始 → 看板详情.

## Validation

```bash
export NVM_DIR="$HOME/.nvm"
. "/opt/homebrew/opt/nvm/nvm.sh"
nvm use 24.20.0
node -v   # v24.20.0
pnpm -v   # 11.23.0
pnpm typecheck
cargo check --workspace
cargo test -p poria-infrastructure -- registered_repo
cargo test -p poria-channels -- xingyun
```

Manual: register `git@coding.jd.com:ls/ls-entrance.git`, confirm `~/.poria/repos/ls/ls-entrance`; login; demand list; start with FE+BE+PRD; card on home; detail back.

## Risky files

- `src/components/Shell.tsx` — nav rewrite
- `src/lib/types.ts` / `src/state/*` — ViewType and new collections
- `src-tauri/src/commands/pipeline.rs` — submit contract change
- `crates/poria-infrastructure/src/store/schema.rs` — v2 table
- `crates/poria-core/src/types/pipeline_types.rs` — `PipelineConfig` extras
- `crates/poria-commands/src/executor.rs` — must keep `repos` FE-only

## Rollback points

1. After IA only: restore three-tab Shell; settings Modal
2. After repos table: schema_version 2 unused if UI hidden
3. After submit contract: old pipelines still load; new field optional

## Ready gate

- Parent `prd.md` converged; this file + `design.md` present
- Children created and each has sliced `prd.md`
- `implement.jsonl` / `check.jsonl` have real spec/research rows
- User reviewed artifacts before any child `task.py start`
