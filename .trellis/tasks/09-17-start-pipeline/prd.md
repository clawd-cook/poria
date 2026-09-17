# Start pipeline wizard: FE repo, BE repo+branch, JoySpace PRD + backend TRD

Parent: `09-17-delivery-workspace` (R4). Technical shape: parent `design.md`.

## Goal

从需求「开始」走完前端仓 → 后端仓+分支 → JoySpace PRD + 后端 TRD，创建流水线：只把前端写入 `pipeline.repos`，后端仓进 `backend_context`，后端 TRD URL 进 `backend_trd_url`，并在前端设计/编码阶段作为只读辅助。

## Requirements

- 前端仓、后端仓+分支、PRD、后端 TRD 均必填；前端仓与后端仓必须是不同的 ready 登记仓
- 前端不选分支，用托管副本当前检出作为 `base_branch`
- 后端分支来自本地 git（打开该步时 fetch）；只读，不创建后端 MR
- PRD 必填 JoySpace URL；`resolvePrdLink` 能解析则预填可改
- 后端 TRD 必填 JoySpace URL，向导内粘贴；不得与 PRD 相同。不覆盖 feature 里生成的前端 `TRD.md`（产物名 `BACKEND_TRD.md` / config `backend_trd_url`）
- `submit_pipeline` 结构化入参含 `backendTrdUrl`；提交后切首页并选中新任务
- `gen_trd` / `gen_code` 读取 `pipeline.config.backend_trd_url` 与 `backend_context`（路径+分支），注入提示词：只读参考，辅助前端接口/字段/流程，禁止改后端仓
- 去掉粘贴行云链接主入口

Must wait: `repo-registry` 有 ready 仓；`demand-list` 有「开始」入口。

## Out of Scope

- JoySpace 直播导出 Markdown（Init 仍可为 NotImplemented；URL 必须进 config 与提示词）
- 后端交付流水线

## Acceptance Criteria

- [ ] 缺前端、后端、分支、PRD 或后端 TRD 不能提交；PRD 与后端 TRD 同 URL 拒绝
- [ ] 创建的 pipeline `repos` 仅前端；config 含 `prd_url`、`backend_trd_url`、`backend_context`
- [ ] 前端 TRD 生成与编码提示词包含后端 TRD URL 及后端仓只读路径
- [ ] 首页出现新卡片
- [ ] `cargo check --workspace` 与 `pnpm typecheck` 通过
