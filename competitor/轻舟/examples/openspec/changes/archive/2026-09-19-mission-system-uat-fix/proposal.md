## Why

劳动者任务系统初版在 9.18 UAT 中暴露出 5 个 man 端任务创建/编辑页的体验与规则缺陷：结束时间提示文案与 PRD 规则（自然月倒数第二天）不一致、指定日期任务名称被误设为必填、约束条件可重复添加、同一标签可同时正反选、圈选标签后看不到命中人数。这些问题影响运营正确配置任务，需在上线前修复。

## What Changes

- **提醒文案**：基础信息「任务起止」的 extra 文案修正——自然月自动循环结束时间由「当月最后一天」改为「每月倒数第二天」（自然周仍为当周最后一天）；追加开始时间说明「开始时间为月中或周中时，不生成当月或当周子任务」。
- **指定日期任务名称放开必填**：周期配置「指定日期」类型的子任务名称由必填改为非必填，保留最长 30 字校验，提交时空值传 `null`。
- **约束条件锁死单条**：规则与奖励区去掉「添加更多约束条件」入口与「删除」按钮，每档仅保留一条约束；约束条件类型跟随 DUCC 模板锁死（出勤奖励→差评数量），不可切换，用户仅填阈值；DUCC 未返回时兜底 `BAD_COMMENT`。
- **同一标签正反选互斥**：参与人群标签圈选，选择时即时拦截——在正选/反选中选入对方已含的标签时 toast 提示并阻止选入。
- **圈选人数展示**：标签（正/反选）变更后自动（防抖）调用 `estimateCrowd`，在标签区展示「预计圈选 X 人」；仅标签圈选模式生效。

## Capabilities

### New Capabilities
- `task-incentive-editor`: man 端劳动者任务创建/编辑页四段表单（基础信息、参与人群、周期配置、规则与奖励）的配置校验与交互规则。

### Modified Capabilities
<!-- 无既有 spec，全部以新能力 spec 承载 -->

## Impact

- 仓库：`jdcleaning-man`（单仓，前端 React + antd@3）
- 模块：`src/_v2/pages/positiveIncentivesTaskDetail/`
  - `components/BaseInfo.tsx`（文案）
  - `components/CycleConfig.tsx`（指定日期名称非必填）
  - `components/RuleAndReward.tsx`（约束锁死单条）
  - `components/ParticipateGroup.tsx`（正反选互斥 + 圈选人数）
  - `config.ts`（新增文案常量）
- 接口：复用既有 `POST /taskIncentive/estimateCrowd`，无新增后端接口。
- 不涉及列表页、任务查看页、京家政 C 端。
