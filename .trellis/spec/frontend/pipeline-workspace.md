# Pipeline Init Workspace and Registered Repo Sync

> Init creates both worktrees before ReviewPrd/Design. Hosted clones track a recorded default branch. Captured 2026-09-18.

## 1. Scope / Trigger

Use this spec when changing `STAGE_ORDER`, `InitSkill` / `run_init_stage`, worktree create, `registered_repos` schema, `sync_repo`, `submit_pipeline` `base_branch`, ReviewPrd/Design `cwd`, or where `PRD.md` / `PRD_REVIEW.md` / `TRD.md` / `BACKEND_TRD.md` are written.

Do **not** put the backend repo in `pipeline.repos`. Do **not** commit project docs into the frontend git worktree. Do **not** verify on Vite `:1420`.

## 2. Signatures

Rust:

- `STAGE_ORDER`: `Init`, `ReviewPrd`, `Design`, `Dev`, `Cr`, `Deploy` (no `Workspace`)
- `WorktreeResource::create` — frontend `feature_<demand_code>` from `origin/<default_branch>`
- `WorktreeResource::create_detached` — backend `git worktree add --detach` at `~/.poria/worktrees/<pipeline_id>/<backend_name>`
- `git_sync_hosted_clone(path, default_branch)` — fetch + ff-only checkout of that branch on the hosted clone
- Schema v3 `registered_repos`: `default_branch TEXT NOT NULL DEFAULT 'master'`, `sync_status`, `last_synced_at`, `sync_error`
- Tauri: `sync_repo(id)`, `update_repo_default_branch(id, default_branch)`

Frontend (`src/lib/tauri.ts`): `syncRepo(id)`, `updateRepoDefaultBranch(id, defaultBranch)`.

`RegisteredRepo` JSON includes `default_branch`, `sync_status` (`idle` | `syncing` | `synced` | `failed`), `last_synced_at`, `sync_error`.

## 3. Contracts

| Layer | Owns |
| --- | --- |
| Init | JoySpace export to `~/.poria/projects/<demand_code>/` **then** sync both hosted clones **then** frontend feature worktree + backend detached worktree. Any failure fails Init. |
| `submit_pipeline` | `pipeline.repos` = frontend only. `base_branch` = registered `default_branch` (not `git_current_branch`). Backend stays in `backend_context`. After Init, `backend_context.local_path` is the **backend worktree**. |
| ReviewPrd / Design | Agent `cwd` = frontend worktree. Write `PRD_REVIEW.md` / `TRD.md` only under `~/.poria/projects/<demand_code>/`. |
| Deploy | Commit frontend worktree code only; strip leaked `PRD.md` / `PRD_REVIEW.md` / `TRD.md` / `BACKEND_TRD.md` from the worktree before `git add`. |
| Repo tab | Default branch defaults to `master`, editable. Save branch then sync immediately (sync fail does not roll back the field). Manual sync uses the same primitive. Clone success also syncs. |

Worktree paths: `~/.poria/worktrees/<pipeline_id>/<repo_name>`. Hosted clones: `~/.poria/repos/<scope>/<name>` stay on `default_branch` and are not a dev directory.

Backend worktree must be **detached** so it does not lock the same branch as the hosted clone.

## 4. Validation & Error Matrix

| Condition | What you see |
| --- | --- |
| Hosted clone dirty / cannot ff-only | `sync_status=failed`, `sync_error` set; Init fails if this is the pre-worktree sync |
| Save `default_branch` then sync fails | Branch field already saved; status failed; no rollback of the branch |
| Backend branch == hosted `default_branch` without detach | `git worktree add` fails (branch already checked out) — must use `--detach` |
| ReviewPrd before Init worktrees | Stage must fail before marking Running; no Workspace stage to wait on |
| Docs written into git root | Adopt into project dir / strip before Deploy; authority remains `~/.poria/projects` |

## 5. Good / Base / Bad Cases

- **Good**: Init completes → both worktrees exist → ReviewPrd writes `~/.poria/projects/<code>/PRD_REVIEW.md` and frontend worktree root has no that file. Repo list shows `master`, sync time, Sync button.
- **Base**: Existing `registered_repos` rows migrate to `default_branch=master`.
- **Bad**: `STAGE_ORDER` still has Workspace after Design; `base_branch` from current checkout; backend in `pipeline.repos`; Agent cwd = project dir only; commit `TRD.md` from the worktree.

## 6. Tests Required

- `cargo test -p poria-core -- stage_order`
- `cargo test -p poria-infrastructure -- registered_repo` / `schema_v3`
- `cargo test -p poria-resources -- worktree` / `git_sync`
- `cargo test -p poria-skills -- init` / `artifacts`
- `pnpm typecheck`
- Manual: `cargo tauri dev`, **Poria** window — repo sync + start pipeline Init before ReviewPrd (not `:1420`)

## 7. Wrong vs Correct

#### Wrong

```text
Init → ReviewPrd → Design → Workspace → Dev
submit_pipeline base_branch = git_current_branch(hosted clone)
backend_context.local_path = ~/.poria/repos/... after Init
git worktree add <path> master   # while hosted clone is on master
Write PRD_REVIEW.md into the frontend repo root and commit it
```

#### Correct

```text
Init → ReviewPrd → Design → Dev → Cr → Deploy
submit_pipeline base_branch = registered_repos.default_branch
backend_context.local_path = ~/.poria/worktrees/<pipeline_id>/<backend>
git worktree add --detach <path> origin/<wizard branch>
PRD.md / BACKEND_TRD.md / PRD_REVIEW.md / TRD.md only in ~/.poria/projects/<demand_code>/
```
