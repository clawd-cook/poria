# Register and clone repos under ~/.poria/repos/scope/name

Parent: `09-17-delivery-workspace` (R5). Technical shape: parent `design.md`.

## Goal

在仓库 Tab 登记 git URL，克隆到 `~/.poria/repos/<scope>/<repo>`，按 scope 分组展示状态。

## Requirements

- 只填 git URL，不选已有本地目录
- 路径用 `repo_search_path_from_git_url`：`git@coding.jd.com:ls/ls-entrance.git` → `~/.poria/repos/ls/ls-entrance`
- SQLite `registered_repos`（schema v2）；状态 cloning / ready / failed；失败可重试
- 同一规范化 URL 或同一 scope/name 拒绝重复
- 登记时不标注前端/后端角色
- 列表按 scope 分组：名、URL、本地路径、状态

Must wait: `desktop-ia` 提供仓库 Tab。不依赖 SSO（本机 SSH 即可 clone）。

## Out of Scope

- 向导选仓、worktree、后端分支列表（`start-pipeline`）

## Acceptance Criteria

- [ ] 能登记示例 SSH 地址并出现在 `~/.poria/repos/ls/ls-entrance`（或测试用等价 URL 的 scope/name 路径）
- [ ] 失败可重试；重复 URL 被拒绝
- [ ] `cargo check --workspace` 与 `pnpm typecheck` 通过
