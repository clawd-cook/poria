## 1. 文案常量准备

- [x] 1.1 在 `config.ts` 新增任务起止提醒文案常量（自然月倒数第二天 + 自然周最后一天 + 开始时间月中/周中不生成当月/当周子任务），供 BaseInfo 引用

## 2. 项1 提醒文案（BaseInfo.tsx）

- [x] 2.1 将「任务起止」Form.Item 的 `extra` 由「结束时间须为当月/当周最后一天」改为引用新常量：自然周为当周最后一天、自然月为每月倒数第二天，并追加开始时间为月中/周中时不生成当月/当周子任务的说明

## 3. 项2 指定日期任务名称非必填（CycleConfig.tsx）

- [x] 3.1 去掉 `periodLabel_${key}` 的 `{ required: true, whitespace: true }` 校验规则，保留 `max: CYCLE_SEGMENT_NAME_MAX_LENGTH` 校验
- [x] 3.2 确认 getValues 组装 cycleSegments 时空 label 传 `null`（当前为 `(...||"").trim()`，需改为空串转 null）

## 4. 项3 约束条件锁死单条（RuleAndReward.tsx）

- [x] 4.1 移除「添加更多约束条件」入口（`onAddConstraint` 触发的 `<a>`）
- [x] 4.2 移除约束条件行的「删除」按钮（`onRemoveConstraint` 触发的 `<a>`）
- [x] 4.3 约束条件类型 `cCondType` 由可选下拉改为锁定展示（跟随 DUCC 模板的非主约束，出勤奖励→差评数量；DUCC 缺失兜底 BAD_COMMENT），运营仅可填阈值
- [x] 4.4 清理因单约束不再使用的 `onAddConstraint` / `onRemoveConstraint` 及相关渲染分支，确保 getValues 仍正确收集这一条约束

## 5. 项4 正反选互斥（ParticipateGroup.tsx）

- [x] 5.1 为正选 Select 增加 onChange 拦截：选入反选已含标签时阻止并 toast 提示
- [x] 5.2 为反选 Select 增加 onChange 拦截：选入正选已含标签时阻止并 toast 提示

## 6. 项5 圈选人数展示（ParticipateGroup.tsx）

- [x] 6.1 引入 `TaskIncentiveEstimateCrowdApi`，标签圈选模式下正/反选变更后防抖调用预估（入参含 serviceLine/cityScopeType/cityList/crowdConfig，从 getBasicInfo 读取硬过滤字段）
- [x] 6.2 在标签区展示「预计圈选 X 人」；加载中/失败/未选标签时的兜底展示，失败不阻断表单
- [x] 6.3 组件卸载或标签清空时清理防抖定时器与预估态

## 7. 验证

- [ ] 7.1 本地跑 lint/type 检查通过（本环境 node_modules 未安装，lint/tsc 均不可用；已改为逐文件手动走查确认无悬挂引用与语法错误。CI 或装依赖后需补跑）
- [ ] 7.2 手动走查任务创建页：新建出勤奖励任务，逐项验证 5 个改动点（本环境无法起前端服务，留待 code-review/联调环境执行）

