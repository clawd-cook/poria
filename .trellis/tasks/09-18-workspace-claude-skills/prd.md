# 项目工作区软链与标准 Claude skill

## Goal

开工先建本地项目并拉取 JoySpace 资料；再单独建 workspace。项目资料软链进工作区，前后端 worktree 建在工作区里，阶段 skill 改成随应用分发的 Claude 标准 `SKILL.md` 并软链进工作区 `.claude/skills/`。AutoRun 仍按阶段在工作区根 `claude -p` 短指令点名 skill。文档真源不进 git。侧栏可展示工作区路径并在 Finder 打开。

## Background

`09-18-init-workspace-before-docs` 已把 Init 做成：JoySpace 导出到 `~/.poria/projects/<demand_code>/`，再在 `~/.poria/worktrees/<pipeline_id>/<repo>` 建前端 feature worktree 与后端 detached worktree。ReviewPrd / Design 的 Agent cwd 是前端 worktree；阶段实现是 `crates/poria-skills` 的 Prompt 模板 + `claude -p`，不是 Claude 项目 skill。没有工作区根，Claude 看不到统一资料树，也加载不到 `.claude/skills`。

## Decisions

- **D1** 资料真源仍是 `~/.poria/projects/<demand_code>/`（`PRD.md`、`BACKEND_TRD.md`、`PRD_REVIEW.md`、`TRD.md`、`TASK.md`、`CR.md`），不提交进前后端仓。
- **D2** 工作区根为 `~/.poria/workspaces/<pipeline_id>/`，作为 Claude 的 cwd。不再把 worktree 平铺在 `~/.poria/worktrees/<pipeline_id>/`。
- **D3** 工作区内：项目资料软链到 projects；前端 feature worktree 与后端只读 detached worktree 建在该目录下（仓名子目录）。
- **D4** ReviewPrd / Design / Dev / Cr 的执行面是标准 Claude skill。Init / Deploy 仍走 Rust。
- **D5** JoySpace 导出、建目录、软链、`git worktree`、仓同步仍走 Rust。Deploy 的 commit / push / EasyCI SELECT / 建 MR 仍走 Rust。
- **D6** 后端不进 `pipeline.repos`，只读，不交付后端 MR。
- **D7** 桌面 Agent 阶段保持非交互 `claude -p`。skill 里不加 HITL 提示。
- **D8** AutoRun 仍按 Init → ReviewPrd → Design → Dev → Cr → Deploy。每阶段单独 spawn；失败/阻塞停在当前阶段。
- **D9** skill 真源在本仓库随包分发。工作区只软链。第一期无 `~/.poria/skills` overlay。
- **D10** `claude -p` 只发短指令并点名 skill。流程只写在 `SKILL.md`。目录名对齐现有 id：`skill:review-prd` → `review-prd`（`gen-trd` / `gen-code` / `code-review` 同理）。Init 写固定的工作区 `CLAUDE.md`，各阶段不改。
- **D11** 侧栏资源分组加「工作区」：展示当前流水线工作区路径，并用已有 opener 在 Finder 打开。不做工作区内文件浏览器。

## Requirements

- **R1** Init 在同一阶段内：导出 JoySpace 到 `~/.poria/projects/<demand_code>/`；同步登记仓；创建 `~/.poria/workspaces/<pipeline_id>/`；把项目文档软链进工作区；在工作区内创建前端 feature worktree 与后端 detached worktree；把随包 skill 软链到 `.claude/skills/`；写入 `CLAUDE.md`。任一步失败则 Init 失败。
- **R2** AutoRun 仍 `execute_stage`。ReviewPrd / Design / Dev / Cr 的 cwd 为工作区根；`-p` 为短指令 + 点名 skill。Claude 经软链读资料，在工作区内进出前后端 worktree。
- **R3** 本仓库每个 Claude 阶段 skill 一个目录（含 `SKILL.md`），随应用分发。Init 软链进工作区，不复制、不读用户 overlay。目录名去掉 `skill:` 前缀。
- **R4** 前端仓不出现文档真源；Deploy 只提交前端 worktree 代码，并去掉误写进仓的文档。
- **R5** 侧栏资源下「工作区」展示选中流水线的 `workspacePath`，可在 Finder 打开该目录。无选中流水线时说明先从看板打开。
- **R6** 工作区根 `CLAUDE.md` 说明软链、前后端目录、文档不要提交进 git。

## Acceptance Criteria

- [ ] AC1: Init 完成后存在 `~/.poria/projects/<demand_code>/PRD.md` 与 `BACKEND_TRD.md`，以及 `~/.poria/workspaces/<pipeline_id>/`。
- [ ] AC2: 工作区内文档为指向 projects 的软链；前端/后端 worktree 位于该工作区下，而不是 `~/.poria/worktrees/<pipeline_id>/`。
- [ ] AC3: 工作区 `.claude/skills/` 下 `review-prd` / `gen-trd` / `gen-code` / `code-review` 为指向随包 skill 的软链。
- [ ] AC4: ReviewPrd / Design 的 Claude cwd 为工作区根；`-p` 为短指令且点名 skill；`PRD_REVIEW.md` / `TRD.md` 落在 projects（经软链可见）；工作区根有 `CLAUDE.md`。
- [ ] AC5: `pipeline.repos` 仍只有前端；Deploy 不给后端建 MR、不把 projects 文档提交进仓。
- [ ] AC6: 这些 Claude 阶段仍是按阶段 `claude -p`；失败不改成一场长会话把后面阶段跑完。
- [ ] AC7: 侧栏「工作区」显示路径；有 `workspacePath` 时可 Finder 打开。
- [ ] AC8: `cargo test` 覆盖工作区路径、软链、skill 链接、短指令；`pnpm typecheck` 通过。Init 在 Poria 窗口验证，不用 Vite `:1420`。

## Out of scope

- 后端仓进入 Dev / CR / Deploy。
- 文档真源改到 git 仓内。
- 旧流水线 / `~/.poria/worktrees` 双路径兼容。
- 用户 overlay、运行时改 `SKILL.md` 不发版。
- 旧 Prompt 全文与 `SKILL.md` 双轨。
- 工作区内文件浏览器、MCP、多 Agent、Todos 云沙箱。
- skill-creator 评测 viewer / description optimizer。
- `PipelineWorker`、事件流历史、HITL 持久化。
