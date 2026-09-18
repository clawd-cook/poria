# Init workspace before docs — implement

## Checklist

1. **Core 阶段顺序**：`STAGE_ORDER` / `StageEnum` 去掉 `Workspace`。更新所有 match、测试、`stage_skill_map`、`traits::stage_skill_id`。前端 `src/lib/types.ts` 同步。
2. **登记仓 schema v3**：`default_branch`、`sync_status`、`last_synced_at`、`sync_error`。已有行 default `master`。单测 insert/list/update。
3. **同步原语**（`poria-resources` git + `repos` command）：fetch + ff-only 到 `origin/<default_branch>`。IPC 手动同步；改主分支后立刻同步。克隆成功后同步。仓库页 UI。
4. **Worktree**：前端 `-b feature_*` from 主分支；后端 `--detach` 所选分支。Init 拉树前对两仓调用同步。
5. **InitSkill + `run_init_stage`**：导出文档后建双 worktree；output 含两路径；`backend_context.local_path` 改为后端 worktree。删除 `run_workspace_stage` 主路径。回滚删除两个 worktree。
6. **ReviewPrd / Design**：`cwd` = 前端 worktree；产物仍写 `~/.poria/projects/<demand_code>/`；`backend_aid` 用后端 worktree。提示词改绝对文档路径。禁止把文档写进仓根。
7. **submit_pipeline**：`base_branch` = 登记 `default_branch`。向导文案。
8. **验证**：见下。桌面在 Poria 窗口测仓库同步 + 新流水线 Init 后再评审。

## Validation

```bash
export NVM_DIR="$HOME/.nvm"
. "/opt/homebrew/opt/nvm/nvm.sh"
nvm use 24.20.0
node -v   # v24.20.0
pnpm -v   # 11.23.0
pnpm typecheck
cargo test -p poria-core -- STAGE_ORDER
cargo test -p poria-infrastructure -- registered_repo
cargo test -p poria-resources -- worktree
cargo test -p poria-skills -- init
```

桌面：`cargo tauri dev`，**Poria** 窗口（不要用浏览器 :1420）。仓库页改主分支/手动同步；开始一条流水线，确认 Init 建好双 worktree 后再出 `PRD_REVIEW.md`，且 worktree 根目录无该文件。

## Risky files / rollback

- `crates/poria-core/src/types/pipeline_types.rs` — 阶段枚举
- `src-tauri/src/commands/pipeline.rs` — Init/Workspace 执行与 worktree 解析
- `crates/poria-infrastructure/src/store/schema.rs` — 迁移
- `crates/poria-resources/src/worktree/mod.rs` — detach 后端
- `src/components/RepoListPage.tsx`、`StartPipelineWizard.tsx`

回滚：恢复 `Workspace` 阶段与旧 schema 列（或停用新列）。不迁移旧 pipeline。

## Before `task.py start`

- [x] `prd.md` 无未决 Open questions
- [x] `design.md` / `implement.md` 已写
- [ ] 用户确认规划后才 `task.py start`
