<div align="center">
  <img src="public/logo-transparent.png" alt="Poria" width="128" />

  # Poria

  *AI-native delivery platform that turns a product requirement into a deployed merge request.*

  [![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

  [Features](#features) · [Architecture](#architecture) · [Getting Started](#getting-started) · [Usage](#usage) · [Development](#development)

</div>

---

Poria orchestrates the full software delivery lifecycle — from PRD review through technical design, code generation, code review, and deployment — as a single automated pipeline. Each stage is powered by pluggable AI skills, gated by configurable quality checks, and supervised through a human-in-the-loop desktop UI.

## Features

- **End-to-end pipeline** — Seven ordered stages: Init, PRD Review, Technical Design, Workspace Setup, Development, Code Review, and Deploy
- **Quality gates** — Configurable rules for CR score, test coverage, CI build, security scan, diff size, and merge conflicts, each with block/warn/regress behavior
- **Human-in-the-loop** — Pipeline pauses and notifies when it needs human judgment; approve, skip, or cancel from the UI
- **Risk classification** — Automatic risk assessment (low/medium/high/critical) based on changed file patterns and diff size
- **Multi-repo orchestration** — Topological dependency resolution across multiple repositories with coordinated stage execution
- **Pluggable skills** — Each stage maps to a skill (PRD review, TRD generation, code generation, code review, deploy); add your own via the skill contract
- **Pluggable channels** — Integrate with external systems (Coding, JoySpace, Defect trackers, Xingyun) through a uniform channel interface
- **Agent resource management** — Claude CLI agent pool with concurrency limits, output guards (file scope, diff size, dependency validation), and session tracking
- **Automatic rollback** — Rollback instructions (worktree cleanup, branch deletion, MR closure) are recorded per stage and can be executed on failure
- **Desktop app** — Tauri v2 desktop UI with real-time pipeline monitoring, event stream, and stage progress visualization

## Architecture

Poria is structured as a Rust workspace with a React + Tauri v2 frontend:

```
poria/
├── crates/
│   ├── poria-core           # Domain types, state machine, gates, risk classifier
│   ├── poria-commands        # Pipeline executor, error handling, rollback engine
│   ├── poria-skills          # Built-in skill implementations (PRD review, TRD, codegen, CR, deploy)
│   ├── poria-resources       # Terminal, worktree, Claude agent pool, output guard
│   ├── poria-channels        # External system integrations (Coding, JoySpace, Defect, Xingyun)
│   └── poria-infrastructure  # SQLite store, auth, config, logger, metrics, plugin loader
├── src-tauri/                # Tauri v2 app shell, IPC commands
└── src/                      # React frontend (Vite + Tailwind CSS)
```

### Pipeline state machine

A pipeline transitions through a well-defined set of states:

```
Created → Running → WaitingMerge → Completed
              ↓          ↓
           Blocked     Failed
              ↓          ↓
           Running    Cancelled
```

Each stage within a pipeline follows its own lifecycle: `Pending → Running → Completed`, with `Failed`, `Blocked`, and `Skipped` as alternative terminal or recoverable states. Failed stages can retry up to a configurable limit; blocked stages pause the pipeline for human intervention.

### Gate evaluation

Gates run at two phases — **stage exit** (e.g. CR score, security scan) and **deploy** (e.g. CI build, test coverage, diff size, merge conflicts). Each gate rule defines an `on_fail` policy:

| Policy | Behavior |
|---|---|
| `block` | Pipeline pauses, requires human intervention |
| `warn` | Warning recorded, pipeline continues |
| `regress` | Pipeline rolls back to a prior stage (e.g. CR fail → re-run Dev) |

Regression is allowed once per pipeline. A second regression failure escalates to a block.

## Getting Started

### Prerequisites

- **Node.js** 24.x (managed via [nvm](https://github.com/nvm-sh/nvm))
- **pnpm** 11.23.0
- **Rust** stable toolchain (for building the Tauri backend)

### Install dependencies

```bash
# Activate the correct Node version
export NVM_DIR="$HOME/.nvm"
[ -s "/opt/homebrew/opt/nvm/nvm.sh" ] && . "/opt/homebrew/opt/nvm/nvm.sh"
nvm use 24.20.0

# Install frontend dependencies
pnpm install
```

### Build and run

```bash
# Development (frontend + Tauri dev server)
pnpm tauri dev

# Production build
pnpm tauri build
```

## Usage

### Submitting a pipeline

From the desktop UI, paste a demand link to create a new pipeline. Poria will:

1. **Init** — Parse the demand and set up pipeline metadata
2. **PRD Review** — Analyze the product requirement, flag ambiguities
3. **Technical Design** — Generate a technical design document
4. **Workspace** — Create a git worktree and branch for isolated development
5. **Development** — Generate code changes guided by the TRD
6. **Code Review** — Run automated CR, evaluate quality gates
7. **Deploy** — Create merge requests, run CI, evaluate deploy gates

### Human-in-the-loop

When a stage blocks (e.g. low CR score, CI failure), the UI shows an intervention card with three options:

- **Fix and retry** — Resolve the issue and re-run the stage
- **Skip** — Skip the current stage and continue
- **Cancel** — Cancel the entire pipeline

### Configuration

Adjust quality thresholds and behavior from the Settings panel:

| Setting | Default | Description |
|---|---|---|
| CR score threshold | `B+` | Minimum code review grade to pass |
| Test coverage threshold | `80%` | Minimum test coverage to pass deploy gate |
| Max diff lines | `500` | Diff size warning threshold |
| Agent timeout | `300000ms` | Timeout for AI agent execution |
| Max retries | `3` | Maximum retry attempts per stage |

## Development

### Project structure

| Crate | Purpose |
|---|---|
| `poria-core` | Domain types, pipeline state machine, gate evaluation, risk classifier, multi-repo topology |
| `poria-commands` | Pipeline executor, error classification, rollback engine |
| `poria-skills` | Built-in skills: `ReviewPrdSkill`, `GenTrdSkill`, `GenCodeSkill`, `CodeReviewSkill`, `DeploySkill`, `InitSkill`, `WorkspaceSkill` |
| `poria-resources` | `TerminalResource`, `WorktreeResource`, `ClaudeAgentPool`, `OutputGuard`, `SessionTracker` |
| `poria-channels` | Coding, JoySpace, Defect, Xingyun integrations |
| `poria-infrastructure` | SQLite persistence, auth, config, logging, metrics, plugin loader |

### Running tests

```bash
cargo test --workspace
```

### Frontend development

```bash
pnpm dev          # Vite dev server on http://localhost:1420
pnpm build        # Production build
pnpm typecheck    # TypeScript type checking
```

### Tech stack

| Layer | Technology |
|---|---|
| Backend | Rust, Tokio, SQLite (rusqlite), Serde |
| Frontend | React 19, TypeScript 7, Tailwind CSS 4, Vite 8 |
| Desktop | Tauri v2 |
| AI Agent | Claude CLI subprocess pool |
