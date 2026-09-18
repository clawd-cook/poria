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

Load [Tauri Desktop Testing](./tauri-desktop-testing.md) before claiming clone, login, demand list, or pipeline submit works. Cursor browser on port 1420 cannot `invoke` / `listen`. 看板未开始列 default is related-to-me (omit JACP `receiver`); 「由我受理」 is `acceptedByMe` → `receiver` = ERP and only changes that Xingyun query. There is no separate 需求 tab. Start-pipeline submit must send `backendTrdUrl`; backend stays out of `pipeline.repos`. Init creates `~/.poria/workspaces/<pipeline_id>/` (doc/skill symlinks + both worktrees) before ReviewPrd; project docs stay in `~/.poria/projects/<demand_code>/` ([Pipeline Init Workspace](./pipeline-workspace.md)). Agent spawn / Settings Claude status: [Claude CLI Path](./claude-cli.md) — `~/.poria/config.json` `claude_path`, login-shell `which`, probe only in the Poria window. Shell chrome, `PageFrame`, and `ConfigProvider` tokens: [Visual Theme](./visual-theme.md) — single primary, no extra brand palette.

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
