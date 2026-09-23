# Skip 用例根因归属协议

junit 中 `skipped` 状态的用例**不等于合理跳过**。执行完成后，对每条 skip 做根因归属。

## 判定规则（四选一）

| 根因类别 | 判据 | 处置 |
|---|---|---|
| CODE BUG | 测试期望来自 PRD/TRD 且合法，但被测行为的运行时触发链路断裂 | 转 issue → `issue-add --from workflow-run-autotest`，标 `verdict: code-bug` |
| TEST ISSUE | 触发链路完整（步骤 2 排除了 CODE BUG），问题在测试脚本 | 转 issue → `issue-add --from workflow-run-autotest`，标 `verdict: test-fix` |
| VALID SKIP | PRD/TRD 明确移除了该场景，或需求变更导致该行为不再需要 | 保留 skip，注释中引用需求变更来源 |
| ENV CONSTRAINT | 环境/基建不支持（缺三方沙箱、登录态无法取得、物料依赖人工操作） | 保留 skip，注释中标注前置条件，传递到 handoff-qa 做人工验证 |

## 判定方法（skip 必须过这三步才能定性）

1. **查 TRD/PRD**：该用例覆盖的行为是否仍在需求范围内？仍在 → 排除 VALID SKIP。
2. **查运行时链路**：被测行为在运行时能否被触发？方法：找到用例断言的目标（UI 元素 / 状态值 / 行为），反向追溯从用户入口到该目标的完整触发链路（渲染挂载、状态写入、事件绑定）。链路断裂（定义了但没被消费、声明了但没被调用、注册了但没被路由）→ CODE BUG；链路完整但依赖外部前置条件（配置下发、环境变量、三方服务）→ 确认是前置未到位（ENV CONSTRAINT）还是代码未消费前置（CODE BUG）。
3. **查测试脚本**：若代码可达且逻辑完整（步骤 2 排除了 CODE BUG），则问题在测试脚本（选择器失配、mock 未覆盖该路径、时序竞态等） → TEST ISSUE

## 红线

- 禁止未做根因归属直接保留 skip。「mock 覆盖不到」不是终结答案——要追问「代码是否消费了该字段」，mock 不到可能是因为代码根本没读。
- 禁止把 CODE BUG 标为 skip。skip 的语义是「该用例不适用于当前上下文」，不是「代码没实现但我们先不管」。

## 报告格式

追加到 test-report.md 的 skip 分析段：

```markdown
## Skip 用例根因分析

| TC | 描述 | 归属 | 根因 | 处置 |
|---|---|---|---|---|
| TC-09 | campaign ended 状态 | CODE BUG | ladderCampaignEndedAtom 无写入 | → issue-N |
| TC-XX | 某场景 | VALID SKIP | PRD v2 移除了该功能 | 保留 skip |
```
