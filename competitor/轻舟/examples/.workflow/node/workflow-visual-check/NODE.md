# workflow-visual-check

视觉回归校验：渲染截图 vs 设计稿截图对比，发现偏差后自动修复 CSS。

> **定位（已不是独立编排节点）**：本节点不在 profile 编排里。视觉正确性拆三分、还原度左移后，本文的**执行流程 + scripts/（runner/diff/report + weapp 采集）+ 七维目视表 + ⚠️待核闸 + renderTarget 探测**作为**校验规则库**，被 implement 的 `ui-static-integrate` 阶段引用（见 `.workflow/node/workflow-implement/ui-protocol.md`「静态集成 + 视觉闸」）——在**静态集成后、组件改写前**跑（数据为设计稿占位、最纯净）。数据态视觉边界归测试流程、改写越界归 code-review。下文流程即该阶段的执行细则。

## 前置条件（由 implement 的 `ui-static-integrate` 阶段触发）

- 全部 ui-static 静态稿已产出、拼成可跑静态页，且已过 build/类型/lint 闸
- 需求包含设计稿
- 有 `.module.scss` 或组件 `.tsx` 变更

以上任一不满足即跳过视觉校验（不阻断 ui-static-integrate 收口）。

## 执行流程

### 0. 渲染目标判定（选采集后端，首步）

引擎的**比对/判定**（diff + significantRegion + 报告）与渲染目标无关，只有**采集**分后端。判定（小程序识别与 workflow-run-autotest「载体分流」一致）：

- **小程序**（根目录有 `project.config.json`/`project.private.config.json`，或 package.json 含 `@tarojs/*`/`taro build`+`weapp`，或 context.md 写明微信小程序）：
  - 开发者工具可用（装好 + 自动化端口开）→ `renderTarget=weapp-devtool`（经 `wxad` CLI 驱动 IDE 截图；采集细节见 step 3、前置见 step 1）；
  - 开发者工具/CDP 不可用 → `renderTarget=visual-only`（跳过 runner、只走 step 4 目视复核，actual 截图由 IDE/用户手动提供）。
- **H5**（有 H5/浏览器 devServer：`dev:h5`/`taro build --type h5`/vite/webpack-dev-server 等）→ `renderTarget=browser`：Playwright 截浏览器（采集细节见 step 3、前置见 step 1）。

结论写 context.md「视觉校验配置」段的 `renderTarget` 字段。**各 target 的前置准备与配置字段见 step 1。**`visual-only` 时 runner 整段跳过、不视为不合规。

### 1. 前置准备与配置

#### 前置准备（需用户配合的逐项确认就绪，未就绪不跑）

**`browser`（H5）——用户二选一（`AskUserQuestion`；选哪种决定配哪些字段）**：
- **A 自动 devServer（默认，零前置）**：runner 自动 `spawn` devServer 命令（`npm run dev` 等）、访问 `localhost:port`、自带编译 watch。无需用户手动。
- **B whistle 代理（iterative 看真宿主 / 需登录态时）**：用户**自行起本地 dev server + 自行起 whistle 并配好代理规则**（把被开发楼层资源映射进线上页）——这两件 runner 不碰；runner 只按 config 的 `proxy` 走代理截图。逐项确认：dev server 已起 / whistle 已起且规则生效 / 线上 URL 可访问 / 登录态就绪。**cookie 不会自动带**——Playwright 是干净 context、不共享你系统 Chrome 的登录态；需登录态时用 `storageState`（导出的登录态 JSON）**或**在 whistle 规则里注入 cookie，二选一。此模式 runner 探 `pageUrl`（线上）可达即跳过 spawn，故 dev server 须用户先起。

**`weapp-devtool`（小程序）——5 项前置，缺一不可**（逐项确认就绪再跑）：

- ⓪ **全局 wxad 包**（runtime 依赖）：先 `command -v wxad` 探；未命中则报安装命令 `npm install -g @jd/wxa-debugger --registry http://registry.m.jd.com` 让用户确认后装（机器级改动，不静默全局装）。
- ① **编译代码 + 显式确认产物最新**：需要用户完成编译并确保产物最新。**编译完成后须让用户显式确认「产物已含本次改动」再跑**（`AskUserQuestion` 一键确认,勿默认跳过）——iterative 宿主常有常驻 watch 后台编译,但版本/时机不定,拿旧产物截图会让视觉闸假通过或误判偏差。runner 启动会打印产物 `app.json` 最后编译时间辅助核对;时间明显早于本次改动 → 先重新编译再确认。（编译本身 runner 不做,注意用项目要求的 Node 版本,别用主 agent 默认版本盲 build。）
- ② **IDE 开自动化模式**：设置 → 安全 → 开「CLI/HTTP 调用」+ 服务端口（否则 `wxad` 连不上 CDP、`init` 失败）。
- ③ **目标页到位方式**（`AskUserQuestion` 二选一，写入 `pageMode`；字段详情见下方配置表）：**manual**（缺省、稳）用户手动在 IDE 把模拟器停在**已编译**的被测页，runner 只 attach 截；**auto** runner 用 `wxad goto <pageRoute>`（wx.reLaunch）自动导航到目标页，须配准 `pageRoute`。**两模式都要求先编译好**（runner 不构建）。
- ④ **产物路径 `weappProjectPath`**：指向构建产物目录（含 `app.json`，非源码根）。预测方法见下 note。

> **weappProjectPath 预测**：小程序常「业务仓 + 基座仓」一起才能预览——业务代码编译进基座、在基座整体构建出产物再用 IDE 打开；基座常与业务仓**并行目录**、名 `wxapp`、产物目录 `wxa-build`。故按 `{业务仓父目录}/wxapp/wxa-build` 预测（父目录 = 业务仓根上一级）：先跑 `ls -d {业务仓父目录}/wxapp*/wxa-build 2>/dev/null`——命中则作 `AskUserQuestion` 默认推荐项（用户可改），未命中让用户直接给路径。**判存在性禁用截断的 `| head`**，直接 `ls -d …/wxapp*` 精确探（路径因项目而异，务必用户拍板、不擅自定）。

**`visual-only`（小程序 IDE 不可用，step 0 判定）**：无自动采集，runner 跳过、只走 step 4 目视。

#### 首次运行：把「视觉校验配置」写进 context.md

context.md 无「## 视觉校验配置」段时，推断 + `AskUserQuestion` 确认后，把字段逐行 `字段: 值` 写进该段（runner 的 parseConfig 从这里 split + 逐行解析）。按 renderTarget 写：

| renderTarget | 必写字段 |
|---|---|
| 通用（都写） | `renderTarget`、组件 selector 行（`- Name: { nodeId: "..", selector: "[class*='..']" }`）、`pageNodeId`；`designScale` 改过倍率才写 |
| `browser`·A | `cmd` / `port` / `pageUrl`(localhost) |
| `browser`·B | `proxy`（whistle 地址如 `http://127.0.0.1:8899`）/ `pageUrl`（线上 URL）/ `storageState`（登录态 JSON **绝对路径**，需登录态时）|
| `weapp-devtool` | `weappProjectPath`（构建产物目录）/ `weappPort`（缺省 9223）/ `pageMode`（`manual` 缺省 \| `auto`）；`pageMode: auto` 时加 `pageRoute`（目标页路由+参数）+ 可选 `readyOnNetwork`（首屏接口 URL 子串）——无需 `cmd`/`port`/`pageUrl` |

推断来源：`cmd`/`port` 读 package.json scripts（仅 browser·A 需要）；`viewport` 读 px 配置（H5 `postcss.config.js` viewportWidth / Taro `config/index.js` designWidth）；selector 由 tasks.md 组件根 className 生成——**必须属性通配 `[class*='componentName']`，禁止 `.componentName`**（CSS Modules 编译后类名带哈希）。

可选字段（不配则用引擎缺省，一般无需显式写）：

| 字段 | 缺省 | 何时需要配 |
|---|---|---|
| `browserChannel` | Playwright 自带 chromium | 顽固内网 CDN 依赖在自带 chromium 加载不出时，设 `chrome` 用本机系统 Chrome（仅 `browser` 生效） |
| `designScale` | 2 | 与 explore 阶段「CDN 图片导出倍率」同源，改过倍率时同步 |
| `pageSelector` | 无（整页 fullPage） | 宿主页含其他楼层/无关内容，需把整页对比收敛到被测楼层根节点 |
| `regionMinArea` | `(12 × designScale)² px²` | 显著偏差块的最小像素面积（比对图坐标系，随 designScale 缩放）。字体渲染噪声导致普遍误报时才放大，**且须用户确认**（见步骤 6） |
| `regionMinFillRatio` | 0.35 | 显著偏差块的最小填充率，用来把实心块（真偏差）与稀疏轮廓（抗锯齿噪声）分开。一般不动 |
| `threshold` | `{component: 0.95, page: 0.92}` | 不得为让报告变绿擅自下调；确有系统性噪声时向用户说明并确认后才调，并在 context.md 记下原因 |

**「相似度 99.9% 但 FAILED」不是 bug**：相似度是全局比例，小元素偏差会被大面积正确像素稀释（实测 12px 图标缺失，相似度仍 99.95%）。引擎同时检查 `significantRegions`：连通偏差块中面积 ≥ `regionMinArea` 且填充率 ≥ `regionMinFillRatio` 的块有一个即 fail。填充率的作用是把实心块（元素缺失/错位，实测 1.0）与稀疏轮廓（字体抗锯齿噪声，实测 0.07）分开——遇到这类报告去看 `significantRegions` 的坐标与填充率，不要调阈值。

**数据来源**：本节点跑在静态集成阶段（数据为设计稿占位、**不接接口**），故无接口 mock 环境检查——静态占位数据取设计稿占位（文案/图/条数，与 `export_image` 基准同源）即可。browser·B 代理模式下宿主其余楼层的真实接口由用户/whistle 保证，被测楼层仍为静态占位。（接口 mock 策略属改写后的数据态，归 run-autotest / test-plan §6，不在本节点。）

### 2. 下载设计稿基准图

调 MCP 工具获取设计稿截图并下载到 `delivery/<task>/visual-check/baselines/design/`：

- `export_image(pageNodeId, scale=<designScale>)` → `_page.png`
- `export_image(component.nodeId, scale=<designScale>)` → `{name}.png`

> `export_image` 必传 `designId`（= context.md「UI 参数」段设计稿链接的 `id`/fileKey，整个任务同一值；用法见 `relay-mcp.md`）——上面简写省略了它。

`<designScale>` 取 context.md 「视觉校验配置」的 `designScale`（未配则 2，与 explore 阶段
「CDN 图片导出倍率」同源）。**导出倍率必须与该字段一致**——引擎按它设 `deviceScaleFactor`
与像素比对宽度；基准图与截图倍率不一致时降采样会让文字抗锯齿永远对不齐、相似度被系统性压低。

缓存：若文件已存在则复用。设计稿有更新时手动删除 `baselines/design/` 后重新下载——节点自身不检测设计稿变更。

**禁止用 actual 截图覆盖 design baseline**（`cp baselines/actual/* baselines/design/*`）——那等于拿自己跟自己比，100% 假通过，掩盖所有真实 CSS 偏差。design baseline 唯一合法来源是 `export_image(nodeId)`。

**各组件 nodeId 须与 mock 数据实际渲染的变体状态一致**：引擎首次跑完后，对比 `baselines/actual` 与 `baselines/design` 截图，视觉结构（列数、滚动方向、元素数量）不一致说明 nodeId 对应帧有误，须换对应变体帧的 nodeId 重下基准图再对比；mock 数据变更后须重新确认。

### 3. 运行校验引擎

```bash
cd .workflow/node/workflow-visual-check/scripts && npx ts-node runner.ts --context=../../../../delivery/<task>/context.md
```

引擎自动：启动 dev server → Playwright 截图 → pixelmatch 对比 → 输出报告。

> **`renderTarget=weapp-devtool`**：**同一条命令**，runner 按 `renderTarget` 自动切采集后端——不起 dev server，改用 `wxad init`（起/连 IDE）+ `screenshot --full`（页面级 `_page`）+ 逐组件 `eval`（滚进视口取矩形）+ `screenshot --crop`（组件级）采集，比对/报告与 browser 路径一致。**skyline 页兜底**：渲染层无 HTML DOM、不认 `[class*=]` → 组件取矩形失败则**自动跳过、交 step 4 目视**，`_page` 仍照常。**别启那个永不就绪的 weapp watch 把 runner 卡死。** 前置见 step 1「前置准备」weapp 5 项；`wxad` 依赖**全局 wxa-debugger**（与 wxa-debugger skill 共用同一份、版本单一源，避免「框架内置旧版 vs skill 全局新版」双份分叉互踩）——未装则 `npm install -g @jd/wxa-debugger --registry http://registry.m.jd.com`（runner 找不到 `wxad` 会报此指引）。（背景见 `proposals/ui2code/VISUAL-CHECK-WEAPP-DEVTOOL.md`）

截图与报告落 `delivery/<task>/visual-check/`：

- `baselines/actual/` — 当次渲染截图
- `baselines/diff/` — 差异叠加图
- `reports/visual-regression-report.json` — 判定报告

### 4. 目视复核

引擎跑完即可开始，**不依赖引擎结果**——两路检查独立进行，步骤 5 再综合。

以**设计稿截图为唯一标准**，对照路径均在 `delivery/<task>/visual-check/`。设计稿长什么样就还原成什么样，不要凭经验补设计稿里没有的东西，也不要凭印象判"应该差不多"。

**复核范围**：所有配了 selector 的组件 + 页面级 `_page.png`（覆盖未配 selector 的区域，如页面骨架）。对每一项按以下七个维度逐项过，每个维度出「一致 / 偏差（含具体描述）」结论，不得用整体印象替代逐项判断：

| # | 维度 | 看什么 | 操作要点 |
|---|---|---|---|
| 1 | 元素完整性 | 设计稿有、渲染里没有的元素（**优先级最高**，见下方说明） | |
| 2 | 位置关系 | 各元素在组件内的相对位置、层叠顺序 | `position: absolute` 的悬浮元素（角标、徽章、弹层入口）是高频偏移漏检项 |
| 3 | 对齐排列 | 横向（左/右/居中）、纵向（顶/底/居中）、等分/两端对齐 | **金额/多字号文本的基线**是高频漏检项——`¥`(小) + 数字(大) 须共基线，盒底对齐(`flex-end`)会让 `¥` 浮离数字底部，放大看易漏 |
| 4 | 间距行距 | 元素间距、内边距、多行文本行高 | **分层验证**：外层 gap → 内层 gap → padding，逐层对照；嵌套容器每层单独核对 |
| 5 | 尺寸 | 宽高、图片比例、按钮/卡片大小 | 有宽高约束的图片须确认未被拉伸——宽高比须与设计稿一致；**文字元素须逐行数行数**——单行截断 vs 多行展开是高频漏检项 |
| 6 | 样式 | 颜色、字号、字重、圆角、边框、阴影、渐变 | 每个视觉上独立的元素单独过；**border / outline / box-shadow 是高频漏项** |
| 7 | 图片位置 | 切图在组件内的落位与裁切范围 | `background-image` 须检查 `background-position` 和 `background-size`（contain vs cover） |

**页面级 `_page.png` 复核补充要点**：重点看跨组件区域——块间距、背景色拉通、跨组件的装饰元素（如贯穿多个楼层的渐变背景、悬浮吉祥物）。注意：设计稿全页帧含平台注入的 UI（状态栏、导航栏），不在还原范围内；区分方式：看元素是否在楼层容器内部，楼层外的平台 UI 不算偏差。

> **迭代模式（迭代协议·衔接兜底）**：需求形态为 iterative 时，页面级 `_page.png` 复核**强制覆盖衔接区**——新模块 + 相邻既有兄弟**同框**，重点核**新旧楼层间距/对齐/层叠**是否合 TRD §4「衔接约束」。首次跑的 `pageNodeId` 须取 宿主帧（非新组件帧），否则 _page 里看不到既有兄弟、衔接偏差（如楼层间距缺失）漏检。

**强制输出要求（不可跳过）**：每个组件（含页面级）复核完毕后，必须在当前对话中输出如下格式的结论表，再继续下一个。**全部结论表输出完毕后才能进入步骤 5**——没有书面结论表，等同于未执行目视复核。

```
### [组件名] 目视复核结论
| 维度 | 结论 | 描述（偏差时填写） |
|------|------|------------------|
| 1. 元素完整性 | 一致 / 偏差 | |
| 2. 位置关系   | 一致 / 偏差 | |
| 3. 对齐排列   | 一致 / 偏差 | |
| 4. 间距行距   | 一致 / 偏差 | |
| 5. 尺寸       | 一致 / 偏差 | |
| 6. 样式       | 一致 / 偏差 | |
| 7. 图片位置   | 一致 / 偏差 | |
```

**发现缺失元素**：`issue-add --from workflow-visual-check` 建档（记账，不中断当前节点），继续扫完剩余组件。处置见步骤 6。

### 4.5 ⚠️待核 像素值逐条核（闸门三）

implement 阶段对**填了值但无法从设计稿确证**（靠猜/推断，如某状态态的灰值/间距）的像素值标了 `⚠️待核`。本步逐条核实——这是截图 diff 抓不到的一类（猜测值常视觉接近、相似度不掉，却是错的）：

1. `grep -rn '⚠️待核'` 本任务改动的 `.tsx`/`.scss`，列成清单。
2. 每条**对设计稿核实**——`export_image` / `get_screenshot` / 对该节点 `get_node_data` 取真实样式值，与代码里的值比对：一致 → 清 `⚠️待核` 标记；不一致 → 按步骤 6 修（改样式值）后再清。
3. **MCP 断连降级**：无法读设计稿（工具不可达）→ 对应 `⚠️待核` **保持挂起、不清标、不放行为「已核」**，连上再核。**断连绝不当作「已核通过」**。

> 与闸门二（implement 覆盖对账）共用一张对账表时，状态列取 `✅ / ❌ / ⚠️待核` 三值——闸门二管「在不在」（✅/❌），本闸门三管「对不对」（✅/⚠️待核）。

### 5. 判定结果

**第一步：引擎报告与目视结论交叉对账**

读 `delivery/<task>/visual-check/reports/visual-regression-report.json`，对引擎输出的每一个 `significantRegion`，在目视复核结论里找到对应解释，三类合法解释：

| 类型 | 目视确认 |
|---|---|
| CSS 差异 | 间距 / 颜色 / 对齐 / 截断问题 |
| 数据文案差异 | 结构布局一致，仅文案 / 数值不同 |
| 布局模式不匹配 | mock 触发的变体与 nodeId 帧不符 |

**禁止用一句"整体是文案差异"打包跳过多个区域**——每个 region 必须逐个归类。机器报 N 个显著区域，目视解释条数 < N 且剩余无法归类，说明存在未发现的 CSS 问题，不得放行。

**未配 selector 的区域不得因此跳过**：context.md 里未配组件 selector 的区域（如 ActivityHeader、页面骨架）必须通过 `_page.png` actual vs design 目视对比覆盖，发现偏差走步骤 4 目视复核流程处理。

**第二步：综合三份结果判定**

| 来源 | 通过条件 |
|---|---|
| 引擎报告 | 每个组件 `passed: true`（相似度达标 **且** 无显著偏差块）。**`weapp-devtool` 时页面级 + 已成功采集的组件走引擎；skyline 未采集到的组件转目视**；**`visual-only` 时本行整体 N/A**——无引擎报告，判定改由目视复核 + ⚠️待核承担，交叉对账亦跳过 |
| 目视复核结论 | 每个组件七维度全部「一致」 |
| 交叉对账 | 所有 `significantRegion` 已归类，无未解释区域（`visual-only` 无引擎报告时 N/A）|
| ⚠️待核 清单（步骤 4.5） | 无残留未核 `⚠️待核`——已核过的清标、改过的复核，**断连挂起的算不通过** |

三份全通过 **且** ⚠️待核 清零 **且** open issue 为空 → advance；任一不满足 → 进步骤 6 修复循环。（`visual-only` 时「引擎报告 + 交叉对账」两项 N/A，其余项照常必过——**目视复核此时是唯一自动兜底，绝不可再省**；`weapp-devtool` 时 skyline 未采集到的组件**必须目视复核**，不因引擎没覆盖而放过。）

**仅引擎 `passed: true` 就 advance、跳过步骤 4 目视复核不合规**——引擎判不出缺元素，也拦不住 `regionMinArea` 以下的偏差。

### 6. 闭环修复（max 3 轮）

对步骤 5 判定不通过的组件，先判差异类型再决策：

| 类型 | 判断依据 | 处置 |
|---|---|---|
| ① CSS 问题 | 颜色 / 间距 / 对齐 / 截断等样式偏差 | 见下方修复流程 |
| ② 数据文案差异 | 结构布局一致，仅文案 / 数值不同 | 优先让 mock 数据对齐设计稿占位值重跑；确实无法对齐须逐 region 说明原因后放行 |
| ③ 布局模式不匹配 | mock 触发的变体帧与 nodeId 对应帧不同 | 换对应变体帧 nodeId，重下基准图，重对比 |
| ④ 缺失元素 | 步骤 4 建档的 issue | 见下方补全规则 |

**① CSS 修复流程**：

1. 读 diff 图 + `significantRegions` 坐标 → 定位偏差区域（引擎 pass 但目视有偏差的组件跳过此步，直接用步骤 4 结论）
2. 读该组件的 `.module.scss`
3. 对照设计稿截图判定偏差归属七维度中的哪一项，据此定位到具体 CSS 属性
4. Edit scss 文件——**只改样式值，不改 DOM 结构**。DOM 层级来自静态稿，是已验证的视觉还原基线（见 `.workflow/node/workflow-implement/ui-protocol.md`「改写边界」）；结构层面的偏差走步骤 4 的 issue 通道，不在本节点重构
5. 仅对失败组件重跑 runner：
   ```bash
   cd .workflow/node/workflow-visual-check/scripts && npx ts-node runner.ts --context=../../../../delivery/<task>/context.md --components=ProductFloor,CouponFloor
   ```
6. 重新读报告 + 重做该组件的步骤 4 目视复核

**④ 缺失元素补全规则**：

| 缺失元素类型 | 处置 |
|---|---|
| 纯装饰/静态元素（图标、分割线、角标底图） | 原地补全：参照 TRD §2.1 对照表的 nodeId 与设计稿截图写代码，保持现有代码结构与框架一致 |
| 带数据绑定或交互的元素 | 原地补全，但 Props / 字段路径 / 状态分支**必须先 Read TRD 对应章节**拿到真实定义，禁止凭截图推测字段名 |
| 设计稿有但 TRD §11 已明确排除 | 不算偏差，issue 关闭，跳过 |

补全后 issue-resolve 闭环。若补全受阻（依赖信息缺失、超出本节点范围），进 `/workflow-feedback-loop <task>` 记账（category 落 code），issue 闭环后再回来。

**不得为了让报告变绿去调阈值、`regionMinArea` 或 `regionMinFillRatio`**——它们是判据不是旋钮，改它等于把偏差藏起来。

3 轮后仍有组件未通过步骤 5 判定 → 见出口「阻塞」。

**边界**：本节点职责是「校验还原度 + 修样式 + 补缺失元素」。诱惑在于顺手把看着不顺眼的地方一起改了——不要。重构布局、调整 DOM 层级、优化命名、动业务逻辑均不属本节点，发现了 `issue-add` 记账。

## 出口

- 通过 → 满足步骤 5 判定条件，advance（闸门 auto）
- 阻塞 → 3 轮后仍有组件未通过，**或有未核的 `⚠️待核`（含 MCP 断连挂起的）**，输出报告 + diff 图（SendUserFile），不可 advance
