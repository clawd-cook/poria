# 看板统一需求与任务

## Goal

侧栏只留一张看板。行云里的每一条需求就是一条任务：还没在 Poria 开工的排在「未开始」；开工之后只展示并推进 Poria 自己的任务状态，不再用行云 `status_label`。同一 `demand_code` 在看板上最多一张卡。

## Background

当前「需求」页调 `list_demands`（状态来自行云），「看板」只渲染 SQLite `pipelines`。`submit_pipeline` 每次 `create_pipeline_id()`，`pipelines.demand_code` 无唯一约束，同一需求可以多条流水线。需求列表不进全局 store。

用户已确认：不拆需求和任务；每个需求即未开始任务；任务状态单独维护；1:1；历史多流水线只展示并复用 `updated_at` 最新的一条。

## Decisions

- **D1** 工作面只有看板。侧栏去掉「需求」。搜索、由我受理、从链接开始都在看板上。
- **D2** 任务键：`demand_code` 非空用它，否则 `demand_id`。看板同一键最多一张卡。
- **D3** 「未开始」= 当前 Xingyun 查询结果里还没有本地任务的需求，以及本地 `PipelineStatus::Created`。其它列只用本地状态。不渲染行云 `status_label`。
- **D4** `submit_pipeline`：该键已有任务则返回已有 id（必要时只在 `created` 时更新向导里的仓/URL），不插入新行。运行中/终态不覆盖 config。
- **D5** 未登录不调 `list_demands`；本地任务仍可看；未开始列给登录入口。
- **D6** 需求过滤不变：默认 related-to-me（省略 JACP `receiver`）；「由我受理」才 `receiver` = ERP。不要给无 receiver 的行盖登录 ERP。
- **D7** 开工仍必须 `backendTrdUrl`；后端不进 `pipeline.repos`。
- **D8** 同一键多行历史：看板和复用都只认 `updated_at` 最新的一行；更旧行留在 SQLite，不出卡。本期不加 UNIQUE 索引、不删旧行。
- **D9** 未开始卡片（无 pipeline）点开开工向导。已有 pipeline 的卡片点开详情。终态（完成/失败/取消）不自动重跑。
- **D10** 看板列：未开始、运行中、阻塞、待合并、完成、失败、已取消。去掉「其他」。

## Requirements

- **R1** 侧栏一级：看板 / 资源 / 设置。`demands` 视图不再挂菜单；若当前 view 是 `demands` 则落到看板。
- **R2** 看板工具条：关键词、由我受理、从行云链接开始。搜索驱动未开始列的 Xingyun 查询，同时过滤本地列的名称/编号。
- **R3** 未开始列：当前页 Xingyun records 减去已有本地任务键，加上本地 `created` 任务卡。
- **R4** 其它列：`list_pipelines` 对前端已按 D2/D8 去重后的本地任务，按 D10 分组。
- **R5** `submit_pipeline` 按 D4 复用；向导成功后仍 `hydrate`、选中、留在看板。
- **R6** 文档抽屉、开工向导、流水线详情仍可用，只是入口都在看板。

## Acceptance Criteria

- [ ] AC1: 侧栏没有「需求」；搜索 / 由我受理 / 链接开始都在看板。
- [ ] AC2: 登录后，related-to-me 中尚未本地开工的需求出现在「未开始」；已开工需求只出现在对应 Poria 状态列，每键一张卡。
- [ ] AC3: 「由我受理」只改变未开始列的 Xingyun 查询；不改本地任务状态。
- [ ] AC4: 对已有任务的需求再 `submit_pipeline`，不新增 `pipelines` 行，返回已有 id。
- [ ] AC5: 看板卡片状态只来自本地 `PipelineStatus`（未开始列的 Xingyun 卡无状态徽标或只显示「未开始」），不渲染 `status_label`。
- [ ] AC6: 未登录不调用 `list_demands`；未开始列可登录。
- [ ] AC7: SQLite 中同一 `demand_code` 两行时，看板只出 `updated_at` 最新那张。
- [ ] AC8: `pnpm typecheck` 通过；复用/去重有 `cargo test`；Poria 窗口验证，不用 Vite `:1420`。

## Out of scope

- 回写 Xingyun 需求状态。
- 把 crate/表改名为 task。
- 未开始列无限滚动或把 Xingyun 全量写入 SQLite。
- 给 `demand_code` 加 UNIQUE 或删除历史重复行。
- 完成/失败/取消后一键重开新流水线。
- 多 Agent、HITL 持久化、工作区内文件浏览器。
