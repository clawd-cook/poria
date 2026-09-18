# Pipeline Init Workspace and Registered Repo Sync

> Init exports JoySpace docs, then builds `~/.poria/workspaces/<pipeline_id>/` (doc + skill symlinks, frontend feature worktree, backend detached worktree) before ReviewPrd/Design. Hosted clones track a recorded default branch. Captured 2026-09-18, updated 2026-09-18 for Claude skills.

## 1. Scope / Trigger

Use this spec when changing `STAGE_ORDER`, `InitSkill` / `run_init_stage`, `prepare_pipeline_workspace`, `WorktreeResource`, `registered_repos` schema, `sync_repo`, `submit_pipeline` `base_branch`, ReviewPrd/Design/Dev/Cr Agent `cwd`, bundled `skills/<name>/SKILL.md`, or where `PRD.md` / `PRD_REVIEW.md` / `TRD.md` / `BACKEND_TRD.md` are written.

Do **not** put the backend repo in `pipeline.repos`. Do **not** commit project docs into the frontend git worktree. Do **not** verify on Vite `:1420`. Do **not** keep a dual track of old `prompts/*.md` dumped into `--system-prompt`.

## 2. Signatures

Rust:

- `STAGE_ORDER`: `Init`, `ReviewPrd`, `Design`, `Dev`, `Cr`, `Deploy` (no `Workspace`)
- Workspace root: `~/.poria/workspaces/<pipeline_id>/`
- `WorktreeResource::create` — frontend `feature_<demand_code>` from `origin/<default_branch>` at `workspaces/<pipeline_id>/<frontend_name>`
- `WorktreeResource::create_detached` — backend `git worktree add --detach` at `workspaces/<pipeline_id>/<backend_name>`
- `prepare_pipeline_workspace` — six artifact symlinks into projects; four skill symlinks from bundled `skills/` into `.claude/skills/`; write `CLAUDE.md`
- Bundled skills: repo `skills/review-prd|gen-trd|gen-code|code-review/SKILL.md` (dev) or Tauri resource `skills/` (release)
- `git_sync_hosted_clone(path, default_branch)` — fetch + ff-only checkout of that branch on the hosted clone
- Schema v3 `registered_repos`: `default_branch TEXT NOT NULL DEFAULT 'master'`, `sync_status`, `last_synced_at`, `sync_error`
- Tauri: `sync_repo(id)`, `update_repo_default_branch(id, default_branch)`, `open_workspace(pipeline_id)`

Frontend (`src/lib/tauri.ts`): `syncRepo(id)`, `updateRepoDefaultBranch(id, defaultBranch)`, `openWorkspace(pipelineId)`.

`RegisteredRepo` JSON includes `default_branch`, `sync_status` (`idle` | `syncing` | `synced` | `failed`), `last_synced_at`, `sync_error`.

`PipelineDetail.workspace_path` comes from Init output `workspacePath`.

## 3. Contracts

| Layer | Owns |
| --- | --- |
| Init | JoySpace export to `~/.poria/projects/<demand_code>/` **then** sync both hosted clones **then** mkdir workspace, symlink docs + skills, write `CLAUDE.md`, frontend feature worktree + backend detached worktree. Any failure fails Init and deletes the workspace dir (not projects). |
| `submit_pipeline` | `pipeline.repos` = frontend only. `base_branch` = registered `default_branch` (not `git_current_branch`). Backend stays in `backend_context`. After Init, `backend_context.local_path` is the **backend worktree**. |
| ReviewPrd / Design / Dev / Cr | Agent `cwd` = workspace root. `claude -p` is a short prompt that names `review-prd` / `gen-trd` / `gen-code` / `code-review`. Write docs to workspace-root filenames (symlinks to projects). Code only in the frontend worktree. Design entry evaluates **only** `prd_review_p0` (not `trd_exists` / `code_changes_exist`). Unanswered P0 (blank / TODO / 待填写 / 待确认) → pipeline `blocked` + `requirement_ambiguous`; ReviewPrd itself still completes. P1/P2 unanswered warn only. Desktop `write_demand_project_file` may save `PRD_REVIEW.md`; HITL resume re-checks P0. JME send is best-effort placeholder. |
| Deploy | Commit frontend worktree code only; strip leaked `PRD.md` / `PRD_REVIEW.md` / `TRD.md` / `BACKEND_TRD.md` from the worktree before `git add`. cwd remains the frontend git worktree. |
| Repo tab | Default branch defaults to `master`, editable. Save branch then sync immediately (sync fail does not roll back the field). Manual sync uses the same primitive. Clone success also syncs. |
| Sidebar 工作区 | Shows selected pipeline `workspacePath`; Finder via opener. Empty if no pipeline selected. |
| Sidebar 技能 | `list_skills` / `get_skill` scan bundled `skills/*/SKILL.md` only. `id` is the directory name (`review-prd`); `name`/`description` come from YAML; `markdown` is the body without frontmatter. Init / Deploy are pipeline stages, not skills. Drawer portals to `document.body` and scrolls only on the Drawer body. |

Worktree paths: `~/.poria/workspaces/<pipeline_id>/<repo_name>`. Hosted clones: `~/.poria/repos/<scope>/<name>` stay on `default_branch` and are not a dev directory.

Backend worktree must be **detached** so it does not lock the same branch as the hosted clone.

## 4. Validation & Error Matrix

| Condition | What you see |
| --- | --- |
| Hosted clone dirty / cannot ff-only | `sync_status=failed`, `sync_error` set; Init fails if this is the pre-worktree sync |
| Save `default_branch` then sync fails | Branch field already saved; status failed; no rollback of the branch |
| Backend branch == hosted `default_branch` without detach | `git worktree add` fails (branch already checked out) — must use `--detach` |
| ReviewPrd before Init worktrees | Stage must fail before marking Running; no Workspace stage to wait on |
| Design with unanswered P0 in `PRD_REVIEW.md` | Design `blocked` (`requirement_ambiguous` / `P0 unanswered`); HITL 填写 P0 答案; P1/P2 仅提醒 |
| Design with no `PRD_REVIEW.md` | Same P0 block; do not start `gen-trd` |
| Docs written into git root | Adopt into project dir / strip before Deploy; authority remains `~/.poria/projects` |
| Missing bundled `SKILL.md` | Init fails: 随包 skill 不完整 / 找不到随包 skill 目录 |
| Init fails after `git worktree add` | `git worktree remove` both paths **then** delete the workspace dir. Bare `rm -rf` leaves a stale worktree in the hosted clone |
| Deploy `strip_project_docs` on workspace-visible docs | Skip symlink entries (`symlink_metadata`); `is_file()` follows the link and would delete projects docs |
| Short `-p` missing URLs | Append `backend_trd_url` and frontend `base_branch` when non-empty so SKILL.md can open backend TRD and `git diff <base>` |

## 5. Good / Base / Bad Cases

- **Good**: Init completes → `~/.poria/workspaces/<id>/` has `CLAUDE.md`, six doc symlinks, `.claude/skills/{review-prd,gen-trd,gen-code,code-review}`, both worktrees. ReviewPrd cwd is the workspace root; `PRD_REVIEW.md` lands in projects via symlink. Sidebar 工作区 opens Finder.
- **Base**: Existing `registered_repos` rows migrate to `default_branch=master`.
- **Bad**: `STAGE_ORDER` still has Workspace after Design; `base_branch` from current checkout; backend in `pipeline.repos`; Agent cwd = frontend worktree or project dir only; dump old prompt templates into `--system-prompt`; commit `TRD.md` from the worktree; worktrees still under `~/.poria/worktrees`; `list_skills` enumerates Rust `Skill` trait (Init / Deploy).

## 6. Tests Required

- `cargo test -p poria-core -- prd_review`
- `cargo test -p poria-core -- stage_order`
- `cargo test -p poria-infrastructure -- registered_repo` / `schema_v3` / `workspace`
- `cargo test -p poria-resources -- worktree` / `git_sync`
- `cargo test -p poria-skills` (init / artifacts / workspace_layout / claude_prompt)
- `cargo test -p poria-commands -- init_rollback`
- `cargo test -p poria-desktop --lib -- --test-threads=1`
- `pnpm typecheck`
- Manual: `cargo tauri dev`, **Poria** window — Init then sidebar 工作区 (not `:1420`)

## 7. Wrong vs Correct

#### Wrong

```text
Init → ReviewPrd → Design → Workspace → Dev
submit_pipeline base_branch = git_current_branch(hosted clone)
backend_context.local_path = ~/.poria/repos/... after Init
git worktree add <path> master   # while hosted clone is on master
Agent cwd = frontend worktree; --system-prompt = old prompts/*.md
Write PRD_REVIEW.md into the frontend repo root and commit it
~/.poria/worktrees/<pipeline_id>/<repo>
Init fail: rm -rf workspace without git worktree remove
strip_project_docs uses is_file() on a symlink to PRD.md
```

#### Correct

```text
Init → ReviewPrd → Design → Dev → Cr → Deploy
submit_pipeline base_branch = registered_repos.default_branch
backend_context.local_path = ~/.poria/workspaces/<pipeline_id>/<backend>
git worktree add --detach <path> origin/<wizard branch>
Agent cwd = ~/.poria/workspaces/<pipeline_id>
claude -p names review-prd / gen-trd / gen-code / code-review
PRD.md / BACKEND_TRD.md / PRD_REVIEW.md / TRD.md only in ~/.poria/projects/<demand_code>/
  (visible in the workspace via symlinks)
Init fail: git worktree remove frontend+backend, then delete workspace dir
strip_project_docs skips symlinks (symlink_metadata)
short -p includes backend_trd_url and frontend base_branch when set
```
