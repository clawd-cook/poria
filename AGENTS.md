# AGENTS.md

Poria is an AI-native delivery platform built as a Tauri v2 desktop application. The frontend is React 19 + TypeScript + Vite; the backend is Rust, organized as a Cargo workspace of six domain crates plus the Tauri shell.

## Architecture

```
poria/
├── src/                  # React frontend (TSX, hooks, state, components)
├── src-tauri/            # Tauri v2 desktop shell (Rust)
│   └── src/commands/     # Tauri command handlers
├── crates/
│   ├── poria-core/       # Pipeline engine, state machine, gates, risk classifier, contracts
│   ├── poria-infrastructure/  # Auth, config, metrics, SQLite store
│   ├── poria-commands/   # Command dispatch, error handling, rollback
│   ├── poria-resources/  # Claude agent pool, terminal, worktree management
│   ├── poria-skills/     # Stage skill map, human-in-the-loop, fixtures
│   └── poria-channels/   # External integrations (Coding, JME, JoySpace, Xingyun, defect)
├── public/               # Static assets (logos, icons)
└── submodules/           # Git submodules (do not modify directly)
```

Frontend path alias: `@/*` maps to `./src/*` (configured in `tsconfig.json` and resolved by Vite).

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

### Frontend only (Vite dev server)

```bash
pnpm dev         # starts Vite on http://localhost:1420
```

### Full desktop app (Tauri)

```bash
pnpm tauri dev   # builds Rust backend + launches Vite + opens desktop window
```

The Tauri dev server uses port 1420 (strict). Vite watches `src/` and ignores `src-tauri/`.

### TypeScript type checking

```bash
pnpm typecheck   # runs tsc -b
```

### Production build

```bash
pnpm build                     # tsc -b && vite build (frontend only, outputs to dist/)
pnpm tauri build               # full desktop app bundle
pnpm tauri build --target aarch64-apple-darwin   # Apple Silicon
pnpm tauri build --target x86_64-apple-darwin    # Intel Mac
```

## Testing

### Rust tests

```bash
cargo test --workspace                    # all crates
cargo test -p poria-core                  # single crate
cargo test -p poria-core -- state_machine # filter by test name
```

Tests exist across all six crates. Many use `#[tokio::test]` for async tests.

### Frontend tests

No frontend test framework is currently configured. Validate frontend changes by running `pnpm typecheck` and manual testing via `pnpm tauri dev`.

### Rust checks

```bash
cargo check --workspace      # fast type check
cargo clippy --workspace     # lint
```

## Code Style

### TypeScript / React

- **Formatter**: oxfmt (`pnpm exec oxfmt .` or configure editor integration)
  - Sort imports enabled
  - Sort object keys enabled
  - Sort Tailwind CSS classes enabled
  - Config: `.oxfmtrc.json`
  - Ignores: `.claude`, `.trellis`, `submodules`
- **Strict TypeScript**: `noUnusedLocals`, `noUnusedParameters`, `noFallthroughCasesInSwitch`
- **Module system**: ESM (`"type": "module"` in package.json)
- **JSX**: `react-jsx` transform (no manual React imports needed)
- **CSS**: Tailwind CSS v4 via Vite plugin (no separate config file)

### Rust

- Edition 2021 across all crates
- Workspace dependencies are centralized in root `Cargo.toml` `[workspace.dependencies]`
- Key shared dependencies: `serde`, `serde_json`, `rusqlite` (bundled), `tokio` (full), `chrono`, `thiserror`, `tracing`, `async-trait`
- Follow standard `cargo clippy` recommendations
- Error types use `thiserror`; logging uses `tracing`

### File organization

- Frontend components: `src/components/`
- Frontend hooks: `src/hooks/`
- Frontend state management: `src/state/` (store + actions)
- Frontend types and Tauri bindings: `src/lib/`
- Tauri commands: `src-tauri/src/commands/`
- Domain logic: `crates/poria-core/`
- Infrastructure (DB, auth, metrics): `crates/poria-infrastructure/`
- External channel integrations: `crates/poria-channels/src/{coding,defect,jme,joyspace,xingyun}/`

## Build and Deployment

CI is configured in `.github/workflows/pre-publish.yml`. It triggers on `v*-beta*` tags and:

1. Validates the beta tag format (`vX.Y.Z-beta.N`)
2. Builds macOS DMGs for both `aarch64` and `x86_64`
3. Creates a GitHub pre-release with the DMG artifacts

To trigger a beta release, push a tag: `git tag v1.0.1-beta.5 && git push origin v1.0.1-beta.5`

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

## Crate Reference

| Crate | Purpose |
|-------|---------|
| `poria-core` | Pipeline engine: state machine, gates, risk classifier, event system, multi-repo support |
| `poria-infrastructure` | Auth/credentials, config, metrics, SQLite persistence (`rusqlite`) |
| `poria-commands` | Tauri IPC command handlers, error classification, rollback logic |
| `poria-resources` | Claude agent pool, output guard, session tracking, terminal, worktree ops |
| `poria-skills` | Stage-to-skill mapping, human-in-the-loop prompts, test fixtures |
| `poria-channels` | External platform integrations: Coding (git), JME, JoySpace, Xingyun, defect tracking |

## Additional Notes

- Default Homebrew `node` on PATH may be **not** 24. Always activate nvm 24.20.0 in the same shell as pnpm.
- `submodules/` contains git submodules — do not modify files inside directly.
- Before editing a layer, load that package's spec index under `.trellis/spec/`.
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
