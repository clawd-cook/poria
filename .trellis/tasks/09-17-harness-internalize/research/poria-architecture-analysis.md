# Poria Architecture Analysis for Harness Internalization

## 1. Claude Agent Invocation (`poria-resources`)

### Agent Pool (`crates/poria-resources/src/claude/agent_pool.rs`)

- **`ClaudeAgentPool`**: Manages concurrent Claude CLI subprocess dispatches via `tokio::Semaphore`.
- **`AgentSdk` trait** (mockable): `query(prompt, options) -> (Vec<SdkMessage>, Option<session_id>)` — the core SDK abstraction.
- **`AgentQueryOptions`**: `cwd`, `max_budget_usd`, `max_turns`, `model`, `append_system_prompt`, `allowed_tools`.
- **`StageAgentConfig`**: Per-stage agent configs (tools, budget, turns, timeout). Currently 4 stages configured:
  - `ReviewPrd` — Read/Grep, $1, 10 turns, 5min
  - `Design` — Read/Edit/Grep, $3, 20 turns, 10min
  - `Dev` — Read/Edit/Bash/Glob/Grep, $10, 100 turns, 30min
  - `Cr` — Read/Bash/Grep, $5, 30 turns, 15min
- **`dispatch(AgentTaskInput) -> AgentTaskResult`**: Acquires semaphore, calls SDK, handles timeout (30min default + 5min idle).
- **Extension point**: The `AgentSdk` trait is the injection point. A real implementation would call `claude` CLI or Anthropic SDK. Currently only `MockSdk` exists in tests.

### Session Tracker (`session_tracker.rs`)

- In-memory `HashMap<(pipeline_id, stage_name), session_id>`.
- Used for agent resume support — can reattach to existing sessions.

### Output Guard (`output_guard.rs`)

- Validates agent output against: file scope (glob patterns), diff size (500 lines threshold), blocked dependencies.
- Returns `GuardResult { pass, violations }` with `Block` or `Warn` severity.

## 2. Skills / Stage Mapping (`poria-skills`)

### Stage-to-Skill Map (`stage_skill_map.rs`)

```
Init        → skill:init          (parse demand link, export PRD)
ReviewPrd   → skill:review-prd    (analyze PRD → PRD_REVIEW.md with P0/P1/P2)
Design      → skill:gen-trd       (generate TRD.md + trdScope)
Workspace   → skill:workspace     (git worktree + xingyun branch)
Dev         → skill:gen-code      (agent code generation + OutputGuard)
Cr          → skill:code-review   (agent CR + security scan)
Deploy      → skill:deploy        (build, push, create MR)
```

### Skill Trait (`poria-core/contracts/skill.rs`)

```rust
trait Skill: Send + Sync {
    fn metadata(&self) -> &CapabilityMetadata;
    async fn execute(SkillInput, SkillContext) -> Result<SkillOutput>;
}
```

- `SkillInput`: contains `Stage` + `Pipeline` + arbitrary `extra` map
- `SkillContext`: `pipeline_id`, `workdir`, `credentials`
- `SkillOutput`: `output: serde_json::Value` + optional `gates_pass`

### Current Skill Implementation Status

**All 7 skills have the same pattern**: in fixture mode they return hardcoded JSON; in production they return `SkillError::NotImplemented`. **This is the primary gap to fill** — connecting each skill to the Claude agent pool with the right prompt and SKILL content.

### Human-in-the-Loop (`human_loop.rs`)

- `HumanLoop` trait: `notify`, `renotify`, `escalate`, `poll_reply`
- `HumanAction`: Resume, Skip, Cancel (with CJK pattern matching)
- `HumanLoopCoordinator`: stub implementation (fixture mode only)

## 3. Pipeline Engine (`poria-core`)

### State Machine (`state_machine.rs`)

Pipeline states: `Created → Running → {WaitingMerge, Blocked, Failed, Cancelled} → {Completed, Cancelled}`

Stage states: `Pending → Running → {Completed, Failed, Blocked, Skipped}`
- `Failed → Running` (retry), `Blocked → Running` (resume)

### Gates (`gates.rs`)

6 default gate rules:
- `ci_build` (Block, Deploy phase)
- `test_coverage` (Block, Deploy phase, threshold 80)
- `security_scan` (Block, StageExit phase)
- `diff_size` (Warn, Deploy phase, threshold 500)
- `merge_conflict` (Block, Deploy phase)
- `cr_score` (Regress to Dev, StageExit phase, threshold B+)

`evaluate_gates(StageResult, rules, phase) -> GateEvaluation` with blocking/warning failure detection.

### Events (`events.rs`)

Rich event system covering: Pipeline lifecycle, Stage lifecycle, Agent dispatch/progress/completion, Gate evaluation, Human-in-the-loop, Credentials, Git/MR/Worktree operations, Rollback.

### Risk Classifier and Multi-repo

Additional modules for risk classification and multi-repo support.

## 4. Command Dispatch (`poria-commands`)

### PipelineExecutor (`executor.rs`)

The main orchestration engine:
1. Loads pipeline from store
2. Iterates stages in order
3. For each stage: validates credentials → transitions to Running → loads skill via `SkillLoader` → executes → builds rollback instructions
4. Special handling for CR gate (regress to Dev if score < B+, max 1 regress) and Deploy gate
5. Error handling: retry, blocked, or failed based on exception classification

**Key traits** (`traits.rs`):
- `PipelineStore`: load/save/find pipelines
- `SkillLoader`: `load(skill_id) -> Box<dyn Skill>` — **the hook for connecting skills to Claude**
- `CredentialGuard`: ensure valid auth tokens
- `MultiRepoOrchestrator`: orchestrate across repos
- `HumanLoop`, `CodingChannel`, `Terminal`, `Messenger`, `FileSystem`, `FileLock`, `Queue`, `Recovery`

### PipelineWorker (`worker.rs`)

Background worker that:
- Consumes pipeline queue (5s poll interval)
- Polls MR merge status (60s interval)
- Escalates stale MRs (>24h)
- Handles crash recovery

## 5. Frontend State (`src/state/`)

### Store (`store.tsx`)

React Context + useReducer pattern with `AppState`:
- `pipelines`, `selectedPipelineId`, `pipelineDetail`
- `events` (live event stream)
- `humanRequest` (pending human-in-the-loop)
- `auth`, `config`, `sidecar` (backend status)
- `skills`, `channels` (capability registry)
- `ui` (filter, settings, view: pipeline | skills | channels)

Listens to Tauri events: `pipeline:list-changed`, `pipeline:updated`, `stage:progress`, `human:request`, `sidecar:status`, `auth:status-changed`.

### Tauri IPC Commands (`src/lib/tauri.ts`)

Current commands: `listPipelines`, `getPipeline`, `submitPipeline`, `cancelPipeline`, `humanLoopRespond`, `getAuthStatus`, `getConfig`, `updateConfig`, `listSkills`, `listChannels`.

## 6. Frontend UI Components

| Component | Purpose |
|-----------|---------|
| `Shell.tsx` | Main layout shell |
| `PipelineSidebar.tsx` | Pipeline list sidebar |
| `PipelineDetail.tsx` | Selected pipeline detail |
| `StageProgress.tsx` | Stage progress indicators |
| `EventStream.tsx` | Live event stream |
| `GateResults.tsx` | Gate evaluation results |
| `HumanLoopCard.tsx` | Human-in-the-loop interaction |
| `SubmitBar.tsx` | Pipeline submission |
| `AuthStatus.tsx` | Auth status indicator |
| `SettingsPanel.tsx` | Config panel |
| `SkillsPage.tsx` | Skills registry view |
| `ChannelsPage.tsx` | Channels registry view |
| `StatusBadge.tsx` | Status indicator badge |

## Key Extension Points for Harness Internalization

### What Needs to Be Built

1. **Real `AgentSdk` implementation** — Bridge `ClaudeAgentPool` to actual Claude CLI/SDK invocations. The `AgentSdk` trait and `AgentQueryOptions` already support `append_system_prompt` (for SKILL injection) and `allowed_tools`.

2. **Skill implementations** — Each of the 7 skills currently returns `NotImplemented`. They need to:
   - Construct the right prompt (with harness SKILL content as system prompt)
   - Call `ClaudeAgentPool.dispatch()` with stage-appropriate `AgentTaskInput`
   - Parse the agent's result into the expected `SkillOutput` format
   - For `GenCodeSkill`: run `OutputGuard.check()` on the result

3. **SKILL content storage** — The harness `hfe-*` skills currently live as Claude Code skill files. They need to be embedded in or loaded by the Poria app as prompt templates for each stage.

4. **`SkillLoader` implementation** — Connect the `PipelineExecutor` to the actual skill instances (currently `list_skills` command hardcodes them, but executor needs a real loader).

5. **`AppState` in Tauri** — Currently only has `store` and `event_store`. Needs `ClaudeAgentPool`, `SessionTracker`, and eventually `CredentialGuard`, `HumanLoopCoordinator` wired in.

### Architecture Compatibility

The existing architecture is **very well-suited** for this integration:
- The `Skill` trait and `SkillLoader` pattern provide clean injection points
- `ClaudeAgentPool` already handles concurrency, timeout, and session tracking
- `StageAgentConfig` already defines per-stage tool/budget/turn configs
- `OutputGuard` validates agent output for the Dev stage
- The event system tracks all agent lifecycle events
- Human-in-the-loop is modeled but not yet connected to the UI
- The state machine and gate system handle flow control and quality gates

### What's Missing vs. Harness

The harness `@jd/harness-o2o-fe` provides:
- `hfe-review-prd`: PRD analysis → PRD_REVIEW.md (maps to `ReviewPrdSkill`)
- `hfe-gen-trd`: TRD generation with gate checks (maps to `GenTrdSkill`)
- `hfe-execute`: Code generation from TRD (maps to `GenCodeSkill`)
- `hfe-cr-web`: Code review with MR integration (maps to `CodeReviewSkill`)
- `hfe-init`: Project initialization (maps to `InitSkill`)
- `hfe-gen-api`: API generation (new capability, could be a sub-skill of Gen)
- `hfe-gen-ui`: UI generation (new capability, could be a sub-skill of Gen)

The key difference: harness skills run as Claude Code skill invocations (the host Claude loads and executes the skill prompt). Poria needs to do the same thing but as a Tauri desktop app — spawning `claude` CLI processes with the skill content injected as system prompts.
