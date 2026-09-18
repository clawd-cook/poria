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
| [Visual Theme](./visual-theme.md) | Ant Design v6 + Swiss/Minimal tokens | Active |
| [Type Safety](./type-safety.md)                   | Type patterns, validation               | To fill |

---

## When to Use

Load [Tauri Desktop Testing](./tauri-desktop-testing.md) before claiming clone, login, demand list, or pipeline submit works. Cursor browser on port 1420 cannot `invoke` / `listen`. 看板未开始列 default is related-to-me (omit JACP `receiver`); 「由我受理」 is `acceptedByMe` → `receiver` = ERP and only changes that Xingyun query. There is no separate 需求 tab. Start-pipeline submit must send `backendTrdUrl`; backend stays out of `pipeline.repos`. Init creates `~/.poria/workspaces/<pipeline_id>/` (doc/skill symlinks + both worktrees) before ReviewPrd; project docs stay in `~/.poria/projects/<demand_code>/` ([Pipeline Init Workspace](./pipeline-workspace.md)). Mid-pipeline 401 / `请先登录` must `blocked` (not `failed`/`cancelled`) and resume the same stage after SSO; silent refresh only re-reads `auth.json`. Agent spawn / Settings Claude status: [Claude CLI Path](./claude-cli.md) — `~/.poria/config.json` `claude_path`, login-shell `which`, probe only in the Poria window. Shell chrome, `PageFrame`, and `ConfigProvider` tokens: [Visual Theme](./visual-theme.md) — single primary, no extra brand palette. Design entry blocks on unanswered P0 in `PRD_REVIEW.md` (blank / TODO / 待填写 / 待确认); P1/P2 warn only. Desktop 文档 drawer and HumanLoopCard can edit/save `PRD_REVIEW.md` via `write_demand_project_file`, then resume. JME notify is best-effort (placeholder send). Do not leave ReviewPrd incomplete — ReviewPrd still completes, Design is Blocked. Dev waits for frontend `TRD.md` confirmation (`confirm_trd`) or explicit skip-confirm; Design still completes. Dev exit runs OutputGuard against Design `trd_scope` / TRD `## 允许修改范围`; Block (out-of-scope files or blocked deps) keeps the pipeline `blocked` and must not start CR. CR `securityPass` and Deploy `ciBuildPass` / `testCoverage` come from audit JSON / Coding pipelines / coverage reports — missing data is Block, never agent self-score. Stage errors go through `ISSUE_POLICIES` (retry / regress CR→Dev once / HITL class label); do not dump every failure as a generic 「失败」.

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
