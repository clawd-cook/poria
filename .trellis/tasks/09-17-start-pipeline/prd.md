# Start pipeline wizard: FE repo, BE repo+branch, JoySpace PRD

Parent: `09-17-delivery-workspace` (R4). Technical shape: parent `design.md`.

## Goal

从需求「开始」走完前端仓 → 后端仓+分支 → JoySpace PRD，创建流水线：只把前端写入 `pipeline.repos`，后端进 `backend_context`，然后出现在首页看板。

## Requirements

- 三步必填；前端仓与后端仓必须是不同的 ready 登记仓
- 前端不选分支，用托管副本当前检出作为 `base_branch`
- 后端分支来自本地 git（打开该步时 fetch）；只读，不创建后端 MR
- PRD 必填 JoySpace URL；`resolvePrdLink` 能解析则预填可改
- `submit_pipeline` 改为结构化入参；提交后切首页并选中新任务
- 去掉粘贴行云链接主入口

Must wait: `repo-registry` 有 ready 仓；`demand-list` 有「开始」入口。

## Out of Scope

- Init/Workspace skill 完整实现（可创建 pipeline 记录并沿用现有 execute_stage 入口）
- 后端交付流水线

## Acceptance Criteria

- [ ] 缺前端、后端、分支或 PRD 不能提交
- [ ] 创建的 pipeline `repos` 仅前端；config 含 `prd_url` 与 `backend_context`
- [ ] 首页出现新卡片
- [ ] `cargo check --workspace` 与 `pnpm typecheck` 通过
