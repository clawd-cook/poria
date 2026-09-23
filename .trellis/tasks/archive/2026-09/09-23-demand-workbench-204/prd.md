# 2.0.4 需求工作台与交付动线

Product source: [`docs/versions/2.0.4/PRD.md`](../../../docs/versions/2.0.4/PRD.md)

## Goal

把「流水线能跑通」做成「交付好用」：以一条需求/流水线为根对象，列表只负责发现与注意力；点开后进入**需求工作台**（轨迹 / 文档 / 工作区 / 确认），顶栏有阶段与唯一主 CTA；HITL 有全局待办并可到达确认切面。

用户价值：少切侧栏 Tab、少猜下一步；阻塞时能立刻处理。

## Background

- 后端六阶段与 HITL 契约保持不变（`STAGE_ORDER`、IPC 事件语义不改交付含义）。
- 现状：`Shell` 六 Tab；交付挤在 `HomeBoard` 嵌套详情；`humanRequest` 不导航；`WorkspacePage` 寄生选中流水线；`PipelineSidebar` 死代码；渠道/技能在主航道但与交付弱耦合。
- 竞品轻舟：需求列表 → 需求工作台多切面；dsh：Host 投影 + HITL 一等面（只借思想）。
- **交付策略（已决）**：本任务为**单任务**覆盖产品 PRD 的 **P0+P1**；实现按 `implement.md` Slice A→E 顺序推进、可分 PR 回滚。产品 PRD 的 P2（轨迹叙事打磨、可用性微调）不阻塞本任务验收，可同任务末尾顺手或发版后小修。

## Requirements

| ID | Requirement |
|----|-------------|
| R1 | 引入纯函数 **Pipeline ViewModel**：由 pipeline / stages / humanRequest / auth / repos / config 派生 `mode`、`primaryCta`、`badges`；页面只渲染 ViewModel。 |
| R2 | **列表 ↔ 工作台**分离：未选中看列表；选中 `pipelineId`（含启动成功）进入工作台，可返回列表且不丢选中。 |
| R3 | 工作台切面至少：**轨迹**（阶段/流/事件/门禁）、**文档**（项目产物预览）、**工作区**（路径+打开目录）、**确认**（HITL / waiting_merge）。 |
| R4 | 顶栏展示需求名（或流水线标题）、当前阶段、**唯一主 CTA**（文案与 action 来自 ViewModel）。 |
| R5 | HITL：`human:request` 更新**待我处理**队列与角标；**默认**跳入该工作台确认切面（设置可关）；选中流水线不得误清仍有效的 HITL 上下文。 |
| R6 | 列表提供「待我处理」筛选（Blocked + HITL 注意力）。 |
| R7 | 启动就绪门禁：登录 / 仓 / Claude / `backendTrdUrl` 缺失时在向导内引导，补完可回。 |
| R8 | 壳 IA：渠道、技能移出主航道（设置或只读关于页）；独立「工作区」Tab 移除，能力并入工作台切面；清理 `PipelineSidebar` 死代码（禁止双轨）。 |
| R9 | 未登录不调用 `list_demands`；验收必须在 `poria-desktop` 窗口。 |

## Acceptance Criteria

- [ ] AC1：启动成功后自动进入该流水线工作台·轨迹切面；顶栏有阶段与主 CTA。
- [ ] AC2：在设置页触发/收到 HITL 时可见角标，并可一键进入确认切面完成 resume/skip/cancel。
- [ ] AC3：文档切面能打开已有 `PRD.md` / `PRD_REVIEW.md` / `TRD.md`；工作区切面展示路径且能打开目录；无选中时有空态回列表。
- [ ] AC4：列表「待我处理」与角标数量一致（同机事件可达时）。
- [ ] AC5：侧栏不再以主入口展示渠道/技能/独立工作区；repos/settings 仍可达。
- [ ] AC6：`pnpm typecheck` 通过；桌面窗手测主路径：启动 → HITL → 文档/工作区切面。

## Out of Scope

- 产品 PRD **P2** 专轮（轨迹打磨、非阻塞体验微调）——不阻塞 AC
- 真·多轮续聊协议、内嵌终端、云 Diff、Open IDE 深度集成、花费大盘
- 改 `STAGE_ORDER` / EasyCI / Coding 契约 / Cordis 运行时
- 修改 `submodules/`
- 拆 parent/子任务（已否决）

## Technical Notes

- 详见同目录 `design.md`（ViewModel 派生、home 内表面切换、HITL 导航）。
- 执行顺序与验证命令见 `implement.md` Slice A→E。
- Spec 注入清单：`implement.jsonl` / `check.jsonl`（已 validate）。

## References

- `docs/versions/2.0.4/PRD.md`
- `competitor/轻舟/`
- `src/components/Shell.tsx`, `HomeBoard.tsx`, `PipelineDetail.tsx`, `HumanLoopCard.tsx`, `state/store.tsx`
