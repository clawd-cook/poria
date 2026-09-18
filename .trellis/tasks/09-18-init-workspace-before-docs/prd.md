# Init 时建立前后端工作区后再评审和写 TRD

## Goal

开始流水线时仓库已经选好。Init 一次做完 JoySpace 文档导出和工作区准备（前端 feature worktree、后端只读 worktree），然后才 ReviewPrd 和前端 TRD。评审/设计对照真实代码写文档。项目文档统一放在 `~/.poria/projects/<demand_code>/`，**不进 git 仓库**。不保留、不迁移旧阶段顺序。

登记仓记住主分支；托管副本始终同步该主分支最新提交。可手动同步；拉 worktree 前也会同步。仓库列表展示同步状态和最近同步时间。

## Background

旧顺序 Init → ReviewPrd → Design → Workspace → Dev → Cr → Deploy 已否决，不做兼容。

`submit_pipeline` 已填前端仓、后端仓、后端分支、PRD、后端 TRD。前端进 `pipeline.repos`；后端进 `backend_context`，不进 `pipeline.repos`。

`registered_repos`（schema v2）只有克隆状态。开始向导前端 `base_branch` 取托管副本当前检出。Workspace 只 `git_fetch`，且排在 Design 之后。Claude `cwd` 目前是文档目录，评审等于盲写。

现有文档目录已经是 `~/.poria/projects/<demand_code>/`：`PRD.md`、`BACKEND_TRD.md`、`PRD_REVIEW.md`（用户所称 PRE_PREVIEW）、`TRD.md`。Deploy 提交的是前端 worktree 代码，不应夹带这些文档。

## Decisions

- **D1** 推翻历史阶段顺序。不为进行中的旧流水线做迁移或双路径。
- **D2** 工作区准备并进 Init。新顺序：Init → ReviewPrd → Design → Dev → Cr → Deploy。删除独立 Workspace 用户阶段。
- **D3** 后端用独立只读 worktree：`~/.poria/worktrees/<pipeline_id>/<后端仓名>`，检出向导所选分支。托管 clone 只当 fetch 源。后端不建 feature 分支、不进 `pipeline.repos`。
- **D4** 登记仓主分支默认 `master`，仓库页可改。登记表单仍只贴 git URL；已有仓升级后主分支也是 `master`，直到用户改掉。
- **D5** 保存主分支后立刻同步托管副本。失败则主分支字段已更新，同步状态为失败并展示原因，不回滚主分支。
- **D6** ReviewPrd / Design 的 Agent `cwd` 为前端 worktree，以便读代码。`PRD.md`、`BACKEND_TRD.md`、`PRD_REVIEW.md`、`TRD.md` 仍只写在 `~/.poria/projects/<demand_code>/`。前端仓不放项目文档，便于统一管理。

## Requirements

- **R1** Init 在同一阶段内：把 JoySpace PRD / 后端 TRD 导出到 `~/.poria/projects/<demand_code>/`；同步前端托管副本后，按登记主分支拉出前端 feature 工作区；同步后端托管副本后，按向导所选分支拉出后端只读 worktree。任一步失败则 Init 失败，不得进入 ReviewPrd。
- **R2** 后端仍不进入 `pipeline.repos`。Dev / CR / Deploy 只交付前端。后端工作区只读，禁止改后端仓、禁止后端 MR。
- **R3** ReviewPrd / Design 的 Agent `cwd` 为前端 worktree，并必须能读后端只读 worktree。文档权威路径是 `~/.poria/projects/<demand_code>/` 下的 `PRD.md`、`BACKEND_TRD.md`、`PRD_REVIEW.md`、`TRD.md`。
- **R4** 开始向导已填的仓库/分支是工作区来源，不另开选仓步骤。前端 feature 的 base 改为登记仓的主分支，不再用「当前检出」。
- **R5** 阶段枚举、UI、自动连跑、回滚按新顺序改；旧 `StageEnum::Workspace` 从主路径删除。
- **R6** 代码库管理记录每个登记仓的主分支，默认 `master`，可在仓库页修改。保存主分支后立刻同步（与手动同步同一套逻辑）。
- **R7** 本地托管副本（`~/.poria/repos/<scope>/<name>`）始终同步该主分支最新提交：仓库页可手动同步；Init 拉 worktree 前必须先同步对应仓。同步 = fetch + 把托管副本快进到主分支（不在托管副本上开发）。
- **R8** 代码库管理展示同步状态与最近同步时间；失败时展示原因。
- **R9** Deploy 只提交前端 worktree 里的代码变更，不把 `~/.poria/projects` 文档拷进仓库、不把 `PRD_REVIEW.md` / `TRD.md` 当作仓内产物。

## Acceptance Criteria

- [ ] AC1: 新流水线完成 Init 后，前端工作区存在且检出 feature 分支，base 为该仓登记的主分支。
- [ ] AC2: 同时后端只读 worktree 存在且检出向导所选后端分支；托管副本仍在主分支上。
- [ ] AC3: ReviewPrd 完成后 `~/.poria/projects/<demand_code>/PRD_REVIEW.md` 存在；Design 完成后同目录 `TRD.md` 存在。Agent `cwd` 为前端 worktree。前端 worktree 根目录不出现这两份文档。
- [ ] AC4: `pipeline.repos` 仍只有前端；Deploy 不给后端建 MR。
- [ ] AC5: `STAGE_ORDER` 无 Workspace；阶段条与自动连跑为 Init → ReviewPrd → Design → Dev → Cr → Deploy。
- [ ] AC6: 仓库列表可见主分支（默认 `master`）、同步状态、最近同步时间；可改主分支、可手动同步。改主分支后立即同步；失败时主分支已保存、同步状态为失败并有原因。
- [ ] AC7: Init 拉 worktree 前同步失败则 Init 失败，并写回该仓同步状态。
- [ ] AC8: Deploy 提交内容不含 `~/.poria/projects` 下的文档。
- [ ] AC9: `cargo test` 覆盖新顺序、Init 工作区、登记仓同步字段、文档路径；`pnpm typecheck` 通过。

## Out of scope

- 后端仓进入 Dev/CR/Deploy 多仓交付。
- 改开始向导「选哪个仓」的步骤（仍是前端仓 + 后端仓 + 后端分支）。
- 旧流水线迁移、双 `STAGE_ORDER`。
- 在托管副本上做功能开发（功能只在 worktree）。
- 把 PRD / 后端 TRD / PRD_REVIEW / TRD 提交进前端仓。
- 重命名 `PRD_REVIEW.md` 为 PRE_PREVIEW（界面/文档可用「评审」表述，文件名保持现有常量）。
- `PipelineWorker`、事件流历史、HITL 持久化、Claude PATH。
