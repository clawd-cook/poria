# Four-tab shell: home kanban, settings, header auth

Parent: `09-17-delivery-workspace` (R1, R2, R6). Technical shape: parent `design.md`.

## Goal

把主导航换成首页 / 需求列表 / 仓库列表 / 设置。首页用看板展示已有流水线并打开现有详情；设置成为独立页；登录态固定在顶栏。

## Requirements

- 顶栏四 Tab，顺序：首页、需求列表、仓库列表、设置。去掉 Pipeline / 技能 / 渠道
- 顶栏右侧全局 `AuthStatus`（登录/登出）
- 首页按状态分列展示 `list_pipelines`；点击卡片打开现有 `PipelineDetail`，可返回看板
- 设置页承接原 Modal 表单（门禁、超时、重试、数据目录），不再用 `settingsOpen` Modal
- 需求列表、仓库列表本任务可放空状态页（文案占位），不实现登记/拉需求

Must wait: none. Demand/repo pages are filled by sibling tasks.

## Out of Scope

- 仓库克隆、行云需求列表、开始向导、改 `submit_pipeline`

## Acceptance Criteria

- [ ] 仅四个顶栏 Tab，可切换且不丢首页选中的 pipeline
- [ ] 看板列至少含运行中 / 阻塞 / 待合并 / 完成 / 失败；点击打开详情并可返回
- [ ] 设置页可读写原配置；AuthStatus 在四 Tab 都可见
- [ ] 技能/渠道页不再出现在导航
- [ ] `pnpm typecheck` 通过
