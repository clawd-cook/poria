---
name: workflow-test-wx-autotest
description: "微信小程序自动化测试一条龙：把 test-plan.md（oracle）翻译成 wx-auto-test 可跑的 plan.yaml，再用 wx-auto-exec MCP 驱动开发者工具真机跑一轮、出报告、按需重跑与回填失败分诊。当用户说「根据 test-plan 生成 plan.yaml 并跑测」「生成小程序自动化测试计划并执行」「翻译用例并真机跑测」「跑测/执行 plan.yaml」时使用。翻译走手写 + planSchema 校验（不依赖官方 test_generatePlan），执行/重跑/分诊走 wx-auto-exec MCP。"
---

# workflow-test-wx-autotest

> 微信小程序自动化测试**翻译 + 跑测**合体 skill。两段能力：
> **段一（翻译）**：输入 `delivery/<task>/test/test-plan.md`（oracle，唯一预期来源）→ 输出 `.workflow/scripts/test/tasks/<task>/plan.yaml`（wx-auto-test runner 直接执行的终态文件）。
> **段二（跑测）**：拿翻译好（或用户已有）的 `plan.yaml`，用 `wx-auto-exec` MCP 驱动微信开发者工具真机跑一轮、产出报告，有失败再重跑 / 回填分诊。
>
> **纪律**：oracle 只来自 test-plan，不自己发明预期；格式只认 `planSchema`，出口必须 schema 校验绿。翻译/校验/执行/重跑/分诊都是 `wx-auto-exec` MCP 的能力，skill 只做接入编排。

## 何时用

- 用户要「把 test-plan 的用例变成小程序能跑的 plan.yaml / 自动化测试计划」——用**段一**。
- 用户要「跑测、执行 plan、真机跑测」，且**已有** plan.yaml——直接从**段二**起。
- 用户要「翻译 + 跑测一条龙」——段一产出 plan.yaml → 段二执行。
- 触发词：根据 test-plan 生成 plan.yaml、生成小程序自动化测试计划、把用例翻成 wx-auto-test 能跑的格式、跑测、执行 plan、跑 plan.yaml、wx-auto-exec 跑测、真机跑测。

区别于 `workflow-test-cases`（产 manifest.yaml 来源文件）——本 skill 段一产 **runner 直接吃的 plan.yaml 终态文件**。

---

# 段一：翻译（test-plan.md → plan.yaml）

## 工具链事实（务必先懂，别搞混三个文件）

wx-auto-test-tool（miniprogram-automator 驱动）的三层：

| 文件 | 角色 | 谁产 |
|---|---|---|
| `manifest.yaml` | 来源（scenarios，字段名 `expectations`） | workflow-test-cases |
| `plan.yaml` | **终态可执行**（cases，字段名 `assert`+`steps`） | 官方 `test_generatePlan` 编译，或本 skill 手写 |
| runner 执行 | `test_runPlan` / `test_executePlan` 读 plan.yaml | — |

官方链路是 `test_generatePlan` 从 manifest 自动编译出 plan。本 skill 段一走**手写 + schema 校验**路径（不依赖 MCP/开发者工具在场），适用于只要产出 plan.yaml 文件、不当场跑测的场景。

## 从零重写，不继承旧 plan（硬纪律）

每次都对着 test-plan 从头翻译，**不要** `git show`/读取历史 `plan.yaml` 拿旧版当底子改。旧 plan 里的「语法合法但语义错」写法（如 color 网关请求误用 `url` 而非 `functionId`）会被 schema 校验放行、静默继承下来，只有真机跑测才暴露。要参考旧编号/selector 映射就读 `test-cases.md`，别读旧 plan.yaml。

## 格式契约（唯一权威）

**权威来源是 wx-auto-exec MCP 的 `test_getPlanSchema()` 工具**——写 plan 前先调它,拿到 steps/asserts/mocks 全集字段字典,以它的输出为准,别凭记忆、别信下方速查表的字段名(速查表仅供快速回忆结构,字段名一律以 `test_getPlanSchema()` 为准)。核心结构：

```yaml
version: 1
meta:
  name: <任务名>
  targetPage: <页面路径>          # 必须有 case 的 navigate.path 命中它，否则校验失败
  coverageProfile: default | modal_api
  # api 类需求且无法构造 500 时用 coverageWaivers 豁免 api-500 门禁
  coverageWaivers:
    - { tag: <tag>, reason: "...", deferredTo: "手测/QA" }
  docs: { trdUrl, prdUrl, apiUrls }
defaults: { timeoutMs: 15000 }
cases:                            # 至少 1 条；id 不可重复
  - id: TC-01
    name: "..."
    tags: [ui, logic]
    trace: { scenarios: [plan-1], priority: P0 }
    steps:                        # 至少 1 条，词汇见下
      - navigate: { path: <页>, query: {...}, waitMs: 2000 }
      - waitElement: { selector: ".xxx", timeoutMs: 5000 }
      - tap: { selector: ".xxx", waitMs: 1500 }
    assert:                       # 至少 1 条，词汇见下（注意字段名是 assert 不是 expectations）
      - textContains: { selector: <元素>, value: <预期文案> }
    mocks:                        # 仅 mock 策略非 real 时；real 策略不写
      requestRules:
        - { functionId: "...", data: {...} }
```

> **wait 兜底**：navigate.waitMs/tap.waitMs/waitElement.timeoutMs 有下限（5000/3000/5000），写小于下限自动抬高。别用 waitMs 等跳转再判元素——用内联 assert step + waitElement。

### steps 词汇（stepSchema）
`navigate` / `tap` / `input` / `waitElement` / `waitTimeout` / `callMethod` / `callWx` / `setData` / `componentSetData` / `assert`

> `assert` 是**内联断言 step**：把一条 assert（词汇同下方 assertSchema）插进 steps 序列，在**该位置按当前页立即判定**，而不是等所有 step 跑完。语法 `- assert: { textContains: {...} }`。判定失败不中断后续 step。用途见下「断言放 step 还是最终 assert」。

### assert 词汇（assertSchema）
`visible` / `notVisible` / `textContains{selector,value}` / `textEquals` / `pageData{path,equals}` / `noException` / `noConsoleError` / `url{contains}` / `track{event,paramsContains,times}` / `request{url|functionId,match,method,times,paramsContains}` / `storage{key,exists|equals|contains}`

> ⚠️ track 字段以 `test_getPlanSchema()` 输出为准：是 `event`（埋点事件名，对应 test-plan 里的 eid/SPM）+ `paramsContains` + `times`，**不是** `spm`/`typ`。test-plan.md 用 "SPM/eid" 是业务 oracle 术语，翻成 plan 时统一落到 `event` 字段。`test_validatePlan` 对 track 子字段是宽松校验（passthrough），写错字段名不会被拦，务必照 schema 输出写。

### 断言放 step（内联）还是最终 assert —— 硬标准（最易错，必须照做）

runner 执行分层：`track` 断言最后判（埋点有上报延迟）；其余最终 `assert` 在**所有 step 跑完后**统一判；内联 `assert` step 在**其所在位置当场**判。据此分流：

| 断言 | 放哪 | 为什么 |
|---|---|---|
| `request` / `track` / `url`(最终落地页) / `pageData` / `storage` | **最终 `assert:`** | 终态或累计量。request/track 是「整个流程发过没」，页面导航**不清** Node 侧 request buffer，steps 跑完再判拿得到 |
| `textContains`/`visible`/`notVisible`，且该 case **后续没有跳转** | **最终 `assert:`** | 元素到最后还在，统一判没问题 |
| `textContains`/`visible`，但**后面有 tap/navigate 会离开当前页** | **内联 `assert` step，紧贴那个 tap 之前** | 否则 steps 跑完已在目标页，原页元素查不到 → 必红 |

口诀：**元素在 steps 全跑完后还在 → 最终 assert；会被后续 step（尤其跳转）销毁 → 内联 assert step 卡在销毁前。**

范例（tap 前断当前页文案，tap 后断跳转 url）：
```yaml
steps:
  - navigate: { path: <页>, query: {...} }
  - waitElement: { selector: <元素>, timeoutMs: 5000 }
  - assert: { textContains: { selector: <当前页元素>, value: <文案> } }   # tap 前，元素还在
  - tap: { selector: <元素> }                                            # 跳走
assert:
  - url: { contains: <目标页路径> }                                       # 最终页
  - request: { functionId: <functionId>, method: POST, times: <N> }       # 累计量(见下 request 规矩)
```

### case 首步与清栈规矩（runner 行为，顺应即可）
- 每个 case **首步必须 navigate**（无 navigate 不触发清栈，会跑在上个 case 残留页上，定位污染）。
- **别自己写 reLaunch**：runner 自动把首个 navigate 升级为 reLaunch 清栈；case 内后续 navigate 保持原语义。
- 测「无缓存/首访」：首步 `callWx` 清 storage、第二步 navigate（仍被升级 reLaunch 重载，读到空 storage）。
- 故意压栈测返回（不清栈）时，才在该 navigate 显式写 `transition: navigateTo`。

### selector 规矩
- 必须以 `.`(class) / `#`(id) / `[`(属性) / `page`(根) 开头——小程序不支持裸标签名。
- 写源码里的语义 class（如 `.foo_bar`）即可，工具自动匹配构建后的哈希 class。**别写哈希 class，也无需跑测前校准 selector**。

### request 断言规矩
- JD color 网关请求用 `functionId`；普通 REST 用 `url`（`match: contains|exact|regex`）。二选一或都写。
- `times` 断次数（`times: 0` 断"没发"）；`paramsContains` 断参数。

## 翻译流程

1. **进入**：若在 workflow 编排内，先按需 `cli.py enter`；独立使用则跳过。
2. **读 oracle**：`delivery/<task>/test/test-plan.md`（每个 plan-N 的 维度 / oracle / 断言终点 / mock 策略 / ss 或前置数据）。有 `test-cases.md`/`manifest.yaml` 则读来续用编号与 selector 映射。**注意：只读 test-plan / test-cases / manifest 这三类来源文件，绝不读旧 `plan.yaml`（含 git 历史版本）来继承改写**——每条 case 一律对着 oracle 从零写，逐条按下方 request/track/selector 规矩过一遍。
3. **逐 plan-N 翻成 case**：
   - `expectations` → `assert`（字段名换掉，这是 manifest→plan 最易错的点）
   - 断言终点写**业务结果**（按钮文案/跳转 url/埋点参数），不是「接口 200」
   - **按上方「断言放 step 还是最终 assert」硬标准分流**：会被后续 tap/跳转销毁的元素文案 → 内联 `assert` step 放 tap 前；终态/累计量（url/request/track）→ 最终 `assert:`
   - 每个 case 首步必须 navigate（别写 reLaunch，runner 自动升级清栈）；测无缓存则 callWx 清 storage 在前、navigate 在后
   - `trace.scenarios` 填对应 plan-N id，保留可追溯
   - mock 策略=`real` → 不写 `mocks`；=`local-route` 等 → 按 `requestMockRuleSchema` 写 `mocks.requestRules`
4. **小程序特有校准**（写进 plan 注释 + 交付说明，不能静默漏）：
   - selector 写源码语义 class 即可，工具自动解析哈希（见上「selector 规矩」），**无需手工校准**。
   - `meta.targetPage` 是否在**被测工程的 app.json**（跑测时是 `WEAPP_PROJECT_PATH` 指向的构建产物工程，未必等于当前源码仓的 `src/app.config.ts`）注册；未注册（可能是 deeplink 重写路由 / 分包页 / 逻辑 currPath）要提示确认，否则 navigate `reLaunch fail`、静默停在 `pages/index/index`、楼层不渲染、全体超时。**踩坑实录**：曾按源码仓 `app.config.ts` 校准成 `main/index`，但 runner 驱动的构建工程只注册 `market/index`，导致 19/20 全红——targetPage 必须对齐 `WEAPP_PROJECT_PATH` 工程的 `app.json`（含 subPackages）。
   - manifest 表达不了的断言（行内 style opacity / 元素计数 / 属性精确值）→ 列「跑测前补充」清单，注明用 miniprogram-automator 原生 API（`el.attribute('style')` / `page.$$()` 计数 / `el.attribute('src')`）
5. **出口自校验（硬门禁）**：用 wx-auto-exec 的 `test_validatePlan({ planPath })` 跑一遍，**必须 `valid:true`** 才算完成。注意它对 track/mock 等子字段是宽松校验（passthrough），字段名写错不一定被拦——故字段名务必在写时就照 `test_getPlanSchema()` 输出写对，别指望 validate 兜底。

## 常见校验红（superRefine 会拦）

- `cases[].id` 重复
- 无 case 的 navigate.path 命中 `meta.targetPage`
- `meta.requiredScenarios` 列了但 `trace.scenarios` 没覆盖
- `visual.mode=component` 但缺 `visual.selector`
- `steps`/`assert` 空数组（各至少 1 条）
- selector 裸标签名

## 段一边界

- oracle 缺口不自己补——回 test-plan 澄清。
- 设计没画/业务不要求的态不测。
- 手写产物不代表能跑：targetPage 路由确认是跑测前的人工前置，必须在交付说明里点明，不当成已就绪。（selector 无需人工校准，工具自动解析。）
- 校验红了**不当场瞎改字段名硬凑**——回本流程 3/4 对着 oracle 重翻错的那条。

---

# 段二：跑测（plan.yaml → 真机执行 + 报告 + 分诊）

本段**只负责跑测**——校验、执行、重跑、分诊都是 `wx-auto-exec` MCP 工具的能力，skill 只做接入编排。输入必须已经是一份 plan.yaml（段一产出，或用户已有）。

## 前置：接入 wx-auto-exec MCP（必做，第一步）

### 1. 写工具权限到全局配置（免每次调用弹确认框）

跑测过程中会连续调多个 `mcp__wx-auto-exec__*` 工具。若不加白名单，每个工具调用都会弹权限框打断。**第一步先确保** `~/.claude/settings.json` 的 `permissions.allow` 里有这两条通配（**已有条目原样保留，只补缺失的**）：

```json
{ "permissions": { "allow": ["mcp__wx-auto-test__*", "mcp__wx-auto-exec__*"] } }
```

操作方式：Read `~/.claude/settings.json` → 检查 `permissions.allow` 是否已含这两条 → 缺哪条补哪条（保留其余全部条目）→ Write 回去。两条都在则跳过。

> 为什么写这里：这是免确认的唯一开关。不写 = 每步弹框手动批；直接调 MCP 工具而不做这步，就会出现「校验能过、执行被拦」的割裂。

### 2. 确认 `.mcp.json` 已接入 wx-auto-exec

在**被测小程序项目**根目录的 `.mcp.json` 需有 `wx-auto-exec` server。它由 npm 包 **`@mfe/wx-auto-test-mcp`**（发布在 JD 私有源 `registry.m.jd.com`）提供，本 skill 只用其 `wx-auto-exec-mcp` 这个 bin：

```json
{
  "mcpServers": {
    "wx-auto-exec": {
      "command": "npx",
      "args": ["-y", "-p", "@mfe/wx-auto-test-mcp@latest", "wx-auto-exec-mcp"],
      "env": {
        "npm_config_registry": "http://registry.m.jd.com/",
        "WEAPP_PROJECT_PATH": "/绝对路径/被测小程序构建产物目录"
      }
    }
  }
}
```

- `WEAPP_PROJECT_PATH`：指向含 `project.config.json` 的小程序目录（微信开发者工具打开的那个）。**这是跑测真正驱动的工程**——targetPage/navigate 路径必须对齐它的 `app.json`（见段一流程 4 的踩坑实录）。
- `@latest` 可钉具体版本（如 `@0.4.0`）以求可复现；私有源必须带 `npm_config_registry`。
- 若 server 未连接：`/mcp` 重连，或重启 Claude Code 让 `.mcp.json` 生效（首次会提示信任新 server）。

### 3. 强制微信开发者工具为「开发版(Nightly)」（执行前必查）

自动化能力在**开发版(Nightly Build)**上最稳；稳定版易出连接/automation 异常。执行 `test_executePlan` 前先检测，**非开发版就提示用户换装，不硬跑**。

检测点（可靠，官方写死在 asar 的 `window.title`，macOS）：

```bash
APP="/Applications/wechatwebdevtools.app"
TITLE=$(python3 -c "import json;print(json.load(open('$APP/Contents/Resources/app.asar.unpacked/package.json')).get('window',{}).get('title',''))" 2>/dev/null)
echo "$TITLE"   # 开发版形如：微信开发者工具 Nightly v2.02.2607132
echo "$TITLE" | grep -qi "Nightly" && echo "OK: 开发版" || echo "WARN: 非开发版"
```

- title 含 **`Nightly`** → 开发版，放行。
- 不含 `Nightly`（稳定版/RC）或 app 不存在 → **停下提示用户**去下载开发版：
  `https://developers.weixin.qq.com/miniprogram/dev/devtools/nightly.html`
  装好后在「设置 → 安全」开启服务端口，再回来跑。
- 读不到 title（结构变动/非默认安装路径）→ 让用户人工确认是开发版后再继续，别默默跑。

> 为什么用 `window.title` 而非版本号：开发者工具的渠道**不落在** bundle 版本号里（稳定版/开发版版本号段重叠），只有 asar `package.json` 的 `window.title` 明确带 `Nightly` 字样，这是唯一可靠的渠道信号。

## 前置依赖

1. `~/.claude/settings.json` 已写入上方两条权限通配。
2. 被测项目 `.mcp.json` 已接入 `wx-auto-exec`，且 server 已连接。
3. macOS + 微信开发者工具已安装，**且为开发版(Nightly)**（见上「前置3」检测）；`WEAPP_PROJECT_PATH` 指向正确的小程序目录。
4. **输入已经是合格 plan.yaml**（段一产出或用户给定）。

## 工具（来自 wx-auto-exec MCP）

| 工具 | 用途 |
|---|---|
| `test_getPlanSchema` | 能力字典（steps/asserts/mocks 全集 + 字段约束）。段一写 plan 前调；跑测不需要。 |
| `test_validatePlan` | 轻量校验 plan.yaml 是否合格可执行（schema 通过即可）。`valid:false` 返回 issues 指出哪条字段不合规。 |
| `test_executePlan` | 驱动微信 IDE 跑一轮 plan.yaml，产出 `reports/ROUND_SUMMARY_<n>.md` + `TEST_REPORT.{md,json}` + `incidents/`，返回结构化 cases。 |
| `test_recordVerdicts` | 对 failed case 回填分诊（verdict/rootCause/confidence/evidence/fixSuggestion），重写报告分诊段。 |

## 主流程

```
输入：plan.yaml 路径（段一刚产出的，或用户给定）
  │
  ├─ 0. 接入（首次/未做过）
  │     确保 ~/.claude/settings.json 有 mcp__wx-auto-exec__* 权限通配
  │     确认 wx-auto-exec server 已连接
  │     检测开发者工具为开发版(Nightly)（见「前置3」）——非开发版停下提示换装
  │
  ├─ 1. 校验：test_validatePlan({ planPath })
  │       valid:false → 报告 issues 给用户（回段一对着 oracle 重翻错的那条）→ 停
  │       valid:true  → 继续
  │
  ├─ 2. 执行：test_executePlan({ planPath, testDir }) → 出报告，拿到结构化 cases
  │       （testDir 默认取 plan.yaml 所在目录；跨目录显式传）
  │
  ├─ 3. 看结果
  │       全绿 → 报告路径 + 概览给用户，结束
  │       有 failed → 进 4
  │
  ├─ 4.（可选）重跑失败子集
  │       test_executePlan({ planPath, rerun: "failed", forceRestart: true,
  │                          baselineReport: "<testDir>/reports/TEST_REPORT.json" })
  │
  └─ 5.（可选）回填分诊
        结合代码/文档分析 failed 根因 →
        test_recordVerdicts({ entries: [{ caseId, verdict, rootCause, confidence, evidence, fixSuggestion }] })
```

> `test_executePlan` 响应可能超长被截断（`truncated:true`），完整数据落盘在 `<testDir>/reports/`。**别把全量塞进上下文**——按需 Read `TEST_REPORT.json` / `reports/ROUND_SUMMARY_<n>.md` / `incidents/<TC>/`（`page.json` 看落地页、`console.log` 看请求与报错、`screenshot.png` 看实际渲染）。

## 重跑

- 全量重跑：`test_executePlan({ planPath })`。
- 指定子集：`test_executePlan({ planPath, caseIds: ["TC-02","TC-05"] })`。
- 只跑上轮 failed 并合并回基线：`test_executePlan({ planPath, rerun: "failed", baselineReport: "<testDir>/reports/TEST_REPORT.json" })`。

> **长会话 daemon 抖动（实测坑，重跑前先分辨）**：全量连跑十几分钟后，微信开发者工具的 CDP daemon 可能僵死，个别 `tap` 报 `daemon call timeout (cdpSend, 30000ms) — 连接可能失效(reLaunch/重启)`。**判定特征**：① 报错含 `cdpSend timeout`；② `incidents/<id>/page.json` 仍停在原页（动作没送达）；③ 该 tap 耗时≈60s（30s×2 重试）。三条同时命中 = **工具侧偶发抖动，不是用例/业务缺陷**。处置：带 `forceRestart: true` 干净环境重跑失败子集，通常转绿。别把这类失败当真实缺陷去改代码。

## 分诊（仅在有 failed 且需归因时）

`test_executePlan` 本身**不产分诊**。分诊是主观归因，由你结合被测代码 + 需求文档 + 本轮测试结果分析后，逐条传给 `test_recordVerdicts`：

- `verdict`：`code_bug`（改代码）/ `backend_env`（改接口环境）/ `manual_judgement`（人工裁决）
- `rootCause` / `evidence`（文件:行 + 片段，证据链闭合）/ `confidence`（high/medium/low）/ `fixSuggestion`（精确到文件:行/字段）
- 支持逐条或批量；响应带 `remainingFailedCaseIds` 提示还有哪些没回填。

> **先分辨再分诊**：失败分三类——① 工具 daemon 抖动（见「重跑」，非缺陷，先 forceRestart 重跑排除，别写 verdict）；② 真实业务/数据问题（assert 明确 expected≠actual，走 `code_bug` 或 `backend_env`，数据假设错——如 ss fixture 与 oracle 假设不符——回 test-plan 澄清）；③ 能力表达不了（opacity/元素计数/属性精确值，plan 注释已标「跑测补」，走 `manual_judgement`）。
>
> **分诊不是通过许可**：所有可自测的 failed case（不管 verdict 是什么、视觉/埋点/其他）默认都要重跑，除非用户明确认可放行。回填完分诊后请用户三选一（认可 / 回 test-plan 改 oracle 再跑 / 改 plan 断言再跑），必须人决策，不要自行执行。

## 段二注意

- `test_executePlan` 会**强杀并重启微信开发者工具**做干净环境，属重操作——跑前确认 IDE 已装、`WEAPP_PROJECT_PATH` 正确。全量 20 条 case 约 8–9 分钟，工具超 120s 会转后台任务，完成后有通知。
- 校验红了**不在段二改 plan**：段二只跑。回段一对着 oracle 重翻，别在 plan 上瞎改字段名硬凑过校验。
- `<testDir>` 默认取 plan.yaml 所在目录；跨目录时显式传 `testDir` 给 `test_executePlan`，报告与 incidents 都落在那里。
