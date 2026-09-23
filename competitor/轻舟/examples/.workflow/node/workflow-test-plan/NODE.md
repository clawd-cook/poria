# workflow-test-plan：写代码前的另一视角（oracle）

输入 `delivery/<task>/prd/` + `prompt/` + `openspec/changes/<task>/`。
预读 `design-rule.md` + `test-rule.md`+ `docs/test-knowledge/`。

在任务开始之前，先问用户关于本次需求测试覆盖度的偏好。两档：仅覆盖核心范围 / 全量覆盖。
如果仅覆盖核心范围，那么只需要处理P0+P1的核心用例即可，且**不做纸面外发散**——范围就是 PRD/TRD 写明的功能点，下文「先发散」那段主动找纸面外边界只在全量覆盖档做。全量覆盖就是保持现状。
另外无论是哪种档位，针对P3用例，过于复杂或者过于难构造的用例，都可以适当的选择逃生方案——按下文四档覆盖载体处置，理由写「构造成本过高」；但碰真钱/权限越权/不可逆写的不适用逃生，难写就转手测/QA，不许标「不测」。

## 职责
围绕PRD+TRD的事实依据，产生出业务视角的oracle准则。类似研发的TRD，本文章主要为后续的test-cases提供核心指导，需要前置把所有问题解决清楚才能继续。
oracle 质量有两半，缺一半都不算定稿：
**先发散——把场景找全（覆盖完整性）。** 这是本节点最烧脑、也最容易被漏掉的一步。别只把 TRD/PRD 写明的 happy-path 抄下来就开始细化——TRD 只写大致方向，真正要测的边界往往没写在纸上，靠这里主动想出来 。覆盖判断本身就是 oracle 的核心价值——场景没找全，下游再细化也补不回来。
**（迭代模式）既有行为回归也要覆盖。** 需求形态为 iterative（见 context.md）时，发散除了新功能，还须从**迭代协议 regression 面**（TRD §11 / 迭代协议记录的"改动可能影响的既有行为"）派生**既有用例回归**——改了既有文件/类型/宿主页面，既有功能不能被带坏。别只测新楼层。greenfield 无此项。
**再收敛——对每个场景用四问细化。** 每个场景聚焦四问：测什么 / 为什么测 / 怎样算对 / 环境与数据怎么处理。
每一条用例`plan-N`，要说清楚 环境/物料/步骤/断言/清理方式。

步骤和断言很重要，写的不好可能造成假绿。断言要以业务结果为预期，接口调用成功不是终点，业务结果正确才对。
例子：错：调用接口返回200。 正确：调用成功后，执行数据查询，结果正确。
如何判定业务结果终点？我这个操作最终是为了做什么事？那就查询这个事做到了没。

本阶段需要前置完成物料内容的收集。先自行查找，缺失的向用户索要，权限问题向用户发起授权，要保障在运行阶段实现人不在场的全自动化
逐 `plan-N` 初判物料操作档位（🟢/🟡/🔴）。🔴/🟡的内容需要找用户确认是否授权以及具体的执行粒度，🟢直接用。
物料的输入及权限的授权直接接受，无需验证，不要质疑人工输入的正确性。
验证涉及的环境相关问题需要问清楚再继续，可以在state.json中找到。

物料拿不到、e2e 跑不了时（授权拿不到 / 三方环境缺失），摊牌给用户在四档里选这条 plan-N 的覆盖载体：
1. **授权 / 补物料**（首选，保持 e2e 全自动）
2. **降级单测**——e2e 的兜底替代（仅当前 profile 包含 `workflow-unittest` 节点时可选；无此节点则跳过该选项）：在 materials.md 给该 plan-N 标「覆盖载体=降级单测」，由 implement 后的 workflow-unittest 节点写并跑（mock 掉拿不到的依赖）。注意降级会换掉判定基准：单测验逻辑分支、不验端到端业务结果，**这是覆盖损失**，标记时一并写清损失了什么，别当成等价覆盖。
3. **手测 / QA**
4. **不测 + 原因**

如果用例oracle和物料诉求，没决策清楚，被漏到了test-cases或者run-autotest后面的环节，这是严重问题，不能接受。

## 维度标签（每个 plan-N 必标，可多选）

维度贯穿下游——test-cases 分模板 + run-autotest 分物料闸。

| 维度 | 含义 | 触发信号 |
|---|---|---|
| `api` | 接口请求/响应结构变更 | TRD 新增/修改接口、返回值结构变化 |
| `ui` | 视觉/样式变更 | PRD/TRD 涉及组件样式、布局、颜色、字号、圆角、字体等 |
| `tracking` | 数据埋点变更 | PRD 新增/修改埋点点位、SPM ID 变化 |
| `logic` | 纯业务逻辑 | 兜底维度，其他三个都不属于的归这里 |

plan-N 头部样例：
```markdown
### plan-1: 订单列表空态展示
- **维度**: api, ui
- **接口**: GET /api/order/list
- **mock 策略**: local-route（见下 Mock 策略决策）
- **oracle**: 列表为空时展示兜底图 + "暂无订单"文案
```

维度全为 logic 时，下游行为等同基线，零额外开销。

## Mock 策略选择（api 维度必做）

> **边界澄清**：本节点只决策**通道**（用什么方式 mock）；具体的**响应体内容**（正常/空/异常各返回什么 JSON）由 test-cases 段 1 的 `mock-variants.md` 设计。如果 test-cases 设计 variant 时发现通道选择有误（如接口有 cookie 依赖无法 local-route），走 issue-add 回流本节点修正策略，不允许 test-cases 自行改策略。

**mock 源澄清必须在本节点闭环，不允许下沉到 test-cases 再问**——test-cases 只翻译，不决策。

### 动作 1：项目 mock 能力探索

若 `bootstrap-material-map` 已探过则直接读，不重探。扫描清单：

1. `package.json` 依赖：MSW / nock / miragejs
2. whistle 规则文件（`.whistlerc` / `whistle.rules` / `proxy.config.*`）
3. `src/mock.js` / `mock/` 目录（babel-floor / h5 fixture）
4. dev server proxy（webpack/vite `devServer.proxy`）
5. CI/docker-compose 里的 mock server

产出摘要落 `test-plan.md` 头部：
```markdown
## 项目 mock 能力（探索结果）
- whistle: 未发现
- 框架 mock: src/mock.js（babel-floor 本地数据）
- MSW/nock: 无
- dev proxy: webpack devServer.proxy → test 环境
- **建议本地 mock 方案**: Playwright route.fulfill（项目无独立 mock 层，E2E 层拦截最简单）
```

### 动作 2：Mock 策略决策表

对含 api 维度的 plan-N 逐条推荐 + 一次性列表让用户批量确认（**不逐条打断**）：

```markdown
## Mock 策略决策
| plan-N | 接口 | 推荐策略 | 原因 | 用户确认 |
|---|---|---|---|---|
| plan-1 | GET /api/order/list | local-route | 纯前端渲染分支 | ⏳ |
| plan-2 | POST /api/order/create | local-route | 前端只需验证请求后 UI 状态变化 | ⏳ |
| plan-3 | GET /api/user/info | real | 无变更，直连测试环境 | ⏳ |
```

策略选项：

| 策略 | 含义 | 层级 |
|---|---|---|
| `local-route` | Playwright `page.route()` 内联在 spec（**默认首选**） | 主策略 |
| `real` | 不 mock，直连测试环境 | 主策略 |
| `local-mock` | 利用项目已有 mock 基础设施（`src/mock.js` + whistle/dev-proxy 等组合） | 进阶 |
| `easymock-http` / `easymock-jsf` / `easymock-color` | 远程平台，段 2 出口通过 `lbcli easymock` 上传 | 进阶 |

推荐逻辑：纯前端渲染分支 → `local-route`；不可 mock（需后端状态闭环）→ `real`；以上不满足且项目已有 mock 层 → `local-mock`；需远程共享 mock → `easymock-*`。进阶策略增加 test-cases 出口复杂度，无明确理由时默认 `local-route`。

确认协议：用户可一次性批量回复，支持三种标记：
- `✅`：接受推荐
- `→ <新策略>`：改为其他策略（如 `→ real`），需附一句原因
- `❓`：不确定，本节点再补充信息后二次确认

部分确认即可前进——未标记的条目视为 `✅`（沉默即同意）。

## 埋点校验策略（tracking 维度必做）

跟 mock 策略同构——本节点决策、test-cases 只翻译。对含 tracking 维度的 plan-N 逐条推荐 + 一次性列表让用户批量确认：

```markdown
## 埋点校验策略
| plan-N | SPM | 推荐策略 | 原因 | 用户确认 |
|---|---|---|---|---|
| plan-4 | orderList.exposure | local-intercept + easytrack-verify | 新增点位，参数级 + 平台端双验 | ⏳ |
| plan-5 | orderList.click.item | local-intercept | 老点位仅参数变更，平台端已配置 | ⏳ |
| plan-6 | orderList.filter | easytrack-verify | 走接口透传抓不到，只能平台端验 | ⏳ |
```

策略选项：

| 策略 | 触达 | 判定 |
|---|---|---|
| `local-intercept` | Playwright `page.on('request')` 拦 `/log.gif` / `tunnel-ihub` / 项目埋点 wrapper URL | URL + 参数级校验（能查漏字段/错值） |
| `easytrack-verify` | 跑完 spec 调 `lbcli easytrack verify-triggers` | 平台端到达确认（兜"发了但被平台拒"） |
| `local-intercept + easytrack-verify` | 双开 | 参数正确 + 平台真吃到 |
| `skip` | 不测 + 原因 | — |

**默认推荐规则**：
- 新增点位 → `local-intercept + easytrack-verify`
- 老点位仅参数变更 → `local-intercept`
- 走接口透传 / 服务端上报（前端抓不到） → `easytrack-verify`
- 明确不测 → `skip` + 备注原因

## PRD 规则冲突扫描（出口前必做）

在 oracle 定稿前，对 PRD 中所有业务规则做一次 LLM verify 扫冲突：

1. **直翻**：PRD 中每条业务规则翻译为可测断言形式（"当X条件，应该Y结果"）
2. **LLM verify**：让 LLM 检查断言集内部是否存在矛盾（如"列表为空显示兜底图"和"列表始终显示筛选栏"在空态下可能冲突）
3. **处置**：
   - 无冲突 → 规则直接写入对应 plan-N 的 oracle
   - 有冲突 → 当场挂 PM 裁决（不允许带着冲突出口），裁决结果写入 oracle

**为什么放本节点而非 test-cases**：冲突是 oracle 层面的问题（"怎样算对"没定义清楚），不是翻译层面的问题。带着冲突往下走 = 下游必然 issue-add 回流，白跑一轮。

## 产物与契约

先把问题问清楚，最后再落文档。

oracle 落 `delivery/<task>/test/test-plan.md`（含维度标签 + mock 能力探索摘要 + Mock 策略决策表 + 埋点校验策略表 + 规则冲突扫描结果）。
台账落 `delivery/<task>/test/materials.md`。

本期不自动测的场景也要列出，标「手测/QA」或「不测+原因」——范围判断也是 oracle 的一部分。

## 原地处置场景（verdict=workflow-test-plan，不回退）

写代码后 test-cases/run-autotest 静态或运行时发现「oracle 没说清某场景」，根因记 `verdict=workflow-test-plan`（进漏点地图），但**不 reroute 回本节点**——oracle 是纯文档、调整不涉代码、不让下游已完成产物失效，回退反会从 implement 起重走整条下游（CLI 对它的 reroute 会 loud fail）。改为 feedback-loop **原地处置**：增量补对应 `plan-N` 的预期映射（不重写整份）+ 沉淀 test-rule，`issue-resolve` 后从当前节点继续前进。机械流程见 workflow-feedback-loop skill「四、处置」原地分支。

> no-autotest 任务无本节点产物，test-plan 这条 verdict 经 `VERDICT_FALLBACK` 兜到 implement（正常回退），不在此列。

## 出口

定稿含**四**半，本节点没有「半完成」态：

1. **oracle**——每个 plan-N 映射写定，维度标签打好。
2. **前端策略决策**——
   - 有 api 维度时：项目 mock 能力探索摘要 + Mock 策略决策表用户批量确认完毕。
   - 有 tracking 维度时：埋点校验策略表用户批量确认完毕。
3. **PRD 规则冲突扫描**——LLM verify 已跑完，冲突项已挂 PM 裁决并写入 oracle（无冲突时此半自动达成）。
4. **物料决策**——🔴/🟡 已批量摊牌确认、台账落盘；物料拿不到的 plan-N 已定覆盖载体（授权/降级单测/手测QA/不测），降级单测的覆盖损失已在 materials.md 写清。

四半都齐才 `advance <task> --step workflow-test-plan`；advance 收尾会提示回原主窗口跑 `/workflow-engine next` 续接——下一节点在主窗口展开，不在本窗口调。有 open issue 时 `/workflow-engine next` 会强制先跑 feedback-loop。
