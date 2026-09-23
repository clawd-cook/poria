# Code Review Findings #1

- reviewed_commit: de403d3379cb7628cd3eadb3f68c8d1e26ea2902（工作区未提交改动）
- reviewed_at: 2026-09-19
- reviewer: workflow-code-review (single subagent)
- 范围：mission-system-uat-fix 5 项 UAT 改动（config.ts / BaseInfo / CycleConfig / RuleAndReward / ParticipateGroup）
- 总体结论：无 high 缺陷，可合入；2 条 normal 建议顺手修复。

## item-1 [normal] 预估人数触发 BaseInfo 全量校验副作用

- 位置：`ParticipateGroup.tsx` `scheduleEstimate` 内 `getBasicInfo()` 调用（约 196 行）+ Page 传入的 `getBasicInfo=() => basicInfoRef.current?.getValues()`
- 描述：`scheduleEstimate` 里调 `getBasicInfo()` 会走 `BaseInfo.getValues()` → `form.validateFields()`（全量校验）。运营仅修改人群标签、或编辑页挂载 500ms 后触发预估时，会顺带触发基础信息表单必填校验，导致无关字段冒红。不阻断保存，但属交互回归。
- 建议：预估入参改为「有则带、缺则空」的**只读取**方式——不触发 BaseInfo 校验。可在 BaseInfo 暴露一个 `getRawValues()`（用 `form.getFieldsValue()` 而非 `validateFields`）供预估读取硬过滤字段。
- 根因建议：workflow-implement
- **修复结果（原地修，auto）**：BaseInfo 新增 `getRawValues()`（用 `form.getFieldsValue()` 不触发校验），Page 传入 `getBasicInfoRaw`，ParticipateGroup 预估改用它取硬过滤字段。白名单上传仍走原 `getBasicInfo`（需校验）。改动文件：BaseInfo.tsx / Page.tsx / ParticipateGroup.tsx。issue-1 已闭环。

## item-2 [normal] 约束 condType 显隐兜底不一致

- 位置：`RuleAndReward.tsx` 展示 `lockedCondType = constraint.condType || constraintCond?.condType`（约 227-228 行）vs `getValues` 提交 `condType: c.condType || null`（约 322-326 行）
- 描述：展示用带 DUCC 模板兜底，提交侧无兜底。正常路径 state 恒有值、两者一致；仅当某行 `condType` 为空的边界，界面显示模板值而提交为 null，存在显隐不一致隐患。
- 建议：提交侧同样兜底到 `constraintCond?.condType`，保证所见即所提交。
- 根因建议：workflow-implement
- **修复结果（原地修，auto）**：`getValues` 收集约束时新增 `fallbackCond`（DUCC 模板非主约束），`condType`/`op` 缺失时兜底到它，与展示侧一致。改动文件：RuleAndReward.tsx。issue-2 已闭环。
