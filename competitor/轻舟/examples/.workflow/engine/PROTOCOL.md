# 节点公共协议（PROTOCOL）

你是 orchestrator，驱动一个确定性状态机。跨节点的执行协议（enter 门禁 / advance 闸门 / pending_gate 书签 / issue 协议）单一源在 **`PROTOCOL.md`**。

进**编排内节点**（非 `start_node`）时读本文——它是节点共享的执行协议与心智，**单一源，各节点不复述**。**本窗口首次进编排内节点才读**；`start_node`（探索/定位，状态机外、`init` 前跑）刻意不读本文。状态机真值（序列/闸门表/回流表/state schema）在 `_lib/*.py`；运行时指令由 CLI 打印。**CLI 输出即指令：是给你执行的，不是转述给用户手敲的。**

CLI 入口统一为 `python3 .workflow/engine/cli.py <subcommand> <task>`，下文简写子命令名。

## 总则

- `delivery/<task>/state.json` 是单一事实源，**只有 CLI 能写**——任何 skill 不得手改。
- 每个节点生命周期对称两触点：**enter（入口门禁）→ 干活 → advance（出口推进）**。
- CLI 拒绝时照它指的路走，不绕过、不重试原样命令。
- 识别用户的输入意图，查询 `status` / `next` 打印即止，是给人看的导航。继续 `next` 的输出是**指令，照它行动、别停在打印**
- `feedback.open` 非空时主线**不得推进**（advance 被挡=收口点）——但 `issue-add` 记账**不中断当前节点**（current_step 不动，可继续干完当前节点的活）。收口点在 advance：干完想离开当前节点时才被挡、才处置。优先级高于任何闸门。

## 入口：enter

起手第一个动作：`enter <task> --step <本节点名>`。

- 校验进入资格：current_step 匹配、主线未被 open feedback 锁定。**拒绝即停**，照打印的去向走。
- 放行时打印上下文包：测试目标环境、回流工单（如有）、执行方式提醒、出口闸门预告。
- **项目级钩子**（`.workflow/profile/` 的 profile 节点项的 `hooks`，若配）：enter 会多打一行「项目附加指令」（节点开工时 AI 须额外执行的单元）+ 一行「收尾指令预告」（advance 前 AI 须执行的单元）。CLI 按 node_ref 打印（skill 打 `/名`、文档节点打斜杠别名 `/workflow-engine node <名>`），照打印的去执行。
- 顺手清除上一转场的 `pending_gate`（= 机械确认转场已兑现）。

feedback-loop 条件反转：`enter <task> --step workflow-feedback-loop` 要求**必须有** open issue。

## 出口：advance 与转场闸门

干完调 `advance <task> --step <本节点名>`，CLI 推进 current_step、打印 `闸门：<gate>` + 一行执行指令，照做。若该节点配了 `hooks.exit`，advance 会当场打印「收尾指令」——转场前 AI 须执行那个单元（按 node_ref 打印，非念给用户；不阻断推进）。三档（取严序 auto < confirm < confirm-locked）：

| 闸门 | 你的行为 |
|---|---|
| `auto` | 直接执行下一节点（照 CLI 打印的 `/workflow-engine node <名>` 别名或 `/skill`），**不要问，自动执行**——人可能不在现场 |
| `confirm` | 需用户确认，但绝大多数情况会直接放行——用 AskUserQuestion 一键确认降低打断成本，确认后执行下一节点 |
| `confirm-locked` | 锁定人工闸：产物只有人能认证 / 上线等不可逆动作，审核要花真实精力和时间。停下等待，**不要用 AskUserQuestion 一键确认催促**（一键确认会诱导用户没审就点过）；用户明确批准后再继续。**不可被 run-loop 降级** |

实际闸门 = `stricter(出口闸[刚结束节点], 入口闸[即将开始节点])`，每节点由 profile 声明 `exit_gate`/`entry_gate`。「锁定不可降级」语义由档位值本身表达（`confirm-locked`），与节点名解耦。default profile 的钉死点：test-plan 是重思考（产物只有人能认证）、propose 是转写审计（TRD 是否忠实于探讨结论）——出口钉 `confirm-locked`；deploy 入口钉 `confirm-locked`（转场层「进上线要人点一下」）+ 非 `autopilot`（run-loop 层「撞到即交还人」），它的不可逆安全真正落在节点内部闸与节点文档（`.workflow/node/workflow-deploy/NODE.md`）的上线前置清单——**安全放节点内部，转场闸只管要不要人点一下**）。

**断点书签（pending_gate）**：advance 把转场指令写进 `state.main.pending_gate`——转场没当场兑现的（窗口关了、被打断），下次 `status` / `next` 会重提这条指令，断点可续。**重提的指令在用户表达继续意图时是要照办的（按其编码的闸门级别执行），不是只回贴给用户手敲**。兑现 = 下一节点 enter 自动清除；用户拒绝/放弃转场 = 跑 `gate-ack <task>` 清。进独立窗口的转场**不写 pending_gate**，那个方向只能人推。

### 项目配置 `.workflow/config.json` + `.workflow/profile/`

跨任务不变量，`config.py` 只读消费；文件/字段缺失即 loud fail（清单见 `config.py` 头注）。

- `test_env.target`（`test` / `gamma`）= 测试物料写操作 / 部署 / 回归**必须钉死的环境**；无独立测试环境填 `gamma`（预发即测试）。取自 `.workflow/config.json`。
- `.workflow/profile/<名>.json` + `.workflow/config.json` 的 `default_profile`：每文件一份**完全独立**的编排（profile 名=文件名 stem，值=profile 对象），节点集/顺序/闸门零共享、互不推导。`init --profile <名>` 选模式，不给用 `default_profile`。引擎只读节点声明字段、不认节点名，故可自由定义节点集编排出不同开发模式。**profile 对象与单节点的完整字段语义（含缺省值、`has_e2etest`/`enabled`/`autopilot` 等语义）单一源在 `.workflow/config.json` 的 `_doc_profiles` 键，需要时 Read 它，本文不复述。** init 时把选中 profile 的 nodes 解析冻结进 `state.pipeline`、roles 替换 `<task>` 冻结进 `state.roles`，运行时只读冻结值。
- **profile 全部来自项目级** `.workflow/profile/`：用户自写 profile 直接放该目录即可（文件名即 profile 名、无需前缀），`lb init` 只覆盖框架自带的几份、保留用户自写文件（并保留用户对框架自带 profile 的 `enabled` 调整）。`default_profile` 取项目级 `config.json`、须存在且启用。列候选跑 CLI `list-profiles`（打印全部启用模式），供 `workflow-start` 转贴给用户选。

### 产物区域（role）——路径单一源

profile 声明 `roles`（语义名 → 落盘路径，如 `trd`→`openspec/changes/<task>/`）；init 时把 `<task>` 替换成真实任务名、冻结进 `state.roles`。**各节点文档只用 role 名指「产物属于哪个语义区域」，不复述具体路径**——要拿真实路径跑 `roles <task>` 查（`state.roles` 无该 role 或 profile 未声明 roles 时降级看节点文档正文的硬编码路径）。这是路径的单一源，改路径只改 profile 的 `roles`。

### 起步入口（context.md）

需求起点（plan/change/prd 的位置或一句话描述）写在 `delivery/<task>/context.md`，由 AI 在 init 后写、CLI 不碰。切窗口 / 新窗口 / external-window 节点冷启动时读它找回起点。

### 起步节点（start_node）

profile 的 `start_node` 是该模式的**起步/触发节点**（feature 流 `workflow-explore`、bug 流 `workflow-bugfix` 等）：**状态机外、`init` 之前跑**的一道澄清节点，刻意不加载编排细节、不读本文（分阶段加载），结论靠**同会话上下文**带进下游、不进 state、不设门禁。选 profile + 衔接起步的完整流程（正门 `workflow-start`、`init` 守卫拦「跳过起步直接 init」）见 CLAUDE.md /「起步正门」与 `workflow-start` skill。

> **无「无人模式」持久开关**：自治不固化到任何配置——它是 run-loop 每步 `advance --loop` 的动作属性。

## 无人区自驱（run-loop）

`run-loop <task>` 是无人区的循环控制点：CLI 不能替 AI 执行节点，故它是**每节点边界 AI 调一次的「分类器 + 指令」**，循环体活在对话里。**完整机制（自治/`--loop` 降级语义、出口钉死 vs 停人节点两套判定、`autopilot` 语义、zone-entry 强制确认、终态分类真值表、循环协议）单一源在 `.workflow/engine/RUN-LOOP.md`**——只在跑 run-loop 时用到。


## 反馈回路（workflow-feedback-loop）（issue 协议）

**核心价值观——先解决，后沉淀。** 遇到问题：① 先把它解决掉、让用户尽快拿到正向结果；② 解决后再轻量记归属。**默认在当前节点原地修**（含改代码），回退是例外。

发现问题的节点 ≠ 制造问题的节点。`from`（谁拦到的=拦截轴）与 `category`（问题归属 design/code/test=根因轴）是两根独立的轴。

**本文只定「撞到问题当下的记账纪律」**（每个节点干活时都可能用到）；**进 feedback-loop 之后的处置流程（原地修/回退/闭环/沉淀的操作步骤、回退目标判据）单一源在 `workflow-feedback-loop` skill**——那时该 skill 由 CLI（`advance` 被 open issue 挡住 → 「先跑 /workflow-feedback-loop」）触发加载，本文不复述。

- **发现即记账、干完再处置**：normal 问题只 `issue-add --from <本节点>` 记一笔，`current_step` 不动、不中断，**继续干完当前节点**；干完 advance 被 open issue 挡住（收口点：有问题不得越过当前节点）时才进 feedback-loop。别一发现问题就中断当前工作冲去分诊。high 危（数据/安全/资金/不可逆）例外：`--severity high` 当场停、当场处置。
- **红线——验证型节点必须建档**：验证型节点（code-review/test-cases/run-autotest/handoff-qa）发现的是**已存在于上游产物的缺陷**，**必须 issue-add 建档**（漏点地图双轴 from/category 不丢），**严禁绕过建档、连账都不记就焊掉**。建档后**默认在当前节点原地修**（含改代码）+ 就地复验，不必搬 `current_step` 回上游、不必重走下游；回退是用户主动要求或问题大到该重走一大段时的例外。**红线护的是「记账 + 就地复验」，不是「禁止改代码」，更不是「改代码必回退」。** 与**生产型节点**（implement/propose）撞**未覆盖业务语义缺口**的口头修复（前向、无可归因，问人回写产物即走）区分：验证型节点无口头修复许可。
- **进回路的 issue 必有归属**：环境噪音、QA 误解、PRD 本身错在来源节点就地消化、不建档。
- **lint 失败不走回路**：不调 advance、state 停在 lint，原地修后重跑 lint 节点（重新 `enter --step` 并按其 NODE.md 执行）。
- **纯沉淀**（已闭环的「记一下」）直调 workflow-pit-record，不建 issue、不锁主线。

## 特殊执行方式

- **独立窗口节点**（节点声明 `execution: external-window`）：在新开窗口运行（隔离视角 + 人在回路），enter 靠跨窗口共享的 state.json 校验；advance 后照 CLI 提示回主窗口跑 `next` 续接，不在本窗口调下一节点。
- **单 subagent 节点**（节点声明 `execution: subagent`）：enter / advance 在主窗口调，干活派一个 subagent、只返回蒸馏结论（细则见各自节点文档「执行方式」）。执行方式在上一节点 advance 转场时并入闸门指令预告。
