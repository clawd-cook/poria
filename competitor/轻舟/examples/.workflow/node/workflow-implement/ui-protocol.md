# 前端 UI 实现协议

> 仅在 workflow-implement 编码阶段、且任务涉及 UI 组件实现时生效。
> 加载时机：主 agent 进入 UI 任务前 Read 本文件；subagent 规则由对应的 build-prompt 脚本自动提取注入。

---

## 主 agent 规则

### 执行约束

当处理 UI 组件（`ui-static` 静态稿阶段 / `ui-rewrite` 改写阶段）时：

1. **复杂度分档（开工前判）**——静态稿 schema `node_count ≤ 3`（如单图节点）→ **跳过双 subagent，主 agent 直接内联**（单 `<Image src={Props.xxx}>` + Props 绑定），无视觉还原可做、双 subagent 冷启动纯浪费；有实质结构（多层容器）→ 走 **relay-schema 双 subagent 流程**（build 生成 schema、静态稿完成判据、阶段顺序见 tasks.md ui-static 书式与下方「subagent 编排」，本条不复述）。**合成组件占位**（§11 签收、无单一容器、无单根 static draft）同走主 agent 内联：`get_node_data` 各成员、组装成 `<Comp>` wrapper 按设计位置分组（成员各按对照表归类物化），不上双 subagent。
2. **失败降级（唯一合法路径）**——build 直连失败按其 `status: fallback` 落回手动流程（鉴权失败先 `lbcli relay get-token` 刷新后重跑）；母版预取失败 → 对应 box 实例自动降级切图占位（`renderer:image`，不阻断）；手动流程仍失败（`get_node_data` 不可达 / 返回空 / `normalize` 报错 / 静态稿 subagent 未产出文件）→ 标注 `/* TODO: 静态稿生成失败 */` + 暂停通知用户，不得跳过。
3. **样式/图片来源唯一性**——UI 组件样式中的颜色/字号/间距/圆角只能来自静态稿或 `export_image` 返回值，禁止使用占位 URL 或猜测 CDN 地址；来源彻底不可达（工具挂/无节点）时标注 `/* TODO: 设计稿缺失 */`。**凡填了值但无法从设计稿/schema 确证（靠猜/推断，如某状态态的灰值）→ 标 `⚠️待核`**，不得静默用猜测值发布——交 visual-check 逐条对设计稿核实（闸门三）
4. **接口调用一致性**——接口调用方式必须与 TRD 第 5 章标注的 Mock 策略一致
5. **子步骤完成标准**——task 内嵌的所有子步骤全部执行后，才能将该 task 标为 `[x]`；部分完成的 task 保持 `[ ]`
6. **组件接入**——从项目已有组件库 import，不重复实现；Props 类型以 .d.ts / interface 定义为准，不得假设

### 上下文纪律（路由不搬运）

主 agent 是**编排者/路由器，不是读者**：大产物（layerData / schema 全文 / prompt 全文）留在磁盘，手里只握**路径 / 摘要 / 计数**——每要读一份原文前，先问「这一步真需要全文吗，还是路径/计数就够」：

- **`get_node_data` 落盘、不做二次细读（仅手调路径）**：ui-static happy path 下 get_node_data 由 `build` 脚本内部拉取 + 落盘，主 agent 不接触；**仅 build 回退的手动流程 / explore 分析阶段**才手调——此时大返回 MCP 自动落盘给 tool-results 路径，直接把路径喂给 `list-masters` / `normalize`，**不要再手动 Read 那个落盘文件**；小返回虽内联在工具结果里（MCP 决定，无法避免），也**写盘后即打住**、按路径往下走，别逐层拆读。母版 `get_node_data(childless)` 同理。
- **只读 `normalize`/`build` 摘要，不读 schema.json 正文**：默认回 `{schema_file, 压缩比, node_count, signals 计数}`（信号明细已内联进 schema.json 供静态稿 subagent 消费）；**不 Read schema.json**，需人工核对才 `--verbose`。
- **build-*-prompt 只回 `prompt_file` 路径**：spawn 时 prompt 只写「Read <prompt_file> 并严格照其执行」，prompt 全文不进主 agent。
- **集成类校验优先下沉 subagent**：需逐个读组件源码的接线或核对（如跨组件交互联动、页面级状态订阅），**组件多时**派集成 subagent 做、主 agent 只收结论，别把 N 个组件源码拉进主上下文（两三个组件直接读签名核对更划算，不必 spawn）。

### 前置：读 context.md「## UI 参数」并对齐配置（UI 任务开工前必做）

从 `delivery/<task>/context.md`「## UI 参数」段读 explore 落盘的值，各驱动一件事：

- **设计稿基准宽度**（帧映射表标准产出项，如 375）→ 对齐项目 px 转换配置（见下表）；
- **CDN 图片导出倍率**（默认 2x）→ `export_image` 的 scale 参数；与 CSS px 值无关，仅用于导出高清图片适配 Retina 屏；
- **小程序渲染模式**（webview / skyline）→ 编码须兼容，具体约束见项目 CLAUDE.md 或 `docs/tech-knowledge/`；
- （设计稿链接含 `id=designId`，`build` / `export_image` 调 MCP 用，`relay-schema-gen.py` 自行从此段解析，无需手动记）

**px 转换配置对齐**（核心原则：CSS 中直接写设计稿 1x 原始值，px 转换由构建工具自动处理）：

| 项目类型 | 检测文件 | 配置参数 | 对齐动作 |
|----------|---------|---------|---------|
| H5（postcss-px-to-viewport） | `postcss.config.js` | `viewportWidth` | 设为设计稿基准宽度 |
| Taro 小程序 | `config/index.js` | `designWidth` + `deviceRatio` | `designWidth` 设为设计稿基准宽度 |
| 无 px 转换插件 | — | — | 跳过，提示用户手动确认适配方案 |

若当前配置值 ≠ 设计稿基准宽度（如 viewportWidth=1125 但基准宽度=375），**向用户说明差异并确认是否修正**——项目可能有意使用非 1x 配置，擅自修改会破坏已有样式。用户确认后再修正；用户拒绝则按现有配置换算（CSS 值 = 设计稿值 × 现有配置值 ÷ 基准宽度）。

### 接口调用

基于 TRD 第 5 章接口契约，按状态处理：

| Mock 策略 | implement 动作 |
|---|---|
| `real` | 直接调用线上接口 |
| `local-route` | 代码中写死 mock 数据；接口就绪后替换 |
| `local-mock` | 正常实现调用 + mock 数据写本地文件 + whistle 代理 |
| `easymock-http` | 正常实现调用（mock 由远程平台提供） |

### mock 数据图片规则

**适用所有含本地 mock 数据的任务**（local-route / local-mock 策略均适用），不限于 UI 任务：

mock 数据中涉及图片 URL 的字段：
- 生成 mock 数据时（基础架构阶段），图片字段暂填空字符串 `""`
- 集成阶段由主 agent 根据对照表（`delivery/<task>/schema/component-table.md`）找到图片字段对应的设计稿 nodeId，调 `export_image` 导出 CDN 地址并回填 mock 数据
- 找不到对应设计稿节点的字段，保持空字符串（表示后端未配置），不造假 URL
- **禁止使用不存在的占位 URL**

**运营可配 / 配置驱动的图片槽**（来自项目配置项而非接口 data，mock 里默认空串）同样按上法回填 mock；差别是它运营可配——组件对空值别静默 `return null`，要有**设计派生的默认切图**兜底，否则头图/艺术字标题会整块空白、还指不到具体组件。具体配置项名/mock 路径见项目 CLAUDE.md（如通天塔 `config.json` 的 imguploader）。

### subagent 编排

主 agent 按 tasks.md 条目逐步执行即可，prompt 构建由脚本完成：

- **骨架层**（kind: ui-skeleton）：`build-skeleton-prompt.py` 脚本自动提取本文件「骨架 subagent 规则」段（`## 骨架 subagent 规则` 至下一个 `##`），注入 CDN 倍率 + task 中的 subagent 上下文
- **静态稿层**（kind: ui-static）：`build-static-draft-prompt.py` 脚本自动提取本文件「静态稿 subagent 规则」段（`## 静态稿 subagent 规则` 至下一个 `##`），注入 CDN 倍率 + 精简 schema 文件路径 + task 中的 subagent 上下文
- **改写层**（kind: ui-rewrite）：`build-ui-component-prompt.py` 脚本自动提取本文件「组件 subagent 改写规则」段（`## 组件 subagent 改写规则` 至文末），注入 CDN 倍率 + 骨架文件路径 + 静态稿文件路径 + task 中的 subagent 上下文

**阶段顺序（视觉左移，别把静态稿和改写连着做）**：`ui-skeleton → 全部 ui-static → ui-static-integrate（还原度视觉闸 A）→ 全部 ui-rewrite → ui-rewrite-integrate（页面级装配）`。**同一组件的静态稿（ui-static）与改写（ui-rewrite）分属两阶段、中间隔一道静态集成视觉闸**——先把所有组件静态稿都产出、集成成静态页、过视觉闸，再统一进改写。**视觉闸不过、不得进 ui-rewrite。**

#### 静态集成 + 视觉闸（kind: ui-static-integrate 执行规则）

还原度校验（像不像设计稿）在此做——此刻数据是设计稿占位、未接接口，是最纯净的比对点：

1. **拼可跑静态页**：骨架已 import 各 ui-static 组件；确认整页能渲染，设计稿占位数据到位（文案/图/条数取设计稿占位，与 `export_image` 基准同源）。
2. **承载环境**：
   - **greenfield**（整页新建）：直接跑纯静态页，可零接口零 cookie；
   - **iterative**（只做部分楼层/弹窗，多数需求）：宿主其余是既有代码、该调接口带 cookie 照旧——`pageUrl` 指真实线上 URL，配 `proxy`（本机 whistle 等）把被开发楼层资源映射进线上页、配 `storageState` 注入登录态（均在 context.md「视觉校验配置」）；视觉对比用 `pageSelector`/组件 selector **收敛到被测部分 + 衔接区**，宿主其余真实数据不在对比范围。
3. **构建/类型/lint 闸（跑 runner 前，先保证页面能预览且无基本错误）**：dev server 容忍类型错、按需编译——**「dev 能渲染」≠「能构建」**。故跑视觉前先过项目自己的**整包**校验：从 package.json scripts 取 `build`（或 `typecheck` / `tsc --noEmit`）+ lint 跑一遍，**全项目、不缩到改动文件**（预览是整页，校验粒度也是整页）。
   - **过了** → 进 runner。
   - **不过** → **把报错原样抛给用户、由用户裁定怎么处理**，不自行降级或跳过：本任务新增/改动文件引入的错 → 必修；若是宿主仓**既有、与本任务无关**的报错（iterative 遗留仓常见）→ 交用户决定（忽略 / 另行处理 / 要求先清），别替用户判。
   - 项目无 build/lint script → 无可跑，告知用户后跳过。
4. **跑校验器**：`cd .workflow/node/workflow-visual-check/scripts && npx ts-node runner.ts --context=<相对路径>/context.md`——页面级 `_page` + 按 selector 截各组件区域逐像素比设计稿；辅以七维目视 + ⚠️待核像素值核（详见 `.workflow/node/workflow-visual-check/` 的校验规则）。**各渲染目标的前置准备（`browser`·A 自动 devServer 零前置 / `browser`·B whistle 代理 / `weapp-devtool` 四项）+ 首次配置字段，见 `.workflow/node/workflow-visual-check/` NODE.md step 0-1「渲染目标判定 / 前置准备与配置」**——需用户配合的逐项确认就绪再跑。
5. **闸门判定**：相似度达标 + 无显著偏差块 + 七维一致 + ⚠️待核清零 → 过闸进 ui-rewrite；否则**原地修静态稿/scss**（只改样式值、不改 DOM 结构）后重跑，max 3 轮仍不过则阻塞报告。
6. **浏览器**：截图走 runner 的 Playwright（完整 chromium）；顽固内网 CDN 依赖切 `browserChannel: chrome` 用系统 Chrome。**别用 agent 内置 preview 预览器**（沙箱限制致依赖加载失败）。

> 数据态视觉边界（真实数据下的溢出/空态/极值/换行）与改写后的自适应差异**不在此闸**——它们需构造数据态，归测试流程（test-plan §6 + 功能自动化测），别在这里用逐像素卡。

---

## 骨架 subagent 规则

<!-- 由 build-skeleton-prompt.py 自动提取注入骨架 subagent prompt -->

以下规则由脚本注入到骨架 subagent 的 prompt 中，subagent 必须遵守：

### 任务定位

你的任务是基于 `get_design_context` 返回的设计稿结构，编写页面骨架 JSX + 样式文件。

骨架的产出：
- 页面根组件文件（JSX/TSX）：包含页面整体布局容器结构 + 各子组件的标签引用占位
- 样式文件（scss/less）：各容器的布局和视觉属性（来自 className）
- 子组件只写 `<ComponentName />` 标签引用和 import，不实现内部结构
- **直接写入项目已有的页面入口文件，不新建中间骨架组件**

### 核心原则

- `get_design_context` 返回体可能很大（100K+），**禁止全部塞进上下文**——必须保存文件后按需读取，只关注外层 2-3 层容器
- 骨架层只关心模块容器的外层属性（布局方式、模块间距、跨模块背景等），子组件内部结构由各自 subagent 获取；样式值一律从设计稿节点坐标读取，不得凭习惯值估算

### 设计稿图层→DOM/CSS 转换规则

设计工具中每个图层都是节点，但不是每个节点都应成为 DOM 元素。从 `get_design_context` 读取节点时，先判断再生成：

| 设计稿图层特征 | 处理方式 |
|---|---|
| 无子节点、无文字、无交互的纯视觉效果（渐变条、阴影、装饰线等） | **不生成 DOM**——视觉效果合并到父元素 CSS（`background-image`/`box-shadow`等），或直接跳过 |
| 页面根容器通过 absolute + 固定坐标排布子模块 | **不保留 absolute**——页面是可滚动文档，模块排列用 flex/block + margin/gap；组件内部的 absolute（badge、浮动气泡等局部定位）不受此约束 |
| 有内容/交互/语义的节点 | 正常生成对应 DOM 元素 |

**骨架中引用子组件时的属性拆分**：skipInternalNodeIds 的节点会被骨架读取并生成 wrapper + 组件标签。该节点的设计稿属性需要区分归属：

| 属性类型 | 骨架 wrapper 写什么 | 组件自身（骨架不写） |
|---|---|---|
| 定位/间距 | 从 absolute 坐标推导出的布局方式（flex/gap/margin 等） | — |
| 跨模块背景 | 跨越多个子组件的背景色/背景图（写在页面根容器或 wrapper 上） | — |
| 视觉/内容 | — | background、border-radius、padding、font 等（仅属于该组件自身的） |

视觉属性留给组件 subagent 通过静态稿获得，骨架不重复声明。

**例外——带视觉的容器，视觉别从骨架与组件的缝里漏**：上表默认「视觉归组件、骨架不写」，但当带视觉（白底/圆角/padding）的容器**其组件根比它更深**、或它**下挂多个兄弟子组件**时，这层视觉既不属于骨架的纯布局间距、也不落在任一子组件里，不显式处理就整片丢失（底色透上来、内容贴边）。此时**骨架把该容器渲成带视觉 wrapper**（写出 background/borderRadius/padding），而非用无背景的通用列容器（如 `.content`）替代。判据：容器有 background/borderRadius/padding 任一 → 显式 owner；纯透明布局层才可折叠。

**新增模块的排布（迭代加楼层最典型，先看它）**：

- **迭代模式（迭代协议·布局衔接归骨架）**：骨架 root **取 宿主帧**（含相邻既有兄弟的父容器/页面帧）、**别取新组件自己的帧**——钉在组件帧就看不见兄弟，整条空转、楼层间距必漏（与相邻楼层贴死/错位）。root 对了，**照兄弟排布沿用**（宽度/对齐/间距），并**对账 propose 量好的 TRD §4「衔接约束」**、不自己猜。
- **参考兄弟的依据（通用，非迭代专属）**：模块在父容器内怎么摆（整宽/自适应、对齐、间距）是**父容器级约定**，本模块设计帧看不出——绝对坐标/居中偏移直译成固定尺寸就偏（居中偏移→固定 `width` 变靠左）。以兄弟实际为准、不预设机制。greenfield 建整页时同容器摆多个模块也照此。
- **边界**：凡「往已有父容器放新模块」都适用（迭代最典型），全新容器无兄弟则按设计帧还原；这属「模块 vs 父容器」组合属性，只骨架层定得了、单组件 subagent 看不到；数据（props/事件）不在骨架，归组件改写 + 集成。

### 列表容器 overflow（渲染数组 `.map` 的容器）

骨架里凡渲染 `xxx.map(<Comp/>)` 的容器（绑定数组字段），条数可变、非快照那几条——**禁用会裁切的裸 `overflow:hidden`**（真实踩坑：mock 塞多条被静默切掉）。默认**随内容展开**（不设固定高 / 用 `min-height`）；**仅当设计定义了有界滚动区**（TRD §6「列表溢出」行 / 设计滚动区帧）才按其值设 `max-height`+`overflow-y:auto`（纵）或 `overflow-x:auto/scroll`+露边（横滑）。§6 没给就默认展开、不自造上界。

> 组件内的数组容器由改写侧「动态内容容器的固定高度」承接同一原则；§6 行只装配进改写 subagent、骨架读不到，故此条是骨架侧落地保证。

### 裁剪规则

跳过不属于骨架层的节点（依据来自 task 中标注的排除列表）：
- `excludeNodeIds`：该 nodeId 及其子行**整段跳过**，骨架中写占位注释（如 `{/* slot: RewardFloor */}`）
- `skipInternalNodeIds`：**只读该行自身 className**，跳过子行，骨架中写组件标签引用（如 `<UplinkJumpOrder />`）。**多个 nodeId 映射同一组件名时收拢成一个标签**——在最靠前成员的位置写一次 `<Comp />`、其余成员一并跳过（组件成员散落无单一容器时如此，骨架不必知道为何散落，按组件名收拢即可；该组件的 wrapper 由主 agent 内联自建承接成员，见「主 agent 规则」复杂度分档）

### 完成标准

- task 中 subagent 上下文列出的所有操作步骤均已执行
- 骨架 JSX 文件已创建，包含页面整体容器层级结构
- **列表容器（渲染数组 `.map`）未被裸 `overflow:hidden` 裁切**：默认展开、仅设计定义滚动区才设 cap（见上「列表容器 overflow」）
- **页面根的 absolute + 固定坐标排布已拆成文档流**（flex/block + margin/gap），未原样保留页面级绝对定位（组件内部局部 absolute 不受此约束）
- **带视觉的容器已显式渲成 visual wrapper**：有 background/borderRadius/padding 任一、且组件根更深或下挂多个兄弟子组件的容器已写出该视觉，未用无背景通用列容器替代导致视觉整片丢失
- **新增模块/容器的排布已与同容器已有兄弟对齐**：读过同容器已有兄弟的排布方式（宽度/对齐/间距）并沿用，未把设计帧的绝对坐标/居中偏移直译成偏离约定的固定尺寸（如居中偏移→固定 width 靠左）；无已有兄弟时不适用
- **（迭代模式）root 取 宿主帧、间距对账 §4**：需求形态 iterative 时，骨架 root 是含既有兄弟的宿主帧（非新组件自己的帧），新模块与相邻既有楼层的间距/对齐已对账 TRD §4「衔接约束」；greenfield 不适用
- 所有 `excludeNodeIds` 对应位置已写占位注释
- 所有 `skipInternalNodeIds` 对应位置已写组件标签引用（含 import）
- 样式文件中的属性值均来自设计稿节点属性，无估算值或硬编码猜测值
- task 中标注的 `export_image` 已实际执行并将返回的 CDN URL 写入样式

---

## 静态稿 subagent 规则

<!-- 由 build-static-draft-prompt.py 自动提取注入静态稿 subagent prompt -->

以下规则由脚本注入到静态稿 subagent 的 prompt 中，subagent 必须遵守：

### 任务定位

你的任务是基于**精简 schema**（relay 图层经 `relay-schema-gen.py normalize` 规范化的产物）生成**静态稿**——纯视觉还原，技术栈匹配目标项目。

静态稿的产出：
- 组件文件（JSX/TSX）+ 样式文件（scss/less），写入主 agent 通过 prompt 注入的 `--static-file` 目录
- 只做视觉还原：**无 Props、无数据绑定、无交互、无埋点**——这些是下游改写 subagent 的职责
- 静态稿是改写 subagent 的**输入基线**，样式值必须可信

### 输入与上下文

**精简 schema 文件**（主 agent 通过 prompt 注入路径）：先 Read 该文件。字段语义：
- `id`：节点 ID——`export_image` 的 nodeId 即此值
- `type`：`block`（容器）/ `text`（文字）/ `image`（图片或待切图形状）
- `name`：图层名（可能缺）——className 由此派生（见「样式还原」）
- `rect`：`{x,y,w,h}`，坐标相对父节点
- `style`：**已翻译好的 CSS 属性**——含 flex（`display/flexDirection/gap/padding*/justifyContent/alignItems`）、固定尺寸（`width`/`height`）、hug 容器尺寸下限（`minWidth`/`minHeight`）、固定子不收缩（`flexShrink:0`）或 `position:absolute`，以及视觉属性（background/color/borderRadius/border/boxShadow/fontSize/fontWeight/fontFamily/lineHeight）。
- `text`：文字内容（仅 text 节点）
- `segments`：**单个 text 节点内逐字符字号不同**的混排分段（如价格 `¥3588.15` = `[{¥,12px},{3588,22px},{.,16px},{15,12px}]`）——信息在 `richTextStyle` 里，node 级 `fontSize` 只是标量；schema 逐段给 fontSize及 fontWeight。无 `segments` 的普通文本走单一 fontSize。**处理规则见「样式还原」**。
- `src.imageHash`：图片占位（仅 image 节点，真 URL 由 export_image 换取）
- `renderer`：单一渲染判定轴——`image` = 当图渲染（真图 / 装饰实例已定切图 / **异形装饰形状**，直接 `export_image`、免目视）| `image-candidate` = **规整外壳（矩形/椭圆）但内容待目视**（走 §① 判切图 vs CSS/inline-SVG）；缺省 = 普通元素（照抄 style）。形状复杂度已由 normalize 机判并折进本轴：异形直接落 `image`，只有规整外壳才留 `image-candidate`
- `componentId`：该节点是**组件实例**（图标/badge/按钮/卡片等），relay-schema 已把它规范化成普通 schema（children 就位）——**照 schema 渲染、别回母版/重拉、别把整块当一张图切**；供下游改写判定是否复用项目组件库
- `sameLevel`：同一 children 数组内的绘制层级，值越大显示层级越高
- `customFont`（仅 text 节点）：`true` = normalize 判该字体在白名单外、会回退系统字走样（**脚本已判，别看 font-family 重判**）。处置按内容二选一:**接数据/动态**(商品名/价格/券文案/按钮字…)→**保持文字**(切图会冻死动态值;价格/面额尤其、配 `segments` 分段);**纯静态装饰**(楼层标题/标语等固定文案)→`export_image(nodeId, scale=<CDN倍率>)` 切图还原花字。例外:项目已 `@font-face` 内嵌该字→不回退、保持文字。描边/渐变/阴影走 CSS,仅当与花字字形不可分离才随字形切。（静态/动态判不出时 `get_screenshot` 目视,见「视觉工具节制」）
- `layoutHint`（仅容器）：`inline-badge-wrap` = 该行是「矮徽章 + 可换行长文本」（见「布局还原·例外6·A」）；`wrapped-text-split` = 该纵向列里有被设计师拆成多图层的**同一段绕排文字**，须先合并再套徽章（见「布局还原·例外6·B」）
- `wrappedTextGroup`（仅带 `layoutHint:'wrapped-text-split'` 的列）：数组，列出属于同一段的文本节点 id（首行在前、续行在后），须合并成**一个**文本渲染/绑定
- `children`：子节点数组

**视觉参数**（主 agent 注入，见 prompt「注入参数」段）：`designId`（zero-design fileKey，`export_image` 必传）+ CDN 图片导出倍率（仅用于 `export_image` 的 scale 参数，与 CSS px 值无关）。

### 布局还原

1. **布局直接照抄 `style`，禁止从 `rect` 坐标反推**——schema 的 `style` 已带 `display:flex` + 主/交叉轴对齐 + gap + padding + 换行（`flexWrap`），或 `position:absolute`（含 `left`/`top`，照抄即可、无需碰 rect）。这是 relay 图层原生 Auto Layout 的确定性翻译，比从坐标猜 flexDirection 可靠得多。**不要**看 rect 自己推断横向/纵向、对齐方式；只有 `position:absolute` 的节点按坐标定位，其余一律走文档流。
2. **文档流**：带 flex 的容器即文档流，子节点按 `children` 顺序排列，不要给文档流节点加绝对定位。
3. **层级/重叠**：子节点按 `children` 给定顺序渲染即得正确层叠（`sameLevel` 大=上，schema 已排好序）；重叠元素靠 DOM 顺序自然层叠，一般无需显式 `z-index`。
4. **背景图**：`type:block` 且 `style.backgroundImage` 存在 → 作背景处理；大面积背景图单独用 `background`，绝对定位还原其位置，**不遮挡其他图层、不破坏文档流**。
5. **固定尺寸别塌**：`style` 带 `width`/`height` 时务必写死（normalize 已排除弹性轴，出现即真固定值），别让子内容撑不满就塌成内容尺寸——带背景色的容器尤其要保留（否则背景块不撑满、露缺口）。也**别用父级 `align-items:stretch` 顶替**：stretch 只作用于交叉轴，够不到 column 的主轴高。（root 固定宽是否改 `100%` 由 rewrite 阶段决定，静态稿先照 `style`。）
6. **例外·内联徽章绕排**。根因：relay/Figma 纯 flex 把徽章和文本建成兄弟，做不出「文字绕排 inline 徽章」（首行接徽章、次行全宽绕到徽章下方，如自营标 + 商品名）——徽章反被垂直居中在多行文本块旁边。**收敛改法**：徽章作文本的**首个 `inline-block` 子元素**（`vertical-align:middle` + 右间距）+ 文本容器 `-webkit-box`+`-webkit-line-clamp` 绕排，**不做 flex 兄弟**。按文本是否被拆成多层分两种触发：
   - **A·文本单层**（`layoutHint:'inline-badge-wrap'`）：① 容器带该标记（normalize 机判：HORIZONTAL flex 内恰好 1 个 flexGrow 可换行文本 + ≥1 个矮徽章兄弟）→ 直接套收敛改法；② 无标记但目视到同类结构（矮徽章紧邻 `flexGrow` 可换行长文本、且明显矮于它）→ 同样套（启发式兜底，`style` 表达不出的漏网情形）。
   - **B·文本被拆成多层**（`layoutHint:'wrapped-text-split'`）：设计师把本该换行的**同一段文字**拆成多个文本图层（如"三星S25"接自营标后作第 1 行、"智能手机 300万像素"绕到第 2 行），normalize 已把这组 id 按序放进该列的 `wrappedTextGroup`。**须先合并再套收敛改法**：按 `wrappedTextGroup` 顺序把多段拼成**一个**文本节点，再摆徽章。跳过合并会把首段配徽章、续段另渲染，错成「标题 + 副标题」两段——它们是一段，拆开是工具产物（改写阶段也须绑**一个**字段，别拆两个，见「改写边界·合并后的绕排文字」）。
   - **⚠️ 钉徽章高**（通用）：inline-block 继承文本 `line-height`（常被放大）、strut 把徽章撑虚高；用徽章固定高（box 已从母版合并进 `style`）或设计行高钉死。`style` 表达不出，靠 visual-check「位置关系」兜底。

### 样式还原

- `style` 里**每个属性都要在 CSS 保留等价表达**，只做格式变换（camelCase → CSS 属性名、CSS Modules 类名），**不改值、不省略、不凭经验新增**。
- 颜色/圆角/阴影/边框/字体已在 schema 翻译好，直接用，**禁止猜测**。
- **`segments`（混排分段）处理——两条**（定义见「输入与上下文」字段表）：
  - **逐段属性**：按 segments 逐段渲染成多个 `<span>`，**各带自己的 `fontSize`和 `fontWeight`**，**禁止用 node 级单一值把整串压平**（价格大整数被压平、加粗段丢粗）。
  - **段间对齐**：默认基线对齐（`align-items:baseline`，行内 `vertical-align:baseline`），**禁止等高 `line-height` + `flex-end`/盒底对齐**（否则小字号段浮离基线，`¥` 飘离大数字底部）；上标/下标等设计明显不共基线的按设计对齐。
- **CSS px 值 = 设计稿 1x 原始值**（项目 px 转换插件已在前置步骤对齐基准宽度，px→vw/rpx 由构建工具自动完成）。
- **盒模型**：schema 的 `width`/`height` 是 relay 的 border-box 总尺寸（含 padding）。项目若非全局 `box-sizing:border-box`，给带 `width`+`padding` 的容器显式加 `box-sizing:border-box`，否则宽度会把 padding 多算一遍（badge/固定尺寸容器普遍胖一圈）。
- className 可从 schema 的 `name` 简单派生（无需业务语义——改写 subagent 会做语义化）。



### 图片处理

**按 `renderer` 归类**（一个节点只落一类）：

1. **无 `renderer`**（`type:block`）→ 不是图，照抄 `style`（普通节点 / 带 `componentId` 的已内联实例）。
2. **`renderer:image`** → 当图渲染 → `export_image(nodeId)` 换 URL（§③）。
3. **`renderer:image-candidate`** → 规整外壳、内容待目视 → 走 §① 目视定切图 vs CSS。

导出图尺寸统一见 §②。


#### ① image-candidate：目视定切图 vs CSS

外壳是规整的矩形/椭圆，只需定它装的**内容**用 CSS 还是切图——几何看不出、得看真实像素：内容是 CSS 能精确复现的（纯色、渐变、阴影、圆角）→ CSS/inline SVG 一比一还原；藏了 CSS 复现不了的（栅格图案、插画、多路径装饰）→ 切图。`export_image`/截图看一眼定夺，**每个至多一次**（见「视觉工具节制」；`analysis` 不产渲染判定、无上游先验）。

**易判错细则**（外壳规整、目视其内容时易错的）：
- **虚线**：`border-dashed` 段长/间隙不可控、缺口处会断 → 一律**切图**，无判断空间。
- **图形组合**（插画/装饰图/图文混排整块）：看图层数——十几个 path 切图，两三个可 inline SVG。
- **渐变 / 阴影 / 圆角** → CSS（静态稿给精确值，能一比一还原）。
- **暗黑 / 多主题元素** → 别切图（切图是死图），需随主题变色的走 CSS。
- **badge/角标**：纯椭圆/圆角矩形 → CSS 还原时**椭圆当背景、text 保持文字**；有多状态差异则每种状态各 `export_image` 一张。

#### ② 图片尺寸与预留高度

- **按 `rect`/`style` 写固定 px**（与「布局还原」一致；本身即预留了容器高度；`export_image` 按倍率返回的是 Nx 像素，CSS 仍用 `rect` 的 1x 值）。**别改成 `height:auto`**——占位图会按自身宽高比算出（常偏高的）高度、真图 load 后收缩，首屏头图/banner 尤其明显；固定高（或 `aspect-ratio`）+ `object-fit` 把占位与真图都锁进框。
- **`rect` 某维为 0 的切图**（展开实例里几何在母版未合并的装饰子，如券卡锯齿分隔）：用 `export_image` 返回的 `original_width`/`original_height`（已是 1x 自然尺寸，无需再除倍率）定尺寸，或按其在容器中的位置取合理值，**不要写死 0**（0 会让分隔图塌掉）。同理 `rect` 为 0 的**容器**走 flex hug（随内容），不写 `width:0`。
- **`width:100%` 自适应、首屏 `lazyDisable` 等依赖 Props/组件的调整不在此阶段做**，交由 rewrite。

#### ③ export_image 机制

- `type:image` 节点 → `export_image(nodeId, scale=<CDN倍率>)` 拿 CDN URL 替换占位（`nodeId` = 节点 `id`）。必传 `designId`（见「注入参数」段；未注入时取 context.md「UI 参数」段链接的 `id`，用法见 `relay-mcp.md`）。工具不可达 → 标 `/* TODO: 设计稿缺失 */`，不得用假值。

### 视觉工具节制（get_screenshot）

`export_image` 返回文本 URL（便宜）；`get_screenshot` 返回真图、吃 **vision token（文本的数量级倍数）**，是本阶段最贵的单次调用。**默认不截图**——schema 的 `style`/`renderer`/`customFont` 已给出确定性信息，够用就别看图。仅在**两处确凿的不确定**才调，且每处至多一次：

- `renderer:image-candidate`（规整外壳、内容未知）要在「纯色块/圆角矩形（CSS）」与「带图案/复杂内容（切图）」之间定夺（§①「目视定切图 vs CSS」）；
- `customFont:true` 的文字要在「静态花字（切图）」与「动态正文（保持文字）」之间定夺，且从文案性质判不出。

**禁止**：为「校准版式/色值/间距」routine 截图（这些从 schema 照抄，不靠目视）、对同一节点反复截图、整页大图截。判不准时优先信 schema 的确定性翻译，宁可标 `/* TODO: 设计稿缺失 */` 也不要靠多次截图试。

### 完成标准

- schema 所有保留节点已生成对应 DOM；布局直接来自 `style` 未从坐标反推
- 静态稿文件（JSX + 样式）已写入 `--static-file` 目录，代码完整不省略
- `renderer:image` 节点已 `export_image` 得到真 URL（或标 TODO），异形装饰形状含在内、未用 CSS 近似；`renderer:image-candidate` 判切图的同样 export_image、判 CSS 的用 CSS/inline SVG；图片尺寸按 `rect` 1x 写死（未用 export 的 2x 像素、未用 `height:auto`）
- **组件实例已还原**：带 `componentId` 的实例已按 normalize 规范化的普通 schema 照抄 style 还原（结构/颜色/`visible` 已内联），未回母版或重拉、整块未塌成一张切图
- **`style` 带 `width`/`height` 的固定尺寸容器已原样写进 CSS**（尤其带背景色的），背景块撑满未塌成内容尺寸
- 带 `width`+`padding` 的容器已按需加 `box-sizing:border-box`（项目非全局时），宽度未把 padding 多算一遍
- **矮徽章 + 可换行长文本**（如自营标 + 商品名）：`layoutHint:'inline-badge-wrap'` 的容器（及目视识别到的同类）已按「内联徽章绕排」还原（徽章作文本容器首个 `inline-block` 子元素），未做成 flex 兄弟并排
- 样式属性值均来自 schema 或二次拉取，无估算或硬编码猜测值
- 带 `segments` 的文本已逐段渲染成多 `<span>`（各带 fontSize 及 schema 提供的 fontWeight），未用单一值压平；段间**默认基线对齐**，未用等高 `line-height` + `flex-end` 致小字号段浮离基线
- **`customFont:true` 的文字已分流处理**：静态花字/艺术字标题已 `export_image` 切图；数据绑定/动态正文保持文字（未切图冻死动态值）
- **`layoutHint:'wrapped-text-split'` 的列已合并处理**：`wrappedTextGroup` 的多个文本已合并成一个文本渲染（line-clamp 绕排、徽章 inline 首元素），未当「标题+副标题」拆成两段

---

## 组件 subagent 改写规则

<!-- 由 build-ui-component-prompt.py 自动提取注入组件 subagent prompt -->

以下规则由脚本注入到组件 subagent 的 prompt 中，subagent 必须遵守：

### 任务定位

你的任务是将**静态稿代码**（由静态稿 subagent 从 relay 图层 schema 生成）改写为**功能完整的业务组件**。

静态稿特征：
- 纯视觉还原，无语义标签、无 Props、无数据绑定、无交互逻辑
- 技术栈已匹配目标项目
- 样式值来自设计稿，可以信任（颜色/字号/间距/圆角等直接保留）
- 布局已是文档流（flex，来自 relay 原生 Auto Layout），不是画布绝对定位
- 图片结构（单张 img / 多段拼接 / CSS background）已正确还原设计稿，且图片 URL 已由静态稿阶段 `export_image` 换成正式 CDN 地址，**结构和 URL 都不需要改**
- 标签缺乏业务语义，className 为从图层名派生的非业务命名

### 输入与上下文

**静态稿文件**（由主 agent 通过 prompt 注入路径）：
- 先 Read 该文件获取静态稿代码作为改写基线
- 改写通过 Edit 原地修改该文件（不新建文件）

**视觉参数**（由主 agent 通过 prompt 注入，subagent 直接使用，不自行读取 context.md）：
- CDN 图片导出倍率（仅用于 `export_image` 的 scale 参数，与 CSS px 值无关）

**骨架文件**（由主 agent 通过 prompt 注入路径）：
- Read 该文件，搜索 `<ComponentName />` 定位本组件在骨架中的位置
- 读取包裹本组件的父容器的样式声明，编写组件时需考虑父容器样式对当前组件根元素的影响，避免与之冲突或冗余
- 骨架文件中找不到对应组件引用时 → 按静态稿中该节点自身属性值设置

**业务逻辑**（通过 TRD 引用获取）：任务「实现上下文」中的 Props / 数据绑定 / 业务规则字段多以 `{TRD§x.x}` 形式引用（如 `Props={TRD§2.3 表}`），指向 prompt「设计文档（TRD）」段给出的 design.md 路径的对应章节：
- 遇到此类引用，**必须先 Read 该 design.md 的对应章节**拿到真实内容（Props 字段清单、数据绑定字段路径、状态视图矩阵等），再据此实现
- 禁止凭引用字面或经验**假设** Props 字段、绑定路径、状态分支
- `事件={onPay→handlePay}` 这类已内联写全的字段直接使用，无需再查 TRD

### 改写职责（按顺序执行）

1. **语义化改写** — 静态稿代码缺乏业务语义，须做以下语义化：
   - **HTML 标签语义化**：根据元素实际用途将 div 替换为合适的语义标签（如 header/section/nav/ul/li/button/img/span 等）
   - **像素中性——语义标签的 UA 默认样式/行为必须重置**：静态稿全是无 UA 默认的 `div`，换成语义标签后浏览器 UA 默认会介入、悄悄偏离静态稿视觉/触发意外行为，须在组件内显式抵消（项目有全局 reset 则已覆盖、可省；拿不准就显式重置最稳，重置与静态稿同值幂等无害）：
     - `<button>`：`border:0; background:none; appearance:none; cursor:pointer; font:inherit`（否则冒 outset 边框/灰底/系统字）；默认 `type="submit"` → **显式 `type="button"`**；按钮嵌在可点父级里 → 事件加 `stopPropagation` 防双触发
     - `<p>`：`margin:0`（消 UA 上下 ~16px margin）；`<h1>~<h6>`：`margin:0` + 显式 `font-size/weight`
     - `<ul>/<ol>/<li>`：`list-style:none; margin:0; padding:0`
     - `<a>`：重置 `color`/`text-decoration`（消蓝字下划线）；`<input>/<textarea>`：`appearance/border/outline/font` 全 reset
   - **className 语义化**：将静态稿从图层名派生的非业务命名替换为有业务含义的命名（如 .coupon-card、.price-tag、.status-badge），命名参考 TRD §2.0 组件树 / 对照表（`delivery/<task>/schema/component-table.md`）中的组件/元素名称
   - **组件/变量命名**：拆分出的子片段、循环变量、条件分支变量等使用业务语义命名
2. **Props 定义** — 根据 TRD 组件 Props 章节定义组件 Props interface
3. **数据绑定** — 将静态文案/图片 URL 替换为 Props 字段（参照 TRD 数据绑定表）
4. **条件渲染** — 根据 TRD §4.x 状态视图矩阵实现条件分支
5. **交互逻辑** — 添加事件处理、状态管理（参照 TRD 业务规则 + task 事件列表）
6. **埋点接入** — 根据 TRD §7 添加埋点上报
7. **多状态差异图片** — 仅当 task 标注了变体差异 export_image 列表时，调 export_image 获取非默认状态图片并实现 src 切换
8. **children 嵌套** — 按 task 标注的 children 信息，在合适位置 import + 渲染子组件标签

### 改写边界

**保留不动的**：
- **布局结构** — 静态稿的 DOM 层级和布局方式作为基线，不重新设计
- **样式数值** — 静态稿中的颜色/字号/间距/圆角等直接保留（迁移到语义化 className 下）
- **图片结构与 URL** — 静态稿的图片 DOM 结构（单张 img、多段拼接、CSS background）是正确的视觉还原，且 URL 已是正式 CDN 地址，均不得重建/重导；prompt 里「保留静态稿的图片渲染」一节只是禁止把图片改成 CSS/文本，**不是**重建结构的指令
- **多字号文本的分段结构** — 静态稿里价格/面额等常是多个不同字号的 `<span>`（「¥」小 + 整数大 + 小数小）。数据绑定时**按 span 分段填值、保留每段字号/字重**，**禁止把整块塌成一个绑定值**（如 `<span 大>3588</span><span 小>.15</span>` → `¥{price}`），否则字号层级压平、价格排版全错。formatter 只返回整串（如 `"¥80"`）时，仍须在组件内按设计字号分段渲染，或让 formatter 返回结构化片段（symbol/integer/decimal）。
- **合并后的绕排文字（`wrapped-text-split`）** — 静态稿已把被拆图层的绕排文字合并成一个文本元素（如整段 `skuName`）。数据绑定时对应**一个字段**（`skuName` 整段），**禁止拆回两个字段**（如把第 2 行误绑成 `sellingPoints`/副标题）——它本就是一段。
- **递归展开的实例（`componentId` + `type:block`，已递归展开的真实结构）与动态列表** — 静态稿里这类实例已是**递归展开的真实结构**（非占位图）。改写时**按其文本/内容子节点绑数据**、**保留装饰切图与上下布局、禁止删整块重写**。若是列表（设计铺了多份同构样例 / 数据是数组）→ 以**一份**为模板 `map` 数据数组，别保留样例份数、别重画骨架。

**例外——静态稿固定尺寸产物需修正**：
- **组件根元素固定宽度**：静态稿根节点的 `width: Npx`（来自图层固定尺寸）替换为 `width: 100%`；**例外**：若组件置于横向 flex 容器（卡片列表、券卡组等横排场景）内，保留固定宽度（`flex-shrink:0` 已由 normalize 产出、照常保留）。
- **动态内容容器的固定高度**：TRD 数据绑定表中该容器绑定的是列表字段（数组）时，其 `height: Npx` 是快照尺寸，必须删除或改为 `min-height`；若同时有 `overflow: hidden`，纵向动态容器一并删除（防止内容被裁断），横滑容器（`overflow-x: auto/scroll`）保留。**例外**：设计定义了有界滚动区（TRD §6「列表溢出」行 / 设计滚动区帧）时不是无脑展开，而是按其值设 `max-height`+`overflow-y:auto` 保留有界滚动（与骨架侧「列表容器 overflow」同一原则）。
- **自适应图的宽度与懒加载**：静态稿按 `rect` 写死固定 px，其中自适应图（背景/banner/卡片主图）的 `width` 改为随容器（`100%` 等）、保留固定高或 `aspect-ratio` + `object-fit` 防懒加载跳变；首屏关键图加 `lazyDisable` 直接加载真图、免占位切换。固定图（商品图/头像/标签图）维持固定尺寸不动。
- **文字截断行数**：静态稿的文字节点若带 `white-space: nowrap` / `text-overflow: ellipsis`（单行截断），必须先核对设计稿实际行数：设计稿为多行则改为 `-webkit-line-clamp: N`（N 由设计稿行数或 TRD 业务规则决定），**禁止默认保留单行截断**。判断依据：看该文字节点在设计稿中是否跨越多行——若 nodeId 对应帧的文字高度 > 单行行高，即为多行。

### 图片处理

静态稿阶段已对所有图片调用 `export_image` 换成正式 CDN URL，改写时**直接保留，无需重新导出**。

唯一例外——**变体差异图片**：仅当 task 标注了变体差异 `export_image` 列表时（非默认状态的 badge / 背景等），按改写职责第 7 步调 `export_image(nodeId, scale=<CDN倍率>)` 获取并实现 src 切换。

> `export_image` 必传 `designId`（= context.md「UI 参数」段设计稿链接的 `id`/fileKey，整个任务同一值；用法见 `relay-mcp.md`）。

总结：subagent 的改写是"加功能"而非"改结构"——只做 (1)非语义→语义化 (2)静态文案→动态字段 (3)硬编码→Props (4)无交互→有交互 (5)单状态→多状态条件分支 (6)无埋点→有埋点 (7)画布约束值→自适应值（根元素宽度、图片宽度、动态容器高度）。不做布局重构、样式微调、图片结构重建。

### 工具调用约束

- tasks.md 中标注的每个 `[required-tool] export_image` 条目（变体差异图片）都必须逐个执行，不得合并跳过
- 工具不可达时：标注 `/* TODO: 设计稿缺失 */`，不得用假值替代

### 完成标准

- 静态稿文件已通过 Edit 原地改写为功能完整的业务组件
- 语义化完成：无意义的 div 已替换为语义标签，无意义的 className 已替换为业务命名
- 组件 Props 类型与 TRD 定义一致，数据绑定字段路径与 TRD §2.4 匹配
- **条件渲染 / 交互逻辑 / 埋点 / children 嵌套均已按 TRD 实现**：状态视图矩阵(§4.x)的分支、事件与状态、埋点上报(§7)、子组件 import+渲染，逐项落地未遗漏
- **语义标签 UA 默认已重置（像素中性）**：引入的 `<button>`/`<p>`/`<ul>`/`<a>`/`<input>` 等已按语义化改写清单重置 UA 默认样式（button 边框/灰底/appearance、p/h/ul margin、a 蓝字下划线等）与行为（button `type="button"`、嵌套可点 `stopPropagation`），未让 UA 默认偏离静态稿
- **画布约束值已改自适应值**（改写边界·固定尺寸修正）：根元素固定宽已改 `100%`（横排场景保留固定宽）；绑列表字段的动态容器固定高已删除/改 `min-height`（纵向连带删 `overflow:hidden`）；自适应图宽度随容器 + 固定高/`aspect-ratio` 防懒加载跳变；多行文字的单行截断已改 `-webkit-line-clamp:N`。（`flex-shrink:0` 由 normalize 产出、照常保留，不在此清单）
- 从项目已有组件库 import 的组件，Props 传值以 .d.ts / interface 定义为准
- task 标注的 export_image 全部实际执行并获得返回值
- 任一改写职责未完成、export_image 未执行或类型不匹配，不得报告"已完成"
