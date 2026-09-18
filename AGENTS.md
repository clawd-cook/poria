# AGENTS.md

Poria is an AI-native delivery platform: a Tauri v2 macOS desktop app that turns a Xingyun demand into a Coding merge request. Frontend is React 19 + TypeScript + Vite + Ant Design 6 / Ant Design X. Backend is Rust, a Cargo workspace of six domain crates plus the Tauri shell (`poria-desktop`).

This file is for coding agents. Human product docs live in `README.md`.

## Architecture

```
poria/
├── src/                      # React frontend (TSX, hooks, state, components)
│   ├── components/           # Ant Design pages and pipeline UI
│   ├── hooks/                # usePipeline, useTauriEvents
│   ├── lib/                  # types + invoke wrappers (`tauri.ts`)
│   └── state/                # reducer store + actions
├── src-tauri/                # Tauri v2 shell (crate name: poria-desktop)
│   └── src/commands/         # IPC handlers (the only Tauri command layer)
├── crates/
│   ├── poria-core/           # Types, state machine, gates, risk, contracts, artifacts
│   ├── poria-infrastructure/ # Auth, config, metrics, SQLite
│   ├── poria-commands/       # Pipeline executor, error classification, rollback
│   ├── poria-resources/      # Claude agent pool, terminal, worktree, output guard
│   ├── poria-skills/         # Stage skills (Init … Deploy), HITL, fixtures
│   └── poria-channels/       # Coding, JoySpace, Xingyun, JME, defect
├── public/                   # Logos / static assets
└── submodules/               # Git submodules — do not modify files inside
```

Frontend alias: `@/*` → `./src/*` (`tsconfig.json`; Vite does not need a special alias beyond this).

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

- Put domain types and state-machine rules in `poria-core`.
- Put HTTP / git-host / JoySpace / Xingyun clients in `poria-channels`.
- Put Claude CLI / git worktree / shell in `poria-resources`.
- Put stage behavior in `poria-skills`.
- Put orchestrate / retry / rollback in `poria-commands`.
- Put `#[tauri::command]` only in `src-tauri/src/commands/`. `poria-commands` is **not** the IPC layer.

Shared Rust deps live in root `Cargo.toml` `[workspace.dependencies]`. Add versions there, then `{ workspace = true }` in crate manifests.

### Pipeline stages → skills

| `StageEnum` | Skill id            | Implementation                                                                                                                                                                                            |
| ----------- | ------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Init`      | `skill:init`        | Export JoySpace docs into `~/.poria/projects/<demand_code>/`, create `~/.poria/workspaces/<pipeline_id>/` (doc + skill symlinks, `CLAUDE.md`), then frontend feature worktree + backend detached worktree |
| `ReviewPrd` | `skill:review-prd`  | Workspace-root `claude -p` short prompt naming `review-prd`; write `PRD_REVIEW.md` via symlink into projects                                                                                              |
| `Design`    | `skill:gen-trd`     | Same cwd; write frontend `TRD.md` into the demand project dir via workspace symlink                                                                                                                       |
| `Dev`       | `skill:gen-code`    | Same cwd; codegen + `TASK.md`; only the frontend worktree is modified                                                                                                                                     |
| `Cr`        | `skill:code-review` | Same cwd; `CR.md` + gates                                                                                                                                                                                 |
| `Deploy`    | `skill:deploy`      | Commit if dirty → **push** → EasyCI SELECT bind → find/create MR (frontend worktree cwd)                                                                                                                  |

Map source of truth: `crates/poria-skills/src/stage_skill_map.rs` and `crates/poria-commands/src/traits.rs` (`stage_skill_id`). Keep both in sync.

Artifacts (`poria-core` `feature_context`): `PRD.md`, `PRD_REVIEW.md`, `TRD.md`, `BACKEND_TRD.md`, `TASK.md`, `CR.md`. Frontend TRD and backend TRD are different files — do not collapse them.

Desktop agent stages must stay non-interactive (`claude -p`). Do not add HITL prompts inside those skills.

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

Do not run `npm install`, `yarn`, or `bun install`.

## Development Workflow

### Frontend only (Vite)

```bash
pnpm dev         # http://localhost:1420 — Vite only, no Tauri IPC
```

Do **not** claim login, demand list, clone, or pipeline submit works from a Cursor browser tab on `:1420`. Those require `invoke` / `listen` in the `poria-desktop` window.

### Full desktop app (Tauri)

```bash
pnpm tauri dev   # Rust backend + Vite + native window (title: Poria, process: poria-desktop)
```

Port **1420 is strict** (`vite.config.ts` `strictPort: true`). Vite watches `src/` and ignores `src-tauri/`. If 1420 is taken, free it before `tauri dev`.

Prefer testing the debug binary (`target/debug/poria-desktop`), not a stale `/Applications/Poria.app`.

Before claiming clone / SSO / Xingyun / `submit_pipeline` works, follow `.trellis/spec/frontend/tauri-desktop-testing.md`.

### TypeScript

```bash
pnpm typecheck   # tsc -b
```

### Production build

```bash
pnpm build                     # tsc -b && vite build → dist/
pnpm tauri build               # desktop bundle
pnpm tauri build --target aarch64-apple-darwin
pnpm tauri build --target x86_64-apple-darwin
```

## Testing

### Rust

Tests live **in the crate sources** (`#[cfg(test)]` / `#[tokio::test]`), not a separate `tests/` tree. Cover all six domain crates when the change spans them.

```bash
cargo test --workspace
cargo test -p poria-core
cargo test -p poria-core -- state_machine
cargo check --workspace
cargo clippy --workspace
```

Add or update tests for Rust you change. No coverage gate is configured.

### Frontend

No frontend test runner. After UI/IPC changes: `pnpm typecheck`, then exercise the **desktop** window. `pnpm exec oxfmt .` for format.

## Code Style

### TypeScript / React

- Formatter: `pnpm exec oxfmt .` (`.oxfmtrc.json`: sort imports, sort object keys, sort Tailwind classes; ignores `.claude`, `.trellis`, `submodules`)
- Strict TS: `noUnusedLocals`, `noUnusedParameters`, `noFallthroughCasesInSwitch`
- ESM (`"type": "module"`), JSX `react-jsx` (no default React import)
- UI: Ant Design 6 + `@ant-design/x*`, locale `zh_CN`, light theme `poriaTheme` in `src/theme.ts` (Swiss/Minimal tokens, no extra brand palette)
- Global state: `src/state/store.tsx` reducer + `src/state/actions.ts`. Do not introduce a second store.
- IPC: wrap `invoke` in `src/lib/tauri.ts`. Tauri v2 maps JS **camelCase** args to Rust snake_case (`pipelineId` → `pipeline_id`, `gitUrl` → `git_url`, `backendTrdUrl` → `backend_trd_url`).
- CSS: Tailwind v4 via Vite (no `tailwind.config.*`). Prefer Ant Design props for layout; Tailwind only where already used.

### Rust

- Edition 2021; no repo `rustfmt.toml` — use default `rustfmt`
- Errors: `thiserror`; logs: `tracing`
- Skills implement `poria_core::contracts::Skill` (`async_trait`)
- Channels implement the channel contract in `poria-core`
- Do not add unused GraphQL variables (EasyCI returns `UnusedVariable`)
- EasyCI `createChange` **only** accepts `branchOperateType: SELECT`. `CREATE` is not a valid enum. SELECT requires the branch already on the remote — **push first, then bind**

### File organization

| Area            | Path                                                                                      |
| --------------- | ----------------------------------------------------------------------------------------- |
| Pages / widgets | `src/components/`                                                                         |
| Hooks           | `src/hooks/`                                                                              |
| Store           | `src/state/`                                                                              |
| Types + IPC     | `src/lib/`                                                                                |
| Tauri commands  | `src-tauri/src/commands/{auth,channels,config,demands,pipeline,projects,repos,skills}.rs` |
| Capabilities    | `src-tauri/capabilities/default.json`                                                     |
| Domain          | `crates/poria-core/`                                                                      |
| SQLite / auth   | `crates/poria-infrastructure/`                                                            |
| Channels        | `crates/poria-channels/src/{coding,defect,jme,joyspace,xingyun}/`                         |

## Integration contracts (do not guess)

- **SSO**: Cookie in `~/.poria/auth.json`. Commands that hit Xingyun / JoySpace / Coding must fail with 请先登录 when cookie is missing. Logged-out demand list UI must not call `list_demands`.
- **Demand list**: default is related-to-me (omit JACP `receiver`). 「由我受理」 is `acceptedByMe` → `receiver` = ERP. Do not stamp the logged-in ERP onto rows that have no receiver.
- **Start pipeline**: `submit_pipeline` must send `backendTrdUrl`. Frontend `base_branch` is the registered `default_branch` (default `master`). Backend repo stays **out of** `pipeline.repos`. Init creates `~/.poria/workspaces/<pipeline_id>/` (doc/skill symlinks + both worktrees) before ReviewPrd. Docs stay in `~/.poria/projects/<demand_code>/`.
- **JoySpace export**: SSO cookie + POST `/v1/pages/content` (`poria-channels` joyspace). Init writes `PRD.md` / `BACKEND_TRD.md` under `~/.poria/projects/<demand_code>/`; ReviewPrd / Design write `PRD_REVIEW.md` / `TRD.md` there too via workspace-root symlinks — not into the git worktree.
- **Deploy**: commit (`feat(<demand_code>): <name>`) → `git push -u` → `bind_branch` (EasyCI SELECT) → `find_mr_live` / `create_merge_request_live`. Do not bind before push.
- **Clone dest**: `~/.poria/repos/<scope>/<name>`.
- **Workspaces**: `~/.poria/workspaces/<pipeline_id>/` is Claude cwd. Frontend/backend git worktrees live in that directory. Bundled skills: repo `skills/<name>/SKILL.md`.

Load `.trellis/spec/` for the layer you edit. Cross-layer payload / demand-filter / TRD changes: `.trellis/spec/guides/cross-layer-thinking-guide.md`.

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

## Build and Deployment

GitHub Actions:

| Workflow                            | Tag                                  | Result                            |
| ----------------------------------- | ------------------------------------ | --------------------------------- |
| `.github/workflows/pre-publish.yml` | `vX.Y.Z-beta.N`                      | GitHub **pre-release** macOS DMGs |
| `.github/workflows/publish.yml`     | `vX.Y.Z` (no `-beta`/`-rc`/`-alpha`) | **latest** GitHub Release         |

Both build `aarch64-apple-darwin` and `x86_64-apple-darwin`. `APPLE_*` secrets are **optional**; missing certs must ad-hoc sign and still produce a DMG (do not fail-fast on empty secrets). Unsigned notes should mention `xattr -cr`.

```bash
git tag v1.1.1-beta.1 && git push origin v1.1.1-beta.1
git tag v1.1.1 && git push origin v1.1.1
```

GitHub runs the workflow file **on the tagged commit**. Re-running an old tag will not pick up `main` YAML fixes — retag or cut a new version.

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
- Skills that call Claude: require `claude` on PATH; keep `--tools` / disallowed-tools consistent with `poria-resources` CLI wrapper.
- EasyCI bind errors: `522721` / branch not found → remote branch missing (push first). GraphQL `UnusedVariable` → drop the unused field from the query.

## Pull requests / commits

No required title prefix. Prefer short why-focused messages. Deploy-generated commits use `feat(<demand_code>): <demand name>`.

Before finishing a code change:

1. `pnpm typecheck` if `src/` changed
2. `cargo test -p <crate>` (or `--workspace`) if Rust changed
3. Do not commit `submodules/` pointer dirt, `~/.poria`, or secrets unless the user explicitly asks

## Additional Notes

- Default Homebrew `node` on PATH may be **not** 24. Always activate nvm 24.20.0 in the same shell as pnpm.
- `submodules/` — clone/reference only. Implement against those APIs in **this** repo’s crates, never by editing the submodule tree.
- Before editing a layer, load that package’s spec index under `.trellis/spec/`.
- Trellis skills live under `.cursor/skills/` (and `.claude/skills/`). Follow `.trellis/workflow.md` when a Trellis task is active.

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
