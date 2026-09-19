# Thinking Guides

> **Purpose**: Expand your thinking to catch things you might not have considered.

---

## Why Thinking Guides?

**Most bugs and tech debt come from "didn't think of that"**, not from lack of skill:

- Didn't think about what happens at layer boundaries → cross-layer bugs
- Didn't think about code patterns repeating → duplicated code everywhere
- Didn't think about edge cases → runtime errors
- Didn't think about future maintainers → unreadable code

These guides help you **ask the right questions before coding**.

---

## Available Guides

| Guide                                                         | Purpose                                  | When to Use                       |
| ------------------------------------------------------------- | ---------------------------------------- | --------------------------------- |
| [Code Reuse Thinking Guide](./code-reuse-thinking-guide.md)   | Identify patterns and reduce duplication | When you notice repeated patterns |
| [Cross-Layer Thinking Guide](./cross-layer-thinking-guide.md) | Think through data flow across layers    | Features spanning multiple layers |

---

## Quick Reference: Thinking Triggers

### When to Think About Cross-Layer Issues

- [ ] Feature touches 3+ layers (API, Service, Component, Database)
- [ ] Data format changes between layers
- [ ] Multiple consumers need the same data
- [ ] You're not sure where to put some logic
- [ ] You are adding an event kind, JSONL record, RPC payload, or config field
- [ ] UI / command code starts casting raw payload fields directly
- [ ] Xingyun demand list filter changes (`acceptedByMe` / `receiver` / `processor`)
- [ ] Start-pipeline wizard / `submit_pipeline` / `backend_trd_url` / `BACKEND_TRD.md` vs frontend `TRD.md`
- [ ] Init `~/.poria/workspaces/<id>` vs `~/.poria/projects` docs vs `pipeline.repos` vs registered `default_branch` sync (`workspacePath` ≠ frontend `worktreePath`)
- [ ] Design entry `prd_review_p0`: unanswered P0 (blank / TODO / 待填写 / 待确认) Blocked; desktop `write_demand_project_file` + HITL resume; P1/P2 warn only
- [ ] Dev entry `trd_confirmed`: frontend `TRD.md` must exist and be confirmed (or explicitly skipped); HITL 「确认 TRD」 / 「跳过确认」
- [ ] Dev exit OutputGuard: Design `trd_scope` / TRD `## 允许修改范围`; Block out-of-scope files or blocked deps — no CR
- [ ] Dev local verify after OutputGuard: package.json / Settings commands in the frontend worktree; `LocalVerifyError` retries then HITL; no CR
- [ ] CI / coverage / security gates: EasyCI/Coding pipeline + report files / npm audit; missing data is fail, never agent self-score
- [ ] Independent CR reviewer: new Claude `-p` session + reviewer system prompt; P0/P1 posted as Coding MR notes after Deploy
- [ ] Exception routing: `ISSUE_POLICIES` retry / CR regress once / HITL shows issue class, not a generic fail
- [ ] HITL: Blocked → JME notify + desktop card; first reply wins; escalate_at then degrade to manual
- [ ] Xingyun writeback on WaitingMerge/Completed/Failed/Cancelled; quality-gate defect; unmerged close MR / merged revert MR (no auto-merge)
- [ ] Desktop PipelineWorker: file lock + recover RUNNING + SQLite queue shared with silent auto-run (no second in-memory executor); up to N in-flight (default 2), Blocked does not hold a slot
- [ ] Local observability: SQLite summary of stage success, p50/p95 duration, cost USD, HITL rate on 看板/设置; single pipeline still shows events + cost
- [ ] Deploy success is WaitingMerge; poll MR; 一键确认 comments merge-ready and never clicks Merge
- [ ] `claude_path` / `probe_claude` / Agent `spawn("claude")` / `~/.poria/config.json` vs CWD `poria.config.json`

→ Read [Cross-Layer Thinking Guide](./cross-layer-thinking-guide.md)

### When verifying the desktop app

- [ ] Does the flow call `invoke`, `listen`, git clone, Xingyun, SSO, or `submit_pipeline`?
- [ ] If yes, is this the `poria-desktop` window (not Cursor browser on port 1420)?
- [ ] Is port 1420 free, and was the app started with `cargo tauri dev` (not `pnpm tauri` unless a `tauri` binary is on PATH)?
- [ ] If starting a pipeline, does invoke include `backendTrdUrl`, and is backend still out of `pipeline.repos`?
- [ ] After Init: `~/.poria/workspaces/<id>/` has doc+skill symlinks, `CLAUDE.md`, frontend feature worktree + backend detached worktree before ReviewPrd; docs only under `~/.poria/projects/<demand_code>/` (see [Pipeline Init Workspace](../frontend/pipeline-workspace.md))
- [ ] If ReviewPrd/Design/Dev/Cr: Agent cwd is the workspace root, short `-p` names the skill; Settings Claude status still Poria window only (see [Claude CLI Path](../frontend/claude-cli.md) and [Pipeline Init Workspace](../frontend/pipeline-workspace.md))

→ Read [Tauri Desktop Testing](../frontend/tauri-desktop-testing.md)

### When to Think About Code Reuse

- [ ] You're writing similar code to something that exists
- [ ] You see the same pattern repeated 3+ times
- [ ] You're adding a new field to multiple places
- [ ] **You're modifying any constant or config**
- [ ] **You're creating a new utility/helper function** ← Search first!
- [ ] Two files read the same untyped payload field with local casts
- [ ] Multiple branches update the same derived state from `kind` / `action`

→ Read [Code Reuse Thinking Guide](./code-reuse-thinking-guide.md)

### When Verifying AI Cross-Review Results

- [ ] Reviewer claims "user input can be malicious" → Check the actual data source (internal manifest? user config? external API?)
- [ ] Reviewer flags "missing validation" → Is the data from a trusted internal source?
- [ ] Reviewer says "behavior change" → Read the code comments — is it intentional design?
- [ ] Reviewer identifies a "bug" in test → Mentally delete the feature being tested — does the test still pass? If yes → tautological test

**Common AI reviewer false-positive patterns**:

1. **Trust boundary confusion**: Treating internal data (bundled JSON manifests) as untrusted external input
2. **Ignoring design comments**: Flagging intentional behavior documented in code comments as bugs
3. **Variable misreading**: Not tracing a variable to its actual definition (e.g., Map keyed by path vs name)

**Verification rule**: Every CRITICAL/WARNING finding must be verified against the actual code before prioritizing. Budget ~35% false-positive rate for AI reviews.

---

## Pre-Modification Rule (CRITICAL)

> **Before changing ANY value, ALWAYS search first!**

```bash
# Search for the value you're about to change
grep -r "value_to_change" .
```

This single habit prevents most "forgot to update X" bugs.

---

## How to Use This Directory

1. **Before coding**: Skim the relevant thinking guide
2. **During coding**: If something feels repetitive or complex, check the guides
3. **After bugs**: Add new insights to the relevant guide (learn from mistakes)

---

## Contributing

Found a new "didn't think of that" moment? Add it to the relevant guide.

---

**Core Principle**: 30 minutes of thinking saves 3 hours of debugging.
