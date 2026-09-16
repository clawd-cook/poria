# harness-o2o-fe Skills Inventory

> Source: `@jd/harness-o2o-fe@0.8.4` (globally installed npm package)
> Skills location: `~/.claude/skills/hfe-*` (8 skills total)
> Package: `~/.nvm/versions/node/v24.20.0/lib/node_modules/@jd/harness-o2o-fe/`

---

## Pipeline Overview (Execution Order)

```
hfe-init → hfe-review-prd → hfe-gen-trd → hfe-execute → hfe-cr-app / hfe-cr-web
                                ↑               ↑
                           hfe-gen-api      hfe-gen-ui
                           (sub-skill)      (sub-skill)
```

---

## 1. hfe-init — Project Initialization & Deep Analysis

**Purpose**: Initialize or upgrade the harness-o2o-fe framework in a frontend project. Scaffolds `.harness/` directory, detects project shape (single-app vs monorepo), and performs deep analysis to populate 4 core documents.

**Inputs**:
- Target project directory
- User confirmations for: project shape (single/monorepo), APP_TYPE (C_SIDE/B_SIDE/B_SIDE_REACT/RN_SIDE/FLUTTER_SIDE/HYBRID), runtime params

**Outputs**:
- `.harness/` skeleton: `ARCHITECTURE.md`, `CAPABILITIES.md`, `COMPONENT_INVENTORY.md`, `CONVENTIONS.md`
- `.harness/manifest.json` (app_name, app_type, tooling.cli_version, last_updated)
- `.harness/templates/{TRD,API}_TEMPLATE.md`
- `.harness/features/` directory structure
- Gates injected into `~/.codex/AGENTS.md`, `~/.claude/CLAUDE.md`, `~/.cursor/plugins/`

**Key Scripts**:
- `scripts/detect-runtime.sh` — project signal detection
- `scripts/locate-skeleton.sh` — skeleton root resolution
- `scripts/post-init-check.sh` — post-init validation
- `scripts/lib/scan-components.js` — component scanning
- `scripts/lib/scan-design-tokens.js` — design token scanning
- `scripts/lib/taro-platform-lint.js` — multi-platform lint
- `assets/skeleton/scripts/bootstrap-project.sh` — scaffold creation
- `assets/skeleton/scripts/add-app-to-monorepo.sh` — incremental app addition
- `assets/skeleton/scripts/upgrade-skeleton.sh` — version upgrade

**Dependencies**: None (entry point)

**Metrics**: `analyze_start` / `analyze_done` events via `~/.harness/scripts/metrics/`

---

## 2. hfe-review-prd — PRD Frontend Landing Clarification

**Purpose**: Analyze a PRD from the frontend perspective, decompose into functional modules, confirm scope with user, and produce a clarification checklist (gaps, corrections, cross-module consistency).

**Inputs**:
- PRD source (JoySpace link / file path / pasted content)
- Feature directory (auto-created if needed: `.harness/features/feat-NNN-<slug>/`)

**Outputs**:
- `<feature>/PRD_REVIEW.md` — two chapters:
  1. "本期范围" (in-scope/excluded modules with reasons + linked Q numbers)
  2. "前端落地澄清清单" (P0/P1/P2 grouped Q/A checklist)
- `<feature>/PRD.md` (if JoySpace fetch was done)
- `<feature>/images/` (if JoySpace images fetched)

**Key Scripts**:
- JoySpace fetch: `archive` command reads macOS Chrome cookies for `jd.com`

**Interactive Steps**:
1. Module decomposition (small-req escape hatch)
2. Scope gate — hard wait for user to exclude modules
3. Frontend clarification (GAP/FIX/XMOD per in-scope module)
4. Optional AskQuestion interaction for enumerable P0/P1 questions
5. Write PRD_REVIEW.md

**Dependencies**: None (can be entry point after hfe-init)

**Metrics**: `stage_start`/`stage_done` for `request-analysis` stage, `prd_clarify_done` event

---

## 3. hfe-gen-trd — Generate Technical Design Document

**Purpose**: From PRD + UI (or prototypes), produce a frontend technical design document (TRD.md). API docs are optional — supports "UI-only" and "API-connected" dual modes.

**Inputs**:
- Feature directory (must be confirmed)
- `PRD.md` (required)
- `PRD_REVIEW.md` (read if exists — gaps, clarifications)
- API mode decision (no-API vs has-API)
- UI mode decision (no-UI vs has-UI)
- Scope selection (user picks which frontend items are in-scope)
- Target repository (current git root)

**Outputs**:
- `<feature>/TRD.md` — structured tech design with:
  - Page details, interface lists, functional boundaries, mermaid flowcharts
  - Code location tree with naming
  - Appendix B (grilling results)
  - Undetermined items
  - Change log

**Key Scripts**:
- `scripts/validate-trd.py` — structural completeness check (required gate)
- `references/_template.md` — TRD structure template
- `references/_grill-workflow.md` — grilling methodology
- `references/_review-prompt.md` — adversarial review prompt

**Interactive Steps**:
1. Confirm feature dir, API mode, UI mode, scope
2. Read template + cross-parse materials
3. Information classification
4. Grilling pressure test (4 questions per batch, all G-n must be resolved)
5. Write TRD.md
6. Schema validation (must PASS)
7. One-round adversarial review (isolated subagent, no self-review)
8. Answer review findings
9. Re-validate and deliver

**Dependencies**: 
- hfe-review-prd (upstream, optional but consumed if exists)
- hfe-gen-api (called when user chooses "has API docs")
- hfe-gen-ui (called when user chooses "has UI")

**Metrics**: `stage_start`/`stage_done` for `tech-design` stage, `trd_reuse` event

---

## 4. hfe-gen-api — Generate API Contract Document

**Purpose**: From JoySpace API docs or pasted interface info, produce a structured API contract document (API.md). Purely contract — no frontend implementation logic.

**Inputs**:
- JoySpace link OR pasted API info (at least one required)
- Feature directory
- `PRD.md` (optional, for semantic validation)
- `PRD_REVIEW.md` (optional, contract-related gaps)

**Outputs**:
- `<feature>/API.md` — interface contracts:
  - Per-interface: metadata, field tables, examples, error codes
  - Appendix B (grilling results)
  - Change log
- `<feature>/source/` — raw API material backup (read-only fact source)

**Key Scripts**:
- Depends on `joyspace-to-markdown` skill for JoySpace export
- `references/_template.md` — API doc template
- `references/_grill-workflow.md` — grilling methodology

**Interactive Steps**:
1. Validate input, confirm feature dir
2. Obtain raw materials to `source/`
3. Parse per template constraints
4. Grilling contract stress-test (4-per-batch gate)
5. Write API.md
6. Self-check and deliver

**Dependencies**:
- `joyspace-to-markdown` skill (for JoySpace link processing)
- Called by hfe-gen-trd when user has API docs

---

## 5. hfe-gen-ui — Put UI Assets into Feature Directory

**Purpose**: Sub-skill of hfe-gen-trd. Places UI materials into the same feature directory as TRD. NOT standalone — always orchestrated by hfe-gen-trd.

**Inputs**:
- Feature directory (passed from hfe-gen-trd)
- UI source choice (one of three):
  1. Deco MCP — fetch generated UI code from Relay
  2. Manual copy — user copies files into `ui/`
  3. tool-joyrelay — fetch prototype files from Relay design

**Outputs**:
- `<feature>/ui/` — UI materials (code or images)
- Returns `ui/` absolute path to hfe-gen-trd

**Dependencies**:
- Deco MCP (dynamic tools: `user-Deco-prod-mcp`)
- tool-joyrelay skill (for design file fetching)
- Must be called from hfe-gen-trd (not standalone)

---

## 6. hfe-execute — Document-Driven Code Implementation

**Purpose**: From TRD (+ PRD + API), generate a TASK.md execution plan, then orchestrate isolated SubAgents to implement code and run quality gates.

**Inputs**:
- Feature directory (required)
- `TRD.md` (required — main source for task splitting)
- `PRD.md` (required)
- `API.md` (required if TRD is "API-connected" mode)
- `PRD_REVIEW.md` (read if exists)
- `ui/` (if exists, must be included in briefing)

**Outputs**:
- `<feature>/TASK.md` — execution plan with:
  - Task items (T-1, T-2, ...) with status, scope, dependencies, files, acceptance
  - Confirmed open-item handling
  - Schedule batches
  - Gate records
  - Blockers
- Business code (written by SubAgents, not main agent)

**Execution Model**:
1. Main agent: validates input, extracts open items, creates TASK.md
2. User reviews TASK.md
3. Main agent dispatches implementation SubAgents per T-n (respecting dependencies)
4. One review SubAgent runs quality gates after all implementations
5. Main agent handles review findings (up to 2 additional rounds)

**Hard Constraints**:
- Main agent NEVER writes business code
- SubAgents are isolated (generalPurpose type)
- Must pass quality gates before marking [x]
- Review SubAgent must not modify business code

**Dependencies**:
- hfe-gen-trd (upstream, provides TRD.md)
- hfe-gen-api (upstream, provides API.md)

**Metrics**: `stage_start`/`stage_done` for `coding-impl`, task snapshots per T-n, `self_check`, `review_result`, optional `satisfaction_score`

---

## 7. hfe-cr-app — C-Side Code Quality Tool (App CR)

**Purpose**: C-side (Taro) specialized code review. Two commands: incremental review (`hfe-cr-app`) and full-scan (`hfe-scan-app`).

**Incremental Review (hfe-cr-app)**:
- Two modes: local walkthrough (no token) and MR review (needs Coding API Token)
- Local uses two-stage: `preflight` (detect platforms/branch/tracking/data-source) → user confirms → `local-scan`
- 12 review dimensions: NS, LG, ST, PF, SEC, ERR, DEP, API, TRK, MOCK, DU, CP
- FE-AST-* deterministic checks (left-shift from MR gate)
- validator + adversarial challenge pipeline
- Report uploaded to `cctv.jd.com`

**Full Scan (hfe-scan-app)**:
- Scans entire repo, module by module
- 4-round progressive scan per unit (deterministic → semantic → architecture → adversarial)
- Multi-tech-stack: frontend, iOS, Android, HarmonyOS
- Health score calculation, TOP10, risk heatmap

**Key Scripts**:
- `scripts/mr_review.py` — all commands (init, preflight, local-scan, hfe-cr-app, full-scan, upload-report, etc.)
- `scripts/validate_findings.py` — post-review validation
- `scripts/challenge_findings.py` — adversarial challenge generation
- `scripts/enrich_context.py` — call chain / blast radius context
- `scripts/init_cr_report.py` — report template
- `scripts/detect_platforms.py` — platform/tracking/data-source detection

**Outputs**: `cr-reports/<report>.md` (incremental) or `cr-reports/full-scan/<timestamp>/` (full scan)

**Dependencies**: None directly (can run independently)

---

## 8. hfe-cr-web — Web/B-Side Code Review

**Purpose**: Architecture-first code review for web/B-side projects. Supports local review and cloud AI CR queue submission.

**Modes**:
- Local review: pulls MR context, creates head_sha snapshot, reviews against rule set, generates report
- Cloud AI CR: submits to remote queue only (no local analysis)

**Key Features**:
- Dynamic rule set from `CR_RULES_URL` (positive/negative rules)
- Multi-agent mode for large MRs (>3 files or ≥3 modules)
- Severity rubric-based finding classification
- Structured upload to `cctv.jd.com` / `h2o.jd.com`

**Key Scripts**:
- `scripts/mr_review.py` — Token, MR parsing, snapshot, report init, upload, cloud-ai-cr
- `scripts/init_cr_report.py` — report template

**Outputs**: `cr-reports/<report>.md`

**Dependencies**: None directly

---

## Cross-Cutting Concerns

### Metrics System
All skills report events via `~/.harness/scripts/metrics/`:
- `bind-change.sh` — bind feature session
- `lib/report-event.sh` — report lifecycle events
- `lib/task-snapshot.py` — task-level snapshots
- `flush-metrics.sh` — aggregate and upload

### Gate System (Global CLAUDE.md)
Two gates defined in `~/.claude/CLAUDE.md`:
1. **hfe-gen-trd gate**: Requires `PRD_REVIEW.md` with all P0 questions answered
2. **hfe-execute gate**: Requires `TRD.md` in feature directory

### Feature Directory Convention
All document-driven skills share: `.harness/features/<feat>/`
```
.harness/features/feat-001-batch-export/
├── PRD.md          (hfe-review-prd or user)
├── PRD_REVIEW.md   (hfe-review-prd)
├── API.md          (hfe-gen-api)
├── TRD.md          (hfe-gen-trd)
├── TASK.md         (hfe-execute)
├── CR.md           (hfe-cr-app / hfe-cr-web)
├── source/         (hfe-gen-api raw materials)
├── ui/             (hfe-gen-ui)
└── images/         (JoySpace fetch)
```

### JoySpace Integration
- `joyspace-to-markdown` skill — exports JoySpace documents
- Used by hfe-review-prd (PRD fetch) and hfe-gen-api (API doc fetch)
- Reads macOS Chrome cookies for `jd.com`

### Deco / Relay Integration
- Deco MCP — fetch generated UI code (via `user-Deco-prod-mcp` dynamic tool)
- tool-joyrelay — fetch design prototype files
- Both used by hfe-gen-ui

### Monorepo Support
All skills detect and adapt to monorepo structure:
- Single `.harness/` at project root
- `ARCHITECTURE.md` MONOREPO_APPS block for app registry
- Per-app analysis in 4 core documents
- TRD frontmatter `app:` field for feature attribution
