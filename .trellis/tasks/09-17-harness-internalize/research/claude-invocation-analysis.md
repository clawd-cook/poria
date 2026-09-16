# Claude Invocation & Skill-Based Orchestration — Analysis

## Current Architecture

### 1. Claude Agent Pool (`crates/poria-resources/src/claude/agent_pool.rs`)

The pool is the central piece for invoking Claude:

- **`ClaudeAgentPool`** wraps an `Arc<dyn AgentSdk>` behind a `Semaphore` for concurrency control.
- **`AgentSdk` trait** (line 101): single method `query(prompt, options) -> (Vec<SdkMessage>, Option<session_id>)`.
- **`AgentQueryOptions`** (line 87): `cwd`, `max_budget_usd`, `max_turns`, `model`, `append_system_prompt`, `allowed_tools`.
- **`dispatch(input: AgentTaskInput)`**: acquires semaphore, builds `AgentQueryOptions` from input, wraps in `tokio::time::timeout`, returns `AgentTaskResult`.

**Key: `append_system_prompt`** — This is the hook for injecting skill-specific instructions into Claude. It maps directly to `AgentTaskInput.system_prompt`.

### 2. Agent Task Types (`crates/poria-core/src/types/agent.rs`)

```rust
pub struct AgentTaskInput {
    pub prompt: String,           // The user prompt / task description
    pub worktree_path: String,    // Working directory for the agent
    pub system_prompt: Option<String>,  // ← SKILL INSTRUCTIONS GO HERE
    pub model: Option<String>,
    pub max_budget_usd: Option<f64>,
    pub max_turns: Option<i32>,
    pub timeout_ms: Option<i64>,
    pub extra_tools: Option<Vec<String>>,  // Allowed tool whitelist
}
```

### 3. Stage-to-Skill Mapping (`crates/poria-skills/src/stage_skill_map.rs`)

Static map from pipeline stages to skill IDs:

| Stage | Skill ID |
|-------|----------|
| Init | `skill:init` |
| ReviewPrd | `skill:review-prd` |
| Design | `skill:gen-trd` |
| Workspace | `skill:workspace` |
| Dev | `skill:gen-code` |
| Cr | `skill:code-review` |
| Deploy | `skill:deploy` |

### 4. Per-Stage Agent Config (`agent_pool.rs:24-69`)

Budget, tools, and timeouts are configured per stage:

| Stage | Tools | Budget | Max Turns | Timeout |
|-------|-------|--------|-----------|---------|
| ReviewPrd | Read, Grep | $1 | 10 | 5 min |
| Design | Read, Edit, Grep | $3 | 20 | 10 min |
| Dev | Read, Edit, Bash, Glob, Grep | $10 | 100 | 30 min |
| Cr | Read, Bash, Grep | $5 | 30 | 15 min |

### 5. Skill Contract (`crates/poria-core/src/contracts/skill.rs`)

```rust
pub trait Skill: Send + Sync {
    fn metadata(&self) -> &CapabilityMetadata;
    async fn execute(input: SkillInput, ctx: SkillContext) -> Result<SkillOutput, ...>;
}
```

Where `SkillInput` carries `stage`, `pipeline`, and extra JSON, and `SkillContext` has `pipeline_id`, `workdir`, `credentials`.

### 6. Current Skill Implementations (`crates/poria-skills/src/`)

All seven skills (InitSkill, ReviewPrdSkill, GenTrdSkill, WorkspaceSkill, GenCodeSkill, CodeReviewSkill, DeploySkill) are **stub implementations** — they return `SkillError::NotImplemented` in non-fixture mode and fixture JSON in fixture mode.

### 7. Session Tracker (`crates/poria-resources/src/claude/session_tracker.rs`)

In-memory `HashMap<String, String>` mapping `(pipeline_id, stage_name)` to agent session IDs. Supports record/get/remove/clear. The actual persistence is handled by the stages table `agent_session_id` column.

### 8. Output Guard (`crates/poria-resources/src/claude/output_guard.rs`)

Validates agent output:
- File scope check (allowed globs from TRD)
- Diff size check (default 500 lines)
- Dependency blocklist

### 9. Tauri Command Layer (`src-tauri/src/commands/`)

- `list_skills` creates all 7 skill instances and returns metadata.
- `submit_pipeline` creates a Pipeline with all stages in Pending.
- `cancel_pipeline` + `human_loop_respond` handle lifecycle.
- **No `execute_stage` or `run_skill` command exists yet** — the actual agent dispatch loop is not wired.

### 10. Frontend (`src/lib/`)

- `tauri.ts`: thin invoke wrappers for all commands.
- `types.ts`: mirrors Rust types (PipelineSummary, PipelineDetail, StageDetail, SkillInfo).
- No frontend code for triggering skill execution or streaming agent output.

---

## Harness-o2o-fe Skills (Global `~/.claude/skills/hfe-*/`)

Eight skills that form the full dev workflow:

| Skill | Purpose | Inputs | Output |
|-------|---------|--------|--------|
| `hfe-init` | Initialize harness framework in project | Project dir | `.harness/` skeleton |
| `hfe-review-prd` | PRD clarification from FE perspective | PRD (link/file/text) | `PRD_REVIEW.md` |
| `hfe-gen-api` | Generate API contract doc | JoySpace link or pasted API | `API.md` |
| `hfe-gen-ui` | Drop UI assets into feature dir | UI files (orchestrated by gen-trd) | `ui/` directory |
| `hfe-gen-trd` | Generate frontend TRD | PRD + UI + optional API | `TRD.md` |
| `hfe-execute` | Document-driven code generation | TRD.md (required) + PRD + API | `TASK.md` + code |
| `hfe-cr-app` | C-end Taro code review | Diff or full repo scan | CR reports |
| `hfe-cr-web` | Web code review + cloud AI CR | MR link | CR report + optional cloud queue |

### Gate System

Two gates defined in `harness-o2o-fe-gates.mdc` (also mirrored in global `CLAUDE.md`):
- `hfe-gen-trd`: requires `PRD_REVIEW.md` with P0 questions answered
- `hfe-execute`: requires `TRD.md` in feature directory

---

## Gap Analysis: What Needs to Change

### Currently Missing

1. **No stage execution loop** — `submit_pipeline` creates the pipeline but nothing drives stages forward.
2. **All skills are stubs** — the `execute()` methods return `NotImplemented`.
3. **No skill content storage** — the harness skill prompts (SKILL.md content) are not loaded into the Rust skills.
4. **No `run_stage` / `execute_skill` Tauri command** — frontend can't trigger execution.
5. **No streaming** — agent messages aren't streamed to the frontend.
6. **No gate enforcement** — the gate rules from harness aren't wired into the pipeline.

### What Exists and Is Reusable

1. **`AgentSdk` trait + `ClaudeAgentPool`** — ready for real SDK integration.
2. **`append_system_prompt` / `system_prompt`** — the exact field for injecting skill prompts.
3. **`stage_agent_config()`** — per-stage tool/budget/timeout configs.
4. **`stage_to_skill_id()`** — maps stages to skill IDs.
5. **`Skill` trait** — contract is defined; implementations need filling.
6. **`OutputGuard`** — ready to validate agent output post-Dev stage.
7. **`SessionTracker`** — ready to track agent sessions for resume.
8. **`HumanLoop`** — human-in-the-loop mechanism exists.
9. **Pipeline + Stage types** — data model is complete.
10. **Tauri event system** — `app.emit()` for frontend notifications.

### Integration Path

To wire "Claude + SKILL" for each stage:

1. **Store skill prompts** — embed hfe-* SKILL.md contents as resources, or load from a configurable directory at runtime.
2. **Implement real `AgentSdk`** — wrap the Claude Code SDK (`@anthropic-ai/claude-code` npm package or the `claude` CLI binary) as a subprocess. The SDK query returns streamed messages.
3. **Fill skill `execute()` methods** — each skill reads its prompt, builds `AgentTaskInput` with `system_prompt = skill_prompt + stage_context`, calls `pool.dispatch()`, returns structured output.
4. **Add `execute_stage` Tauri command** — takes `pipeline_id` + `stage_name`, looks up skill, runs it, emits events.
5. **Add pipeline executor** — loop that advances stages, checks gates between stages, handles human-loop pauses.
6. **Stream agent output to frontend** — emit Tauri events for each `SdkMessage`.
7. **Implement gate checks** — PRD_REVIEW.md P0 check before gen-trd, TRD.md existence before execute.
