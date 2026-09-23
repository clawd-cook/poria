# workflow-code-review

审 implement 产出的 diff。

## 规约(MUST)
仅当项目为 frontend 项目时，必须遵循 `./frontend.md`。

## 执行方式：单 subagent

主窗口派**subagent** 执行。根据需求内容灵活判断，如果是大需求，则单独给任务1派一个subagent，任务2和3公用一个subagent。如果需求小，使用一个subagent也可以。

## 审什么：项目特有 delta + 设计一致性

1.审代码是否遵循设计，每条是否都已经完成，有没有多做的内容。
2.审代码本身是否有优化空间。
3.审是否存在违反 `code-rule.md` `design-rule.md` `code-review-rule.md` 的代码或设计规范。其中`code-review-rule.md`是本阶段专属的团队层面的编码规约补充。
4.**（迭代模式）既有改动的副作用/回归**：需求形态为 iterative（见 context.md）时，额外审「对既有代码的改动是否有副作用」——本次改了的既有文件/类型/组件（迭代协议 §5 增量契约、host 宿主文件等），其**既有消费方/既有行为**是否被破坏（爆炸半径）。greenfield 无此项。
5.**（UI 改写越界）**：有 ui-rewrite 任务时，审改写是否越出 `ui-protocol.md`「改写边界」白名单——只应做 数据/Props/交互/多态 + 画布约束→自适应 + 语义标签 UA 重置，**不得布局重构、样式微调、图片结构重建**（静态稿经 ui-static-integrate 视觉闸的 DOM/样式是已验证的还原基线，改写推翻它即越界）。越出 → finding，根因建议 `workflow-implement`。

## 产物与出口

产出 `delivery/<task>/review/findings-<n>.md`（`<n>` 从 1 起递增不覆盖；记 `reviewed_commit`/`reviewed_at`，每条 finding 含 位置/描述/建议/**根因建议**）。根因建议取 `workflow-implement`（CR 比对的是代码 vs 规约+TRD），供 feedback-loop 记归属参考。

> **CR 不出 `test-*` 根因**——implement 只产业务代码、CR 看不到测试；测试缺陷由 run-autotest/handoff-qa 检出。CR 报的问题 category 通常记 `code`（见 feedback-loop「源→category 约束」）。

- **有 finding**：每条 `issue-add --from workflow-code-review`（context 指 `findings-<n>.md#item-X`；粗判 high 危加 `--severity high`）**记一笔即可，`current_step` 不动**——先记账、别在评审当口就焊（漏点地图不丢，见 PROTOCOL「反馈回路」红线）。**把本轮 diff 评审完、所有 finding 都记下来**，再调 `advance`；advance 被 open issue 挡住时进 `/workflow-feedback-loop`——**默认原地修**（在这儿直接改代码 + 就地复验），不必回退。high 危例外当场停、当场处置。
- **无 finding**：`advance <task> --step workflow-code-review`，照 CLI 打印的闸门指令当场接续。

## 收尾·团队偏好沉淀（独立于 feedback-loop）

本轮 CR 过程中若用户抛出「这种写法我们不喜欢 / 以后 CR 都拦一下 X」类**团队风格偏好**（**非故障、非本次要修的 finding**——就是团队不喜欢的写法/命名/API），退出前汇总这些偏好、**逐条问用户是否固化**，确认的追加进 `docs/spec-rule/code-review-rule.md`「检查项」段（去重合并，一条一行含出处）。

- **这条通道独立于 feedback-loop**：风格偏好不是故障——不建 issue、不锁主线、不记 severity、不进漏点地图。用户没抛偏好则跳过本节。
- **与故障链的红线**：CR 抓到的**实现缺陷**（实现走样、代码 bug）仍走 `issue-add --from workflow-code-review` → feedback-loop → 值得则 pit-record 进 `code-rule.md`（见上「有 finding」）；只有**纯风格偏好**走本节进 `code-review-rule.md`。判据：是「犯了错该防再犯」还是「没错但团队不喜欢」。
- **可机判偏好指路**：能被脚本判的（禁 `==`、命名正则等）建议引导团队走 `/bootstrap-lint-script` 落 `.workflow/scripts/lint/` 脚本兜底，别只写进规约靠 CR 肉眼查。
