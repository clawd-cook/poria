# Init workspace before docs — design

## Boundaries

- **In**: `STAGE_ORDER` / `StageEnum`；`InitSkill`；`WorktreeResource`；`registered_repos` schema 与仓库页；`submit_pipeline` 的 frontend `base_branch`；ReviewPrd / Design 的 Agent `cwd` 与产物路径；Init 回滚；自动连跑。
- **Out**: 后端进 `pipeline.repos`；文档进 git；旧流水线迁移；向导选仓步骤；Claude PATH。

`WorkspaceSkill` 的 git worktree 逻辑并进 Init（或由 Init 调用同一资源）。删除用户可见的 Workspace 阶段；`src-tauri` 不再走 `run_workspace_stage`。

## Stage order

```
Init → ReviewPrd → Design → Dev → Cr → Deploy
```

Rust `STAGE_ORDER`、`stage_skill_map`、`traits::stage_skill_id`、前端 `src/lib/types.ts` 的 `STAGE_ORDER` / `STAGE_LABELS` 必须同一份。SQLite 已有 `workspace` 行的旧 pipeline 不保证可跑。

## Init data flow

1. 导出 JoySpace → `~/.poria/projects/<demand_code>/PRD.md`、`BACKEND_TRD.md`（现有 Init）。
2. 对前端登记仓 `sync_registered_repo`（见下），再 `WorktreeResource::create`：新分支 `feature_<demand_code>`，base = 登记主分支。
3. 对后端登记仓同样先 sync，再只读 worktree：路径 `~/.poria/worktrees/<pipeline_id>/<backend_name>`，检出向导所选分支。
4. 把前端 `worktreePath`、后端 worktree 路径写入 Init `output` 与 `config.backend_context`（`local_path` 改为后端 **worktree**，托管 clone 路径仍可留在登记仓上）。ReviewPrd/Design/`backend_aid` 读 worktree，不读托管 clone。
5. 任一步失败 → Init Failed，自动连跑停下。

后端 worktree 必须 `git worktree add --detach <path> origin/<所选分支>`（或等价：不占用与托管副本相同的分支锁）。托管副本始终 checkout 主分支；若后端所选也是 `master`，非 detach 的 `worktree add` 会失败。

前端 feature 仍用 `-b feature_… origin/<主分支>`，与托管副本占着主分支不冲突。

## Registered repo sync

Schema v3（`registered_repos` 增列，缺省兼容已有行）：

| Column | Default / meaning |
|---|---|
| `default_branch` | `'master'` |
| `sync_status` | `idle` / `syncing` / `synced` / `failed` |
| `last_synced_at` | nullable RFC3339 |
| `sync_error` | nullable；可与 clone `error` 分开，避免克隆失败和同步失败互相覆盖 |

克隆成功后立刻按 `default_branch` 同步一次。

`sync_registered_repo(id)`（仓库页按钮、改主分支、Init 拉 worktree 前共用）：

1. `clone_status` 必须 `ready`。
2. `git fetch origin`。
3. 在**托管副本**上 `checkout` 主分支，`--ff-only` 到 `origin/<default_branch>`。托管副本禁止当开发目录；若 dirty 或无法快进 → `sync_status=failed`，写原因。
4. 成功：`synced` + `last_synced_at=now`。

改 `default_branch`：先 persist 新值，再调用同一 sync。失败不回滚字段。

IPC：`sync_repo`、`update_repo_default_branch`（或合并成一个 update + 自动 sync）。`RegisteredRepo` / 前端 `types.ts` / `RepoListPage` 展示主分支、同步状态、时间；ready 仓有「同步」；主分支可编辑。

`submit_pipeline`：前端 `base_branch` = 该登记仓 `default_branch`，不再 `git_current_branch`。向导文案去掉「当前检出」。

## Agent + artifacts

- ReviewPrd / Design：`worktree_path` = 前端 worktree（Init output）。提示词带文档目录绝对路径和后端 worktree 路径。
- Write `PRD_REVIEW.md` / `TRD.md` 到 `project_dir`（`FeatureContext`），禁止写进 git 根。
- Init 仍把 `PRD.md` / `BACKEND_TRD.md` 写到同一 `project_dir`。
- Deploy：只 `git add` 前端 worktree 代码，不复制 `~/.poria/projects`。

## Rollback

Init 成功后的 rollback 含：删除前端 worktree、删除后端 worktree（现 Workspace 回滚只处理前端，要补后端）。不删 `~/.poria/projects` 文档（与「统一管理项目文档」一致，除非产品以后要求）。

## Compatibility

无。新 `STAGE_ORDER` 创建的 pipeline 才是支持对象。

## Trade-offs

- 删除 Workspace 阶段：阶段条更短，与「初始化时建工作区」一致；旧任务 SQLite 作废。
- 后端 detach worktree：只读安全、不与主分支锁冲突；用户在该目录看不到具名分支属预期。
- 文档不进仓：项目文档集中在 `~/.poria/projects`；MR 里没有 PRD_REVIEW/TRD。
