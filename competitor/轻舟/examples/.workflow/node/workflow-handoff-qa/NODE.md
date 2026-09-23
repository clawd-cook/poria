# workflow-handoff-qa：转测

产出 `delivery/<task>/handoff-report.md`，**唯一目标：让 QA 不读代码就能开始测**。前置：autotest 任务校验 `test/test-report.html` 存在且 run-autotest 出口 ok。

## 报告要让 QA 答出这几问

不读码就能测，QA 至少得知道：**改了什么、碰了哪些面、怎么测、自动化覆盖到哪、风险在哪**。各问的料散在产物里，照来源取、别凭空写：

- **改了什么**：一句话「解决谁的什么问题」+ PRD 链接（`delivery/<task>/context.md` 起步入口 + `prd/`）。
- **碰了哪些面**：模块 / HTTP·JSF 接口 / 数据表 / JIMDB key / JMQ topic / DUCC 配置项（`git diff master...HEAD --stat` +（有则）TRD）。
- **怎么测（重点）**：必测路径及各自期望、多租户隔离、边界/异常、灰度切流观察点（`test-plan` 角色区域标 `manual` 的项 + 人工补）。
- **自动化覆盖到哪**：用例数指向 test-cases.md，结果指向 HTML 报告（不内联）。
- **已知风险**：TRD「兼容性」「上线策略」小节。

部署信息取 `deploy` 角色区域。不抄 PRD/代码全文，只引用。

> **无自动化测试的 profile降级**：无 test-plan/test-cases/report。「自动化覆盖」写「本任务未做自动化测试」；「怎么测」改为 AI 从 TRD/PRD + diff 提炼**全量人工必测路径**——这是本任务唯一的测试兜底，要写得更完整。

## 出口与边界

`advance <task> --step workflow-handoff-qa`，照 CLI 打印的闸门指令接续，确认问题里带上：QA 有问题人工触发 `/workflow-feedback-loop`，QA 通过才进 archive，未出结果可不确认、停驻等 QA。

- **不自动等 QA 结果**——QA 异步，产出报告即推进 state。
- **QA 反馈先分流再建档**（进 feedback-loop 的 issue 必有阶段根因）：QA 理解偏 → 解释关闭；PRD 本身错/缺 → 上报 PO，不进回路；环境问题 → 管道外处理。**确认是真问题**才 `issue-add --from workflow-handoff-qa` 建档记账（粗判 high 危加 `--severity high`，别绕过建档就焊，见 PROTOCOL「反馈回路」红线）——建档后进 `/workflow-feedback-loop`（或 `--new` 人工登记）**默认原地修**（直接改代码 + 就地复验），category 落 design 或 code（QA 的比较两端是行为 vs 需求理解，不含测试产物）；「autotest 怎么没拦住」是拦截轴信息进漏点地图，该补的回归用例顺手补。仅用户主动要求或问题大到该重做时才回退。
