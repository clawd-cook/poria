# Implementation Plan: Internalize harness-o2o-fe into Poria

## Execution Order & Dependencies

```
Child 1: Claude CLI Invoker + Skill Resources
  ↓ (provides AgentSdk, skill prompts)
Child 2: Feature Context + Pipeline Wiring
  ↓ (provides FeatureContext, gate rules, AppState wiring)
Child 3: Stage Skill Implementations
  ↓ (fills 4 skill stubs, connects to agent pool)
Child 4: Frontend Workflow UI
  (consumes stage events, stream output, human-loop)
```

Child 1 is the foundation. Child 2 and Child 4 can partially overlap. Child 3 depends on Child 1 + 2.

---

## Child 1: Claude CLI Invoker + Skill Resource System

### 1.1 Implement `ClaudeCliSdk`

**File**: `crates/poria-resources/src/claude/cli_sdk.rs` (new)

- [ ] Implement `AgentSdk` trait for `ClaudeCliSdk`
- [ ] Build CLI args from `AgentQueryOptions` (prompt, system-prompt, allowed-tools, max-turns, output-format, cwd)
- [ ] Spawn `claude` subprocess via `tokio::process::Command`
- [ ] Parse `stream-json` stdout line by line into `SdkMessage` variants
- [ ] Extract `session_id` from `result` event for session resume
- [ ] Handle stderr for error capture
- [ ] Support process kill for cancellation (via `AbortHandle` or `child.kill()`)
- [ ] Auto-detect `claude` binary path (PATH lookup, configurable override)

**Validation**: Unit test with mock CLI script that outputs stream-json format.

### 1.2 Stream Event Bridge

**File**: `crates/poria-resources/src/claude/stream_bridge.rs` (new)

- [ ] Define `StreamEvent` enum: `TextDelta(String)`, `ToolUse { name, input }`, `ToolResult { output }`, `Result { session_id, cost }`
- [ ] Parser: `fn parse_stream_line(line: &str) -> Option<StreamEvent>`
- [ ] Bridge to Tauri event system: `fn emit_stream_event(app_handle, pipeline_id, stage, event)`
- [ ] Rate limiting for high-frequency text deltas (batch within 50ms window)

**Validation**: Parse sample stream-json output from a real Claude CLI run.

### 1.3 Skill Prompt Templates

**Dir**: `crates/poria-skills/src/prompts/` (new directory)

- [ ] Migrate and distill `hfe-review-prd` → `prd_review.md`
- [ ] Migrate and distill `hfe-gen-trd` → `trd_gen.md`
- [ ] Migrate and distill `hfe-execute` → `code_impl.md`
- [ ] Migrate and distill `hfe-cr-web` → `cr_web.md`
- [ ] Embed via `include_str!()` in skill module
- [ ] Template variable renderer: `render_prompt(template, vars) -> String`

**Distillation rules**:
- Remove harness-specific script calls (`scripts/validate-trd.py`, `scripts/mr_review.py`, etc.)
- Remove global install references
- Remove metrics reporting calls
- Keep core analysis/generation logic and quality rubrics
- Adapt file paths to `.poria/features/` convention
- Keep grilling/adversarial review methodology

**Validation**: Each prompt renders without unresolved `{{vars}}` given test input.

### 1.4 Wire ClaudeCliSdk into ClaudeAgentPool

**File**: `crates/poria-resources/src/claude/agent_pool.rs` (edit)

- [ ] Add `ClaudeAgentPool::new_with_cli()` constructor that creates `ClaudeCliSdk`
- [ ] Keep existing `new()` for test/fixture mode with `MockSdk`

---

## Child 2: Feature Context + Pipeline Wiring

### 2.1 Feature Context Module

**File**: `crates/poria-core/src/feature_context.rs` (new)

- [ ] `FeatureContext` struct: id, pipeline_id, root path, created_at
- [ ] `FeatureContext::create(project_root, name) -> Self` — creates `.poria/features/<id>/` dir
- [ ] `artifact_path(name) -> PathBuf`
- [ ] `has_artifact(name) -> bool`
- [ ] `list_artifacts() -> Vec<String>`
- [ ] `write_artifact(name, content)` — write file to feature dir
- [ ] `read_artifact(name) -> Option<String>` — read file from feature dir

### 2.2 Dev Workflow Gate Rules

**File**: `crates/poria-core/src/gates.rs` (edit)

- [ ] Add gate: `prd_review_p0` — Block at Design entry, checks PRD_REVIEW.md P0 completion
- [ ] Add gate: `trd_exists` — Block at Dev entry, checks TRD.md exists
- [ ] Add gate: `code_changes_exist` — Block at Cr entry, checks git diff non-empty
- [ ] P0 completion checker: parse PRD_REVIEW.md, find P0 section, verify no `TODO`/`待填写`/empty answers

### 2.3 AppState Wiring

**File**: `src-tauri/src/main.rs` or `src-tauri/src/lib.rs` (edit)

- [ ] Add `ClaudeAgentPool` (with `ClaudeCliSdk`) to `AppState`
- [ ] Add `SessionTracker` to `AppState`
- [ ] Add `SkillLoader` (real implementation) to `AppState`
- [ ] Add `HumanLoopCoordinator` (with Tauri event bridge) to `AppState`
- [ ] Wire `PipelineExecutor` with all real implementations

### 2.4 Stage Execution Tauri Command

**File**: `src-tauri/src/commands/pipeline.rs` (edit)

- [ ] Add `execute_stage` command — triggers execution of next pending stage
- [ ] Add `retry_stage` command — retries a failed stage
- [ ] Add `skip_stage` command — marks a stage as skipped
- [ ] Ensure `submitPipeline` creates FeatureContext alongside pipeline

### 2.5 HumanLoop Tauri Bridge

**File**: `crates/poria-commands/src/human_loop_bridge.rs` (new)

- [ ] Implement `HumanLoop` trait backed by Tauri events
- [ ] `notify()` → emit `human:request` event with stage info + prompt
- [ ] `poll_reply()` → await on `tokio::sync::oneshot` channel, fed by `humanLoopRespond` command
- [ ] Handle timeout (configurable, default 24h)

---

## Child 3: Stage Skill Implementations

### 3.1 ReviewPrdSkill

**File**: `crates/poria-skills/src/review_prd.rs` (edit existing stub)

- [ ] Read PRD from feature context
- [ ] Render `prd_review.md` template with PRD content + feature dir
- [ ] Dispatch to agent pool with `prd-review` stage config
- [ ] Check PRD_REVIEW.md created as gate output
- [ ] Return P0 question count in output for gate evaluation

### 3.2 GenTrdSkill

**File**: `crates/poria-skills/src/gen_trd.rs` (edit existing stub)

- [ ] Read PRD.md + PRD_REVIEW.md from feature context
- [ ] Optionally read API.md, ui/ if present
- [ ] Render `trd_gen.md` template
- [ ] Dispatch to agent pool with `design` stage config
- [ ] Check TRD.md created

### 3.3 GenCodeSkill

**File**: `crates/poria-skills/src/gen_code.rs` (edit existing stub)

- [ ] Read TRD.md + PRD.md from feature context
- [ ] Render `code_impl.md` template
- [ ] Phase 1: Dispatch main agent to generate TASK.md
- [ ] Human gate: wait for TASK.md approval
- [ ] Phase 2: Parse TASK.md tasks, dispatch SubAgents per T-n respecting dependencies
- [ ] Run OutputGuard on each SubAgent output
- [ ] Aggregate results

### 3.4 CodeReviewSkill

**File**: `crates/poria-skills/src/code_review.rs` (edit existing stub)

- [ ] Generate git diff for feature branch
- [ ] Render `cr_web.md` template with diff + context
- [ ] Dispatch to agent pool with `cr` stage config
- [ ] Parse CR score from output for gate evaluation (B+ threshold)
- [ ] Write CR.md to feature context

---

## Child 4: Frontend Workflow UI

### 4.1 Stream Output Component

**File**: `src/components/StreamOutput.tsx` (new)

- [ ] Listen to `agent:stream` Tauri events
- [ ] Render text deltas as they arrive (typewriter effect)
- [ ] Collapsible sections for tool_use / tool_result
- [ ] Auto-scroll with manual scroll lock
- [ ] Markdown rendering for Claude text output

### 4.2 Workflow Panel Enhancements

**Files**: `src/components/StageProgress.tsx`, `src/components/PipelineDetail.tsx` (edit)

- [ ] Show dev workflow stages with clear visual status
- [ ] Stage status: pending → running → waiting-review → completed / failed / skipped
- [ ] Running stage shows StreamOutput
- [ ] Waiting-review stage shows HumanLoopCard with context
- [ ] Feature context file browser (view PRD_REVIEW.md, TRD.md, etc.)

### 4.3 Enhanced HumanLoopCard

**File**: `src/components/HumanLoopCard.tsx` (edit)

- [ ] Show stage-specific context (e.g., PRD scope for ReviewPrd, TRD content for Design)
- [ ] Three actions: approve / reject / request-changes
- [ ] For PRD Review: inline P0 question editor
- [ ] For TASK.md review: task list with checkboxes

### 4.4 Tauri IPC Bindings

**File**: `src/lib/tauri.ts` (edit)

- [ ] Add `executeStage` binding
- [ ] Add `retryStage` binding
- [ ] Add `skipStage` binding
- [ ] Add stream event listener setup

### 4.5 State Updates

**File**: `src/state/store.tsx` (edit)

- [ ] Add `streamOutput` to state (per-stage stream buffer)
- [ ] Handle `agent:stream` events in reducer
- [ ] Handle stage status transitions for dev workflow stages

---

## Validation Commands

```bash
# Rust compilation
cargo check --workspace
cargo clippy --workspace

# Rust tests
cargo test --workspace
cargo test -p poria-resources -- cli_sdk
cargo test -p poria-skills -- review_prd
cargo test -p poria-core -- feature_context
cargo test -p poria-core -- gates

# Frontend
pnpm typecheck

# Integration (manual)
pnpm tauri dev
# → Submit a pipeline with a PRD → verify each stage invokes Claude
```

## Review Gates

- After Child 1: `cargo test -p poria-resources` passes, `ClaudeCliSdk` can parse stream-json
- After Child 2: `cargo test -p poria-core` passes, gates evaluate correctly
- After Child 3: `cargo test -p poria-skills` passes, each skill produces expected output format
- After Child 4: `pnpm typecheck` passes, workflow UI renders stages with mock data
- Final: `pnpm tauri dev` runs full pipeline end-to-end
