---
name: workflow-feedback-loop
description: "反馈回路处置器。把验证型节点（test-plan/code-review/test-cases/run-autotest/handoff-qa）记下的问题处置掉：默认「原地修」——在当前节点位置直接解决（含改代码）、就地复验、issue-resolve 闭环；只有用户主动要求（或问题大到该重走一大段）才「回退」reroute 到上游。核心价值观：先解决、后沉淀——解决完才轻量记 category（design/code/test 沉淀归属+漏点地图），值得才调 pit-record。纯沉淀（记一下）直调 workflow-pit-record。"
---

# workflow-feedback-loop：把「记下的问题」解决掉，然后轻量沉淀

> **进入特例**：本节点 enter **条件反转**——须有 open issue 才放行（无 open issue 时纯沉淀直调 `/workflow-pit-record`）。通用进入协议见 CLAUDE.md。

**核心价值观——先解决，后沉淀。** 优先级是：① 把问题解决掉，让用户尽快拿到正向结果；② 解决之后再轻量记归属（漏点地图）。**默认在当前节点原地修**（含改代码），不动辄回退——回退会把 `current_step` 搬回上游、逼整条下游重走，用户久久拿不到解法的结果。回退是**例外**，只在用户主动要求、或问题大到确实该重走一大段时才走。

**处置分两档，默认第一档**：

- **原地修（in-place，默认）**：在当前节点位置直接解决问题（**可以改代码、改用例、改文档**）→ 就地复验刚修的那一小片 → `issue-resolve` 闭环。`current_step` 不动，修完 advance 继续前进。
- **回退（reroute，例外）**：仅当**用户明确说「回退到 X」**，或问题大到该从上游某节点重做时——`reroute --to <上游节点>` 搬 `current_step`，下游重走。候选见下方 reroute_options 表。

**沉淀是解决之后的事，不是解决之前的闸**：`category`（design/code/test）只决定问题记进哪本 `docs/spec-rule/*-rule.md` + 进漏点地图。它在 `issue-resolve --category` 时顺手记一笔（可选），不阻断解决。

**category 三分类 = 沉淀层的团队铁律**：

| category | 含义 | 沉淀去向 |
|---|---|---|
| `design` | 没设计好（TRD/方案缺口、设计层认知错） | `docs/spec-rule/design-rule.md` |
| `code` | 编码有问题（实现走样、代码 bug） | `docs/spec-rule/code-rule.md` |
| `test` | 测试设计 / 测试编码问题（oracle 没说清、用例漏场景或用例自身 bug） | `docs/spec-rule/test-rule.md` |

**explore + propose 结论为定论，不作回退目标**：TRD 缺决策 = 探讨没覆盖 → propose 当场口头问人拍板、结论回写产物；TRD 与探讨结论不符（转写走样）→ 原地修补 TRD。生产型节点 propose/implement 撞到探讨/TRD 未覆盖的业务语义缺口时**口头修复**——问人拍板、结论回写产物，不建 issue、不回流。

来源名单（`feedback.can_report`）、各节点 `reroute_options`、`rerouteable` → 单一源在 profile（`.workflow/profile/`）+ `.workflow/engine/_lib/nodes.py`；issue 协议在 `.workflow/engine/PROTOCOL.md`；CLI 完整签名读 `.workflow/engine/cli.py`。

## 入口

```
/workflow-feedback-loop <task>                 # 处理 open 队列首个
/workflow-feedback-loop <task> <issue-id>      # 处理指定 issue
/workflow-feedback-loop <task> --new "<描述>"   # 人工登记还没修的问题（QA 反馈 / 随手发现的 bug）
```

`--new` 先 `issue-add --from manual` 建档再处置；验证节点记下的 issue，「现象」「关联上下文」已由上游填好，本 skill 接力把它解决 + 闭环。

**分流闸——本回路只收「还要修」的问题。** issue 进 open 队列即锁 advance（干完当前节点、想推进时被挡）。纯沉淀（「记一下」，已修完或无需修复）直调 `workflow-pit-record`：不建 issue、不锁主线。

**源头过滤——进本回路的 issue 必有归属。** 没有阶段可背的失败（环境抖动、QA 理解偏、PRD 本身错）在来源节点就地消化、不建档（协议见 run-autotest「失败先判真伪」、handoff-qa「反馈分流」）。建了档的必然落 design/code/test 之一。

## 何时进本回路——干完当前节点、advance 被挡时

**不要一发现问题就冲进来。** 验证节点发现问题时只 `issue-add` 记一笔（`current_step` 不动），**继续干完当前节点其余的活**；干完调 `advance` 被 open issue 挡住，那时才进本回路把累积的问题一起处置。例外：粗判 **high 危**（数据丢失/安全/资金/不可逆，`issue-add --severity high`）当场停、当场处置，不拖到最后。

## 一、解决：默认原地修，用户要才回退

对每条 open issue，读现象 + 关联上下文 + 对应上游产物，判它属于哪一档处置：

- **默认原地修**：绝大多数问题都在当前节点位置直接解决——TRD 清晰但实现成别的语义（改代码）、oracle 没说清某场景（补 oracle 文档）、用例漏场景或用例自身 bug（改用例）。改完**就地复验刚修的那一小片**（重跑那条用例 / 重看那段 diff），确认修好即进步骤二闭环。**不搬 `current_step`、不重走整条下游。**
- **回退（例外）**：仅当①用户明确说「回退到 X」，或②问题大到确实该从上游某节点重做（如整个方案方向错了）。走步骤三的 reroute 分支。

**判定圣经本身该改 = 新需求**：看起来根在探讨/TRD（决策缺失、需求理解偏）的——探讨结论是圣经、不质疑；真判定圣经本身要改，停下来告诉用户开新任务，不在本回路消化。

**回退候选（例外分支才用）**——一个来源能回退到的节点取自它 `feedback.reroute_options`（自动 issue）或全部 `rerouteable!=false` 节点（manual issue）。default profile（`full`）配置：

| 来源 | category 常见归属 | reroute_options（full profile） |
|---|---|---|
| `workflow-test-plan` | design / test | `[]`（无回退候选，只能原地补 oracle） |
| `workflow-code-review` | code | `[workflow-implement]` |
| `workflow-test-cases` | test / code | `[workflow-implement]` |
| `workflow-run-autotest` | code / test | `[workflow-implement, workflow-test-cases]` |
| `workflow-handoff-qa` | code / design | `[workflow-implement]` |
| `manual` | 任意 | 全部 `rerouteable!=false` 节点 |

## 二、闭环：issue-resolve（必标解决方式，顺手记 category）

问题解决后闭环。**先解决后沉淀合成一个动作**：

```
issue-resolve <task> <issue-id> --by "<谁修的，如 workflow-code-review 原地修>" --mode <auto|asked> [--category <design|code|test>]
```

- `--mode` **必填**，见下方口径。
- 给 `--category`：闭环 + 顺手把归属记进漏点地图 + frontmatter。
- 不给 `--category`：只闭环（事后想补归属可再记）。

原地修的问题走这一步即闭环，主线随之解锁，回主窗口跑 `/workflow-engine next` 从当前节点继续前进——**不重走下游**。

### `--mode` 判定口径（照这条判，别自己解读）

**问一句：这条 issue 从建档到闭环之间，我有没有为它主动向人提过问、或等过人回话？**

- **`asked`** 包含：停下来问用户拍板、找人要信息才修得动、以及**人自己动手改的**（人上手 = 人被占用了，算同一类）。
- **`auto`** 是：全程没打断人，AI 自己判断、自己动手、自己复验完的。
- **`--new` 人工登记的（`from=manual`）默认仍是 `auto`**：人报问题给你 ≠ 你打断人。它是人主动来的，你独立修掉了就记 `auto`；报完你又回头问人才修好，才是 `asked`。
- 这个标和 `from` 交叉才有意义：`from=<验证节点>` + `auto` 那一格才是「AI 自己发现、自己解决」。

## 三、回退分支（例外，用户主动要求时）

仅在步骤一判为回退时走：

```
reroute <task> <issue-id> --to <回退节点>
```

CLI 校验目标（真实存在 + `rerouteable!=false` + 不在当前下游，防前跳）后改 `current_step` 到该节点、history 追回流痕。**不把 issue 移到 resolved**——那由下游节点修完时调 `issue-resolve` 做。回退结束即返回，不等下游。**逃生**：同一 issue 回流达 `REROUTE_ESCALATE_THRESHOLD`（3）仍未闭环 → 停止自治、把现象 + 历次尝试列给人升级人工。

## 四、沉淀：值得才走

**不是每条都值得沉淀**——偶发错误进 spec-rule 只会稀释规约。「忘了考虑边界」类认知问题、high 危问题 → 值得，调 `workflow-pit-record`（按 category 映射文件：`design`→`design-rule.md`、`code`→`code-rule.md`、`test`→`test-rule.md`），把返回锚点回填 issue 的 `trap_written_to`。typo / 漏改一行 / 环境抖动类偶发 → 不沉淀。kind 一律 `pit`（经反馈回路来的 = 犯了错）。

## 无人区自裁（run-loop 自驱中且 severity=normal）

人不在现场时，AI 不等人、直接按默认**原地修**解决 + `issue-resolve` 闭环，续 run-loop。此处 `--mode` 恒为 `auto`，high 危不自裁——命中数据丢失/安全/资金/不可逆的，`issue-add --severity high` 锁主线停人（run-loop 因 `HIGH_ISSUE` 终止交还人）。

## 边界

- **必须建档，但默认原地修**：验证型节点发现的是已存在于上游产物的缺陷，**必须 issue-add 建档**（漏点地图双轴 from/category 不丢），**严禁绕过建档就地焊到连账都不记**。但建档之后默认在当前节点原地修（含改代码）+ 就地复验，**不必回退**。回退是用户主动要求的例外。**红线护的是「记账 + 就地复验」，不是「禁止改代码」也不是「强制回退」。**
- **不接 deploy 失败**：按约定 deploy 一定成功。
