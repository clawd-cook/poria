# Frontend Development Guidelines

> Best practices for frontend development in this project.

---

## Overview

This directory contains guidelines for frontend development. Fill in each file with your project's specific conventions.

---

## Guidelines Index

| Guide                                             | Description                             | Status  |
| ------------------------------------------------- | --------------------------------------- | ------- |
| [Directory Structure](./directory-structure.md)   | Module organization and file layout     | To fill |
| [Component Guidelines](./component-guidelines.md) | Component patterns, props, composition  | To fill |
| [Hook Guidelines](./hook-guidelines.md)           | Custom hooks, data fetching patterns    | To fill |
| [State Management](./state-management.md)         | Local state, global state, server state | To fill |
| [Quality Guidelines](./quality-guidelines.md)     | Code standards, forbidden patterns      | To fill |
| [Tauri Desktop Testing](./tauri-desktop-testing.md) | Real-window IPC / clone verification    | Active  |
| [Claude CLI Path](./claude-cli.md)                | `which` resolve, `claude_path`, probe   | Active  |
| [Pipeline Init Workspace](./pipeline-workspace.md) | Init workspaces, skill symlinks, repo sync | Active  |
| [Visual Theme](./visual-theme.md) | Tailwind + shadcn, restaurant-warm tokens | Active |
| [Type Safety](./type-safety.md)                   | Type patterns, validation               | To fill |

---

## When to Use

Load [Tauri Desktop Testing](./tauri-desktop-testing.md) before claiming clone, login, demand list, or pipeline submit works. Cursor browser on port 1420 cannot `invoke` / `listen`. 看板未开始列 default is related-to-me (omit JACP `receiver`); 「由我受理」 is `acceptedByMe` → `receiver` = ERP and only changes that Xingyun query. There is no separate 需求 tab. Start-pipeline submit must send `backendTrdUrl`; backend stays out of `pipeline.repos`. Init creates `~/.poria/workspaces/<pipeline_id>/` (doc/skill symlinks + both worktrees) before ReviewPrd; project docs stay in `~/.poria/projects/<demand_code>/` ([Pipeline Init Workspace](./pipeline-workspace.md)). Mid-pipeline 401 / `请先登录` must `blocked` (not `failed`/`cancelled`) and resume the same stage after SSO; silent refresh only re-reads `auth.json`. Agent spawn / Settings Claude status: [Claude CLI Path](./claude-cli.md) — `~/.poria/config.json` `claude_path`, login-shell `which`, probe only in the Poria window. Shell chrome, `PageFrame`, and color tokens: [Visual Theme](./visual-theme.md) — restaurant-warm palette, Tailwind utilities, no Ant Design. Design entry blocks on unanswered P0 in `PRD_REVIEW.md` (blank / TODO / 待填写 / 待确认); P1/P2 warn only. Desktop 文档 drawer and HumanLoopCard can edit/save `PRD_REVIEW.md` via `write_demand_project_file`, then resume. JME notify is best-effort (JoyClaw send/read; gateway-down does not fail the pipeline). Blocked stages notify desktop + 京ME; `human_loop_respond` or a parsed 京ME reply resume/skip/cancel once. `ISSUE_POLICIES.escalate_at` triggers a second JME ping, then degrades to manual desktop handling. Do not leave ReviewPrd incomplete — ReviewPrd still completes, Design is Blocked. Dev waits for frontend `TRD.md` confirmation (`confirm_trd`) or explicit skip-confirm; Design still completes. Dev exit runs OutputGuard against Design `trd_scope` / TRD `## 允许修改范围`; Block (out-of-scope files or blocked deps) keeps the pipeline `blocked` and must not start CR. After OutputGuard, Dev runs worktree local verify (`pnpm typecheck` / test scripts, or Settings `dev_verify_commands`); failure is `LocalVerifyError` → `ISSUE_POLICIES` retry then Blocked HITL — never agent self-score, never CR. CR `securityPass` and Deploy `ciBuildPass` / `testCoverage` come from audit JSON / Coding pipelines / coverage reports — missing data is Block, never agent self-score. CR is an independent reviewer (`--system-prompt`, new `claude -p` session, never `--resume`); P0/P1 from `CR.md` are posted as Coding MR notes after Deploy creates the MR (best-effort except 401). Stage errors go through `ISSUE_POLICIES` (retry / regress CR→Dev once / HITL class label); do not dump every failure as a generic 「失败」. Deploy gate pass → `waiting_merge` (not `completed`); desktop polls MR every 60s; HumanLoopCard 「确认可合并」 posts an MR note and never clicks Merge. Merged → `completed`; closed → `failed`. WaitingMerge / Completed / Failed / Cancelled write pipeline status + MR URL back to Xingyun (JACP communicate + remark). Quality-gate failures create a defect linked to the demand. Cancel/fail after MR: close unmerged MR or open a revert MR (never auto-merge). Desktop startup acquires `worker.lock` next to `poria.db`, runs `PipelineRecovery.recover_all` (interrupted Running stage → Failed + retry, Dev/CR worktree clean; Blocked re-notifies 京ME), then `PipelineWorker` runs up to Settings `max_parallel_pipelines` (default 2, clamp 1..=8) from the SQLite `queue` table; Blocked / failed / waiting_merge free the slot. Each pipeline keeps `workspaces/<pipeline_id>/`. Silent auto-run / HITL resume / `submit_pipeline` enqueue the same queue — do not spawn a second in-memory executor. 看板 and 设置 show a local SQLite summary (stage success, p50/p95 duration, cost USD, HITL rate); a single pipeline still shows its event stream and cost. Do not add a central metrics service.

---

## How to Fill These Guidelines

For each guideline file:

1. Document your project's **actual conventions** (not ideals)
2. Include **code examples** from your codebase
3. List **forbidden patterns** and why
4. Add **common mistakes** your team has made

The goal is to help AI assistants and new team members understand how YOUR project works.

---

**Language**: All documentation should be written in **English**.
