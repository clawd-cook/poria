# 项目工作区软链与标准 Claude skill — implement

## Checklist

1. **随包 skill**：新建 `skills/review-prd`、`gen-trd`、`gen-code`、`code-review`（各 `SKILL.md`）。从 `crates/poria-skills/src/prompts/` 迁移正文，改成工作区根 + 软链写法，去掉 HITL。Tauri 把 `skills/` 打进 resources；运行时解析 bundled 路径。
2. **工作区根**：`run_init_stage` / `WorktreeResource` 根改为 `~/.poria/workspaces`。路径守卫从 `worktrees` 改为 `workspaces`。Init：建目录、文档软链、skill 软链、`CLAUDE.md`、再双 worktree。output 增加 `workspacePath`。回滚删整个工作区目录。
3. **Claude 阶段薄封装**：ReviewPrd / Design / Dev / Cr 的 cwd = `workspacePath`；`-p` 短指令点名 skill；去掉旧 `render_prompt` 全文 / 巨型 `--system-prompt`。dispatch 后仍验收 projects 产物。`stage_skill_id` 字符串不变。
4. **Deploy / executor**：Deploy 仍用前端 `worktreePath`。Agent 解析 cwd 优先 `workspacePath`。更新 fixture 与 rollback 测试路径。
5. **侧栏**：`ViewType` + Shell 资源组「工作区」；页展示路径 + opener 打开 Finder。无选中流水线时引导去看板。
6. **规格**：更新 `.trellis/spec/frontend/pipeline-workspace.md` 与 `AGENTS.md` 工作区路径表。
7. **验证**：见下。不要用 Vite `:1420` 冒充 Init/Claude。

## Validation

```bash
export NVM_DIR="$HOME/.nvm"
. "/opt/homebrew/opt/nvm/nvm.sh"
nvm use 24.20.0
node -v   # v24.20.0
pnpm -v   # 11.23.0
pnpm typecheck
cargo test -p poria-resources -- worktree
cargo test -p poria-skills
cargo test -p poria-commands -- init_rollback
cargo test -p poria-desktop --lib -- --test-threads=1
```

桌面：`cargo tauri dev`，**Poria** 窗口。新流水线 Init 后确认 `~/.poria/workspaces/<id>/` 含软链、`.claude/skills`、双 worktree、`CLAUDE.md`。侧栏「工作区」能打开该目录。ReviewPrd 的 cwd 为工作区根且 `PRD_REVIEW.md` 落在 projects。

## Risky files / rollback

- `src-tauri/src/commands/pipeline.rs` — Init 路径、cwd、守卫
- `crates/poria-resources/src/worktree/mod.rs` — 默认根
- `crates/poria-skills/src/review_prd.rs` 等 — 短指令
- `src-tauri/tauri.conf.json` — bundle resources
- `src/components/Shell.tsx`、`src/lib/types.ts`

回滚：恢复 `~/.poria/worktrees` 与 Prompt 模板路径。不迁移已建的新工作区。

## Before `task.py start`

- [x] `prd.md` 无未决 Open questions
- [x] `design.md` / `implement.md` 已写
- [ ] 用户确认规划后才 `task.py start`
