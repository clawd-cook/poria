# Architecture Decision: Three-Layer Dispatch Model

> Source: user clarification, 2026-09-17

## Principle

**非必要不用 AI** — Only invoke Claude when understanding, reasoning, or generation is required.

## Dispatch Layers

### Layer 1: Platform Operations (No AI)

Tasks that are purely mechanical — reading data, calling APIs, managing git operations.

| Task | Executor |
|------|----------|
| Read requirement docs (JoySpace fetch) | channel (joyspace) |
| Create branch | resource (worktree / git) |
| Link branch to task | resource / channel |
| Submit MR | channel (coding) |
| Deploy environment | channel (xingyun / jme) |
| Fetch API docs | channel (joyspace) |
| Fetch UI assets (Deco/Relay) | channel (joyrelay) |

### Layer 2: AI Capabilities (Claude + Skill)

Tasks that require understanding, analysis, reasoning, or code generation.

| Task | Harness Equivalent | Poria Skill |
|------|-------------------|-------------|
| PRD review & clarification | hfe-review-prd | skill: prd-review |
| TRD generation | hfe-gen-trd | skill: trd-gen |
| Code implementation | hfe-execute | skill: code-impl |
| Code review (App) | hfe-cr-app | skill: cr-app |
| Code review (Web) | hfe-cr-web | skill: cr-web |
| E2E test generation | (new) | skill: e2e-test |

Invocation: App spawns a local Claude session with the specific skill prompt injected as system prompt. The skill prompt is bundled inside the Poria app, not fetched from a global npm package.

### Layer 3: Orchestration

The Poria pipeline engine (`poria-core`) orchestrates the flow:
- State machine manages stage transitions
- Gates enforce prerequisites (e.g., PRD_REVIEW.md must exist before TRD)
- Human-in-the-loop gates pause for user review at key checkpoints
- Risk classifier may escalate or skip stages based on change scope

## Key Implications

1. **Skills are app-internal** — no dependency on globally installed `@jd/harness-o2o-fe`
2. **Claude is a tool, not the orchestrator** — pipeline engine drives the flow, Claude handles specific AI tasks
3. **Channel/resource modules handle all I/O** — Claude never directly calls external services
4. **Each AI stage is stateless from Claude's perspective** — all context is provided via skill prompt + input files
