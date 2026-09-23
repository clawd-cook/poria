# AGENTS.md

Poria is an AI-native delivery platform: a Tauri v2 macOS desktop app that turns a Xingyun demand into a Coding merge request. Frontend is React 19 + TypeScript + Vite + Tailwind CSS + shadcn/ui. Backend is Rust, a Cargo workspace of six domain crates plus the Tauri shell (`poria-desktop`).

This file is for coding agents. Human product docs live in `README.md`. Architecture overview lives in `ARCHITECTURE.md`. Contribution process lives in `CONTRIBUTING.md`.

Pinned frontend toolchain (also in `package.json` / CI `env`): Node.js **24.20.0**, pnpm **11.23.0**. App identifier: `com.poria.desktop`. Window title: `Poria`. Process: `poria-desktop`.

## Architecture

```
poria/
├── src/                      # React frontend (TSX, hooks, state, components)
│   ├── components/           # Pages, pipeline UI, and `ui/` shadcn primitives
│   ├── hooks/                # usePipeline, useTauriEvents
│   ├── lib/                  # types + invoke wrappers (`tauri.ts`)
│   ├── state/                # reducer store + actions
│   └── styles.css            # Tailwind v4 tokens, fonts, markdown, reduced-motion
├── src-tauri/                # Tauri v2 shell (crate name: poria-desktop)
│   └── src/commands/         # IPC handlers (the only Tauri command layer)
├── crates/
│   ├── poria-core/           # Types, state machine, gates, risk, contracts, artifacts
│   ├── poria-infrastructure/ # Auth, config, metrics, SQLite
│   ├── poria-commands/       # Pipeline executor, error classification, rollback
│   ├── poria-resources/      # Claude agent pool, terminal, worktree, output guard
│   ├── poria-skills/         # Stage skills (Init … Deploy), HITL, fixtures
│   └── poria-channels/       # Coding, JoySpace, Xingyun, JME, defect
├── skills/                   # Bundled Claude SKILL.md (review-prd, gen-trd, gen-code, code-review)
├── public/                   # Logos / static assets
└── submodules/               # Reference trees — do not modify files inside
```

Frontend alias: `@/*` → `./src/*` (`tsconfig.json`; Vite does not need a special alias beyond this).

Shell views (`src/components/Shell.tsx`): 需求 `home`（列表 ↔ 需求工作台）、仓库 `repos`、设置 `settings`. 渠道 / 技能 / 独立工作区已移出主航道（工作区并入工作台切面）. Tabs stay mounted via `PersistentTab`.

### Crate dependency direction (do not invert)

```
poria-core                    # no other poria crates
  ├── poria-infrastructure
  ├── poria-resources
  └── poria-channels
        └── poria-skills      # also depends on poria-resources
poria-commands                # poria-core + poria-infrastructure only
src-tauri (poria-desktop)     # wires all crates + IPC
```

Cargo workspace members are listed in root `Cargo.toml`. Shared Rust deps live in `[workspace.dependencies]`. Add versions there, then `{ workspace = true }` in crate manifests.

- Put domain types and state-machine rules in `poria-core`.
- Put HTTP / git-host / JoySpace / Xingyun clients in `poria-channels`.
- Put Claude CLI / git worktree / shell in `poria-resources`.
- Put stage behavior in `poria-skills`.
- Put orchestrate / retry / rollback in `poria-commands`.
- Put `#[tauri::command]` only in `src-tauri/src/commands/`. `poria-commands` is **not** the IPC layer.

### Pipeline stages → skills

`STAGE_ORDER` is the full-chain profile (12 nodes). There is **no** `Workspace` stage. Init creates the workspace and both worktrees before Clarify. `WorkspaceSkill` in `crates/poria-skills/src/workspace.rs` is leftover and is **not** in `STAGE_ORDER` — do not add it back.

After each successful node the executor stops with `awaiting_advance` until `pipeline_advance` (`continue` / `redo` / `skip` / `annotate`). Production never silently auto-chains; only `PORIA_PIPELINE_FIXTURE=1` may chain.

| Rust variant | JSON id | Skill id | Notes |
| ------------ | ------------------- | ------------------- | ----- |
| `Init` | `init` | `skill:init` | Docs + workspace + worktrees |
| `ReviewPrd` | `clarify` (alias `review_prd`) | `skill:review-prd` | `PRD_REVIEW.md` |
| `Design` | `propose` (alias `design`) | `skill:gen-trd` | frontend `TRD.md` |
| `TestPlan` | `test_plan` | `skill:test-plan` | thin `test-plan.md` |
| `Dev` | `implement` (alias `dev`) | `skill:gen-code` | codegen + `TASK.md` |
| `Lint` | `lint` | `skill:lint` | worktree verify; fail stays in dialogue |
| `Cr` | `code_review` (alias `cr`) | `skill:code-review` | `CR.md` + gates |
| `TestCases` | `test_cases` | `skill:test-cases` | thin `test-cases.md` |
| `RunAutotest` | `run_autotest` | `skill:run-autotest` | thin `test-report.md` |
| `HandoffQa` | `handoff_qa` | `skill:handoff-qa` | thin `handoff-report.md` |
| `Deploy` | `deploy` | `skill:deploy` | push → EasyCI SELECT → MR |
| `Archive` | `archive` | `skill:archive` | thin `archive.md`; then `waiting_merge` if MR exists |

Map source of truth: `crates/poria-skills/src/stage_skill_map.rs` and `crates/poria-commands/src/traits.rs` (`stage_skill_id`). Keep both in sync with `STAGE_ORDER` in `crates/poria-core/src/types/pipeline_types.rs` and `src/lib/types.ts`.

JSON / TS stage names are full-profile snake_case (`clarify`, `propose`, `implement`, `code_review`, …). Legacy six-stage ids still deserialize.

Artifacts (`poria-core` `feature_context`): `PRD.md`, `PRD_REVIEW.md`, `TRD.md`, `BACKEND_TRD.md`, `TASK.md`, `CR.md` (+ thin test/handoff/archive markdown). Production Init uses `FeatureContext::create_at` on `~/.poria/projects/<demand_code>/`. Frontend TRD and backend TRD are different files — do not collapse them. Do not commit these files into the git worktree.

Desktop agent stages (Clarify / Propose / Implement / CodeReview) must stay non-interactive (`claude -p`). Do not add HITL prompts inside those skills. Desktop HITL is `HumanLoopCard` → `human_loop_respond` / `pipeline_advance`.

Bundled Claude skills (sidebar 技能 + Init symlinks): repo `skills/{review-prd,gen-trd,gen-code,code-review}/SKILL.md`. Init and Deploy are Rust pipeline skills, **not** bundled `SKILL.md`. `list_skills` / `get_skill` scan bundled markdown only. Keep `--system-prompt` short; procedure lives in `SKILL.md`.

Fixture bypass: `PORIA_PIPELINE_FIXTURE=1` (`crates/poria-skills/src/fixture.rs`). Do not enable this in production paths.

## Runtime (hard constraint)

Develop and run this repo **only** with the **local nvm Node.js 24.20.0** install. Do not use any other Node version or isolated environment.

Pinned version: **v24.20.0**

`.node-version` is `24` (major only). Always pass the patch version explicitly (`nvm use 24.20.0`). Do not rely on `.node-version` to select 24.20.0.

Before any install, lint, typecheck, or app scripts:

```bash
export NVM_DIR="$HOME/.nvm"
[ -s "/opt/homebrew/opt/nvm/nvm.sh" ] && . "/opt/homebrew/opt/nvm/nvm.sh"
nvm use 24.20.0
```

Confirm both of these before continuing:

```bash
node -v          # must print v24.20.0
which node       # must be $HOME/.nvm/versions/node/v24.20.0/bin/node
```

If `node -v` is not `v24.20.0`, **stop**. Do not fall back.

`nvm use 24.20.0` in the **current shell** is required and does not need extra confirmation. Do **not** change nvm's default alias, install another Node, or edit shell rc files to make 24 global.

### Do not use

- Homebrew Node (`/opt/homebrew/bin/node`, may be newer than 24)
- Other nvm versions on this machine (anything except `v24.20.0`)
- Docker, Dev Containers, Nix, asdf, fnm, volta, n, or cloud/CI sandboxes for local work
- `npm`, `yarn`, or `bun` for dependency install (`packageManager` is pnpm)
- A pnpm version other than **11.23.0**
- Introducing a second runtime or changing the pinned Node without an explicit user request

## Package manager (hard constraint)

Use **pnpm 11.23.0** only. Source of truth is root `package.json`:

- `"packageManager": "pnpm@11.23.0"`
- `"devEngines.packageManager": { "name": "pnpm", "version": "11.23.0", "onFail": "download" }`

After `nvm use 24.20.0`, confirm:

```bash
pnpm -v          # must print 11.23.0
```

If `pnpm -v` is not `11.23.0`, **stop**. Do not fall back to another pnpm, npm, yarn, or bun. Do not change `packageManager` / `devEngines` without an explicit user request.

Corepack (or `devEngines` `onFail: "download"`) may fetch this exact pnpm when missing; do not install a different global pnpm to "make it work".

## Setup Commands

```bash
# 1. Activate Node
export NVM_DIR="$HOME/.nvm"
. "/opt/homebrew/opt/nvm/nvm.sh"
nvm use 24.20.0

# 2. Verify toolchain
node -v          # v24.20.0
pnpm -v          # 11.23.0

# 3. Install frontend dependencies
pnpm install

# 4. Verify Rust toolchain (for backend work)
rustup show      # stable toolchain required
cargo check --workspace
```

Do not run `npm install`, `yarn`, or `bun install`. Lockfiles: `pnpm-lock.yaml`, `Cargo.lock`.

## Development Workflow

### Frontend only (Vite)

```bash
pnpm dev         # http://localhost:1420 — Vite only, no Tauri IPC
```

Do **not** claim login, demand list, clone, or pipeline submit works from a Cursor browser tab on `:1420`. Those require `invoke` / `listen` in the `poria-desktop` window.

### Full desktop app (Tauri)

This repo does **not** depend on `@tauri-apps/cli`. `package.json` `"tauri": "tauri"` only works if a `tauri` binary is already on `PATH`. The reliable local command is:

```bash
export NVM_DIR="$HOME/.nvm"
. "/opt/homebrew/opt/nvm/nvm.sh"
nvm use 24.20.0
export PATH="$HOME/.nvm/versions/node/v24.20.0/bin:$HOME/.cargo/bin:$PATH"
cargo tauri dev   # Rust backend + Vite + native window (title: Poria, process: poria-desktop)
```

`tauri.conf.json` `beforeDevCommand` is `pnpm dev`, so Node 24.20.0 + pnpm 11.23.0 must be on `PATH` before `cargo tauri dev`. Do not add `@tauri-apps/cli` without an explicit user request.

Port **1420 is strict** (`vite.config.ts` `strictPort: true`). Vite watches `src/` and ignores `src-tauri/`. If 1420 is taken, free it before `tauri dev`.

Prefer testing the debug binary (`target/debug/poria-desktop`), not a stale `/Applications/Poria.app`.

Before claiming clone / SSO / Xingyun / `submit_pipeline` works, follow `.trellis/spec/frontend/tauri-desktop-testing.md`.

### TypeScript

```bash
pnpm typecheck   # tsc -b
```

There is no ESLint. Format frontend with oxfmt (see Code Style).

### Production build

```bash
pnpm build                     # tsc -b && vite build → dist/
cargo tauri build              # desktop bundle (needs cargo-tauri)
cargo tauri build --target aarch64-apple-darwin
cargo tauri build --target x86_64-apple-darwin
```

## Testing

### Rust

Tests live **in the crate sources** (`#[cfg(test)]` / `#[tokio::test]`), not a separate `tests/` tree. Cover all six domain crates when the change spans them. `src-tauri` lib tests: `cargo test -p poria-desktop --lib -- --test-threads=1`.

```bash
cargo test --workspace
cargo test -p poria-core
cargo test -p poria-core -- state_machine
cargo test -p poria-core -- stage_order
cargo test -p poria-infrastructure -- registered_repo
cargo test -p poria-resources -- worktree
cargo test -p poria-skills
cargo test -p poria-commands -- init_rollback
cargo check --workspace
cargo clippy --workspace
```

Add or update tests for Rust you change. No coverage gate is configured. No repo `rustfmt.toml` — use default `rustfmt`.

### Frontend

No frontend test runner. After UI/IPC changes: `pnpm typecheck`, then exercise the **desktop** window. `pnpm exec oxfmt .` for format.

## Code Style

### TypeScript / React

- Formatter: `pnpm exec oxfmt .` (`.oxfmtrc.json`: sort imports, sort object keys, sort Tailwind classes; ignores `.claude`, `.trellis`, `submodules`)
- Strict TS: `noUnusedLocals`, `noUnusedParameters`, `noFallthroughCasesInSwitch`; target ES2022; JSX `react-jsx` (no default React import)
- ESM (`"type": "module"`)
- UI: Tailwind CSS v4 + shadcn/ui (Radix primitives in `src/components/ui/`). Light theme only. Restaurant-warm tokens in `src/styles.css`: primary `#DC2626` for CTAs only, accent gold `#A16207`, canvas `#FAFAF8`, sidebar `#EBE8E3` (not white), cards `#FFFFFF`, foreground `#1C1917`. Do not wash the shell in red-50 or yellow cream. Headings: Playfair Display SC (`font-serif`); body: Karla (`font-sans`). Icons: `lucide-react`. Toasts: `sonner`. Markdown: `react-markdown` via `MarkdownView`. Pages wrap in `PageFrame`. Prefer semantic tokens (`bg-primary`, `text-muted-foreground`) over raw hex in components. Design source: `design-system/poria/MASTER.md`.
- CSS: Tailwind utilities + `src/styles.css` tokens, hidden scrollbars, `prefers-reduced-motion`. Do not reintroduce Ant Design.
- Global state: `src/state/store.tsx` reducer + `src/state/actions.ts`. Do not introduce a second store.
- IPC: wrap `invoke` in `src/lib/tauri.ts`. Tauri v2 maps JS **camelCase** args to Rust snake_case (`pipelineId` → `pipeline_id`, `gitUrl` → `git_url`, `backendTrdUrl` → `backend_trd_url`).
- Events (`listen` in `useTauriEvents` / `store.tsx`): `pipeline:list-changed`, `pipeline:created`, `pipeline:updated`, `stage:progress`, `sidecar:status`, `auth:status-changed`, `repo:updated`, plus stream chunks. Do not call `invoke`/`listen` from a browser tab on `:1420`.

### Rust

- Edition 2021; no repo `rustfmt.toml` — use default `rustfmt`
- Errors: `thiserror`; logs: `tracing`
- Skills implement `poria_core::contracts::Skill` (`async_trait`)
- Channels implement the channel contract in `poria-core`
- Do not add unused GraphQL variables (EasyCI returns `UnusedVariable`)
- EasyCI `createChange` **only** accepts `branchOperateType: SELECT`. `CREATE` is not a valid enum. SELECT requires the branch already on the remote — **push first, then bind**
- Do not `Command::new("claude")` and hope GUI `PATH` contains it. Resolve via `poria_resources::resolve_claude_path` / `~/.poria/config.json` `claude_path` / login-shell `which`. See `.trellis/spec/frontend/claude-cli.md`.

### File organization

| Area            | Path                                                                                      |
| --------------- | ----------------------------------------------------------------------------------------- |
| Pages / widgets | `src/components/`                                                                         |
| Hooks           | `src/hooks/`                                                                              |
| Store           | `src/state/`                                                                              |
| Types + IPC     | `src/lib/`                                                                                |
| Theme           | `src/styles.css`, `src/components/ui/`, `design-system/poria/MASTER.md`                   |
| Tauri commands  | `src-tauri/src/commands/{auth,channels,config,demands,pipeline,projects,repos,skills}.rs` |
| Capabilities    | `src-tauri/capabilities/default.json`                                                     |
| Domain          | `crates/poria-core/`                                                                      |
| SQLite / auth   | `crates/poria-infrastructure/`                                                            |
| Channels        | `crates/poria-channels/src/{coding,defect,jme,joyspace,xingyun}/`                         |
| Bundled skills  | `skills/<name>/SKILL.md`                                                                  |

New IPC: add `#[tauri::command]` under `src-tauri/src/commands/`, register it in `src-tauri/src/lib.rs` `generate_handler!`, wrap it in `src/lib/tauri.ts`, and add a capability permission only if the command needs one.

## Integration contracts (do not guess)

- **SSO**: Cookie in `~/.poria/auth.json`. Commands that hit Xingyun / JoySpace / Coding must fail with 请先登录 when cookie is missing. Logged-out 看板 must not call `list_demands`.
- **Demand list**: 看板未开始列 pulls Xingyun. Default is related-to-me (omit JACP `receiver`). 「由我受理」 is `acceptedByMe` → `receiver` = ERP and only affects that Xingyun query. Do not stamp the logged-in ERP onto rows that have no receiver. There is no separate 需求 tab.
- **Start pipeline**: `submit_pipeline` must send `backendTrdUrl`. Reuse the latest pipeline for the demand key (`demand_code`, else `demand_id`): `Created` may update config; other statuses return the existing id. Frontend `base_branch` is the registered `default_branch` (default `master`). Backend repo stays **out of** `pipeline.repos`. Init creates `~/.poria/workspaces/<pipeline_id>/` (doc/skill symlinks + both worktrees) before ReviewPrd. Docs stay in `~/.poria/projects/<demand_code>/`. `workspacePath` (workspace root) is not the frontend `worktreePath`.
- **JoySpace export**: SSO cookie + POST `/v1/pages/content` (`poria-channels` joyspace). Init writes `PRD.md` / `BACKEND_TRD.md` under `~/.poria/projects/<demand_code>/`; ReviewPrd / Design write `PRD_REVIEW.md` / `TRD.md` there too via workspace-root symlinks — not into the git worktree.
- **Worktrees**: frontend `feature_<demand_code>` from `origin/<default_branch>` at `workspaces/<pipeline_id>/<frontend_name>`. Backend is `git worktree add --detach` so it does not lock the hosted clone branch. Hosted clones stay at `~/.poria/repos/<scope>/<name>` on `default_branch`.
- **Init failure**: `git worktree remove` both paths **then** delete the workspace dir. Bare `rm -rf` leaves a stale worktree in the hosted clone. Do not delete `~/.poria/projects`.
- **Deploy**: commit (`feat(<demand_code>): <name>`) → `git push -u` → `bind_branch` (EasyCI SELECT) → `find_mr_live` / `create_merge_request_live`. Do not bind before push. Strip leaked project docs from the frontend worktree before `git add`; skip symlink entries (`symlink_metadata`) so you do not delete projects docs.
- **Clone dest**: `~/.poria/repos/<scope>/<name>`.
- **Workspaces**: `~/.poria/workspaces/<pipeline_id>/` is Claude cwd. Frontend/backend git worktrees live in that directory. Bundled skills: repo `skills/<name>/SKILL.md` (dev) or Tauri resource `skills/` (release, `tauri.conf.json` `bundle.resources`).
- **Claude path**: Settings `claude_path` may be unset (login-shell `which`) or an absolute executable. Probe only in the Poria window (`probe_claude`). See `.trellis/spec/frontend/claude-cli.md`.
- **Repo tab**: `default_branch` defaults to `master`, editable. Save branch then sync immediately (sync fail does not roll back the field). IPC: `sync_repo`, `update_repo_default_branch`.

Load `.trellis/spec/` for the layer you edit (frontend index is `.trellis/spec/frontend/index.md`). Cross-layer payload / demand-filter / TRD changes: `.trellis/spec/guides/cross-layer-thinking-guide.md`. Pipeline workspace / worktree / skill symlink changes: `.trellis/spec/frontend/pipeline-workspace.md`. Theme / `PageFrame`: `.trellis/spec/frontend/visual-theme.md`.

## Data paths

| What                       | Where                                                      |
| -------------------------- | ---------------------------------------------------------- |
| Auth                       | `~/.poria/auth.json`                                       |
| Hosted clones              | `~/.poria/repos/`                                          |
| Demand markdown            | `~/.poria/projects/<demand_code>/`                         |
| Pipeline workspaces        | `~/.poria/workspaces/<pipeline_id>/`                       |
| App config                 | `~/.poria/config.json`                                     |
| SQLite (release / default) | `~/Library/Application Support/com.poria.desktop/poria.db` |
| SQLite (dev override)      | `workspace/db/poria.db` if that file exists                |

Do not commit credentials, `workspace/db/`, or `~/.poria` contents.

## Security

- Treat `~/.poria/auth.json`, SSO cookies, Apple / Tauri signing secrets, and agent worktrees as sensitive. Never paste them into Issues, PRs, logs, or chat.
- Security reports go through [SECURITY.md](SECURITY.md) (private advisory), not public Issues.
- Logged-out UI must not call Xingyun / JoySpace / Coding commands.
- Opener capability is limited to `$HOME/.poria/workspaces` and children (`src-tauri/capabilities/default.json`).
- Do not widen shell/opener permissions or write credentials without an explicit user request.

## Build and Deployment

Behavior specs (source of truth for CI): [spec/README.md](spec/README.md).

| Workflow                                | Trigger                                  | Result                                         |
| --------------------------------------- | ---------------------------------------- | ---------------------------------------------- |
| `.github/workflows/pre-publish.yml`     | tag `vX.Y.Z-beta.N`                      | GitHub **pre-release** macOS DMG               |
| `.github/workflows/publish.yml`         | tag `vX.Y.Z` (no `-beta`/`-rc`/`-alpha`) | **latest** GitHub Release                      |
| `.github/workflows/warm-rust-cache.yml` | push to `main` touching Rust             | Pre-warms aarch64 release cache                |
| `.github/workflows/labeler.yml`         | PR / labels.yml on `main`                | Path labels; CODEOWNERS requests `@clawd-cook` |

Publish jobs build **`aarch64-apple-darwin` only**. `APPLE_*` secrets are optional; missing certs must ad-hoc sign and still produce a DMG. Unsigned notes should mention `xattr -cr`. Local Intel builds are still `cargo tauri build --target x86_64-apple-darwin`.

```bash
git tag v1.1.1-beta.1 && git push origin v1.1.1-beta.1
git tag v1.1.1 && git push origin v1.1.1
```

GitHub runs the workflow file **on the tagged commit**. Re-running an old tag will not pick up `main` YAML fixes; retag or cut a new version. CI syncs the tag version into `package.json`, `src-tauri/tauri.conf.json`, and `src-tauri/Cargo.toml` during the job. Do not hand-edit those three to match a tag unless you are cutting a release.

CI Node/pnpm: `24.20.0` / `11.23.0` (workflow `env`).

## High-risk operations (ask first)

**Stop and get an explicit yes from the user** before any operation that changes the machine outside this repository. Do not proceed on implied consent, "it would help", or because a skill/docs suggested it.

Requires confirmation:

- **Global package installs** — `npm i -g`, `pnpm add -g`, `yarn global`, `bun add -g`, `brew install` / `brew upgrade`, OS package managers, editor/CLI plugins installed for all projects
- **Global or extra-repo deletes** — anything outside the checkout: `~/.nvm`, Homebrew prefixes, `/usr/local`, `/opt/homebrew`, other clones, shell history, credentials, nvm versions, `rm -rf` on home or system paths
- **Global environment switches** — `nvm alias default`, `nvm install`, `nvm uninstall`, changing default Node, editing `~/.zshrc` / `~/.bashrc` / `~/.zprofile`, mutating `PATH` persistently, Docker/context switches, logging into cloud CLIs
- **Destructive git/machine actions** — `git push --force`, hard reset of shared branches, rewriting git config, skipping hooks
- **Secrets and identity** — writing credentials, SSH keys, tokens, or changing git `user.*`

How to ask: state the exact command, what it changes (path + scope), why you think it is needed, and a repo-local alternative if one exists (`pnpm add -D`, `pnpm exec`, `pnpm dlx`). Wait for a clear yes. If the user says no or does not answer, skip it and continue with in-repo tools only.

Prefer in-repo, session-local work: `pnpm install` / `pnpm add -D` in the right workspace, `pnpm exec`, `pnpm dlx`, and `nvm use 24.20.0` in this shell.

## Debugging

- Vite `transformCallback` noise in a **browser** tab on `:1420` is expected; it is not the Tauri webview.
- Empty Accessibility `window 1` on macOS: the window title is `Poria`.
- Pipeline SQLite: check which `poria.db` the running binary opened (dev override vs Application Support).
- Skills that call Claude: require a resolvable `claude` binary; keep `--tools` / disallowed-tools consistent with `poria-resources` CLI wrapper. GUI apps often lack Homebrew on `PATH` — use `claude_path` / login-shell `which`, not a bare `claude` spawn.
- EasyCI bind errors: `522721` / branch not found → remote branch missing (push first). GraphQL `UnusedVariable` → drop the unused field from the query.
- `pnpm tauri` → `tauri: command not found`: use `cargo tauri` (this package does not depend on `@tauri-apps/cli`).
- Init leftover worktrees after a failed run: `git worktree list` in the hosted clone; remove with `git worktree remove`, not only `rm -rf` on `~/.poria/workspaces`.

## Pull requests / commits

No required title prefix. Prefer short why-focused messages. Deploy-generated commits use `feat(<demand_code>): <demand name>`.

Default PR template: `.github/PULL_REQUEST_TEMPLATE.md`. Feature / bugfix: `?template=feature.md` / `?template=bugfix.md`. Path labeler: `.github/workflows/labeler.yml`. Reviewers: `.github/CODEOWNERS` (`@clawd-cook`).

Before finishing a code change:

1. `pnpm typecheck` if `src/` changed
2. `cargo test -p <crate>` (or `--workspace`) if Rust changed
3. `pnpm exec oxfmt .` if `src/` changed
4. Do not commit `submodules/` pointer dirt, `~/.poria`, or secrets unless the user explicitly asks

## Additional Notes

- Default Homebrew `node` on PATH may be **not** 24. Always activate nvm 24.20.0 in the same shell as pnpm.
- `submodules/` — clone/reference only. Implement against those APIs in **this** repo’s crates, never by editing the submodule tree.
- Before editing a layer, load that package’s spec index under `.trellis/spec/`.
- Trellis skills live under `.cursor/skills/` (and `.claude/skills/`). Follow `.trellis/workflow.md` when a Trellis task is active.
- `CLAUDE.md` is `@AGENTS.md` — keep this file as the single agent source of truth.

<!-- TRELLIS:START -->

# Trellis Instructions

These instructions are for AI assistants working in this project.

This project is managed by Trellis. The working knowledge you need lives under `.trellis/`:

- `.trellis/workflow.md` — development phases, when to create tasks, skill routing
- `.trellis/spec/` — package- and layer-scoped coding guidelines (read before writing code in a given layer)
- `.trellis/workspace/` — per-developer journals and session traces
- `.trellis/tasks/` — active and archived tasks (PRDs, research, jsonl context)

If a Trellis command is available on your platform (e.g. `/trellis:finish-work`, `/trellis:continue`), prefer it over manual steps. Not every platform exposes every command.

If you're using Codex or another agent-capable tool, additional project-scoped helpers may live in:

- `.agents/skills/` — reusable Trellis skills
- `.codex/agents/` — optional custom subagents

Managed by Trellis. Edits outside this block are preserved; edits inside may be overwritten by a future `trellis update`.

<!-- TRELLIS:END -->
