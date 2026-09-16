# AGENTS.md

`poria` is a Node.js project. Package manager is pinned in root `package.json`: `packageManager` is `pnpm@11.23.0`; `devEngines.packageManager` is pnpm `11.23.0` with `onFail: "download"`.

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

`nvm use 24.20.0` in the **current shell** is required and does not need extra confirmation. Do **not** change nvm’s default alias, install another Node, or edit shell rc files to make 24 global.

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

Corepack (or `devEngines` `onFail: "download"`) may fetch this exact pnpm when missing; do not install a different global pnpm to “make it work”.

## High-risk operations (ask first)

**Stop and get an explicit yes from the user** before any operation that changes the machine outside this repository. Do not proceed on implied consent, “it would help”, or because a skill/docs suggested it.

Requires confirmation:

- **Global package installs** — `npm i -g`, `pnpm add -g`, `yarn global`, `bun add -g`, `brew install` / `brew upgrade`, OS package managers, editor/CLI plugins installed for all projects
- **Global or extra-repo deletes** — anything outside the checkout: `~/.nvm`, Homebrew prefixes, `/usr/local`, `/opt/homebrew`, other clones, shell history, credentials, nvm versions, `rm -rf` on home or system paths
- **Global environment switches** — `nvm alias default`, `nvm install`, `nvm uninstall`, changing default Node, editing `~/.zshrc` / `~/.bashrc` / `~/.zprofile`, mutating `PATH` persistently, Docker/context switches, logging into cloud CLIs
- **Destructive git/machine actions** — `git push --force`, hard reset of shared branches, rewriting git config, skipping hooks
- **Secrets and identity** — writing credentials, SSH keys, tokens, or changing git `user.*`

How to ask: state the exact command, what it changes (path + scope), why you think it is needed, and a repo-local alternative if one exists (`pnpm add -D`, `pnpm exec`, `pnpm dlx`). Wait for a clear yes. If the user says no or does not answer, skip it and continue with in-repo tools only.

Prefer in-repo, session-local work: `pnpm install` / `pnpm add -D` in the right workspace, `pnpm exec`, `pnpm dlx`, and `nvm use 24.20.0` in this shell.

## Setup Commands

Use **pnpm 11.23.0** and Node 24.20.0 only. Install from the **repo root**.

```bash
export NVM_DIR="$HOME/.nvm"
. "/opt/homebrew/opt/nvm/nvm.sh"
nvm use 24.20.0
pnpm -v          # must print 11.23.0
pnpm install
```

Do not run `npm install`, `yarn`, or `bun install`.

## Additional Notes

- Default Homebrew `node` on PATH may be **not** 24. Always activate nvm 24.20.0 in the same shell as pnpm.
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
