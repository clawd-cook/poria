# 需求起点：劳动者任务系统 UAT 修复

**PRD**：`prds/【PRD】劳动者任务系统.md`
**分支**：feature_20260812_mission_system_uat（从初版 feature_20260812_mission_system 切出，专处理 9.18 UAT）
**改动模块**：`src/_v2/pages/positiveIncentivesTaskDetail/`（man 端任务创建/编辑四段表单）

## 9.18 UAT 待办（5 项，均已与用户拍板）

1. **man 端提醒文案**：BaseInfo「任务起止」extra 文案——自然月结束时间由「当月最后一天」改为「每月倒数第二天」（自然周仍为当周最后一天）；追加开始时间说明「开始时间为月中或周中时，不生成当月或当周子任务」。用默认拟稿文案。
2. **指定日期任务名称必填放开**：CycleConfig 指定日期段 `periodLabel` 去掉 `required`，改非必填；保留最长 30 字校验，提交时空 label 传 null。
3. **约束条件锁死单条**：RuleAndReward 去掉「添加更多约束条件」入口与「删除」按钮；约束 condType 跟随 DUCC 模板锁死（出勤奖励→差评数量）不可改，只填阈值；DUCC 未返回兜底 BAD_COMMENT。
4. **同一标签正反选互斥**：ParticipateGroup 正/反选选择时即时拦截——选入对方已含标签时 toast 提示并阻止选入（不进保存时统一校验）。
5. **选标签展示圈选人数**：ParticipateGroup 标签（正/反选）变更后自动（防抖）调 estimateCrowd，在标签区展示「预计圈选 X 人」；仅标签圈选模式，不加基础信息联动、不做手动按钮。

## 范围划界
- 仅 man 端任务创建/编辑四段表单；不动列表页、查看页、京家政 C 端。
- 项1 只补提示文案，不改 RangePicker 实际日期校验逻辑。
- 项5 仅标签模式展示预估，白名单沿用 successCount。
