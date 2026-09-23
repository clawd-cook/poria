
## 规约

### REQ-T01 E2E 用例必须使用 shared/ 公共基建

- **内容**：`.workflow/scripts/test/tasks/<task>/` 下的用例和 helpers 禁止重复实现 `.workflow/scripts/test/shared/` 已提供的能力，必须 import 使用。当前 shared 导出清单：
  - `installProxyIframeKiller` / `removeProxyIframeIfNeeded` — 代理 iframe 防护
  - `API_RESPONSE_HEADERS` / `API_RESPONSE_HEADERS_CREDENTIALED` — mock 响应 headers 模板
  - `deepMerge` — payload 深合并工具
  - `suppressFrequencyControl` / `clearFrequencyControl` — 频控 key 预设/清除
- **禁止**：
  - 在 tasks/ 下自定义 `deepMerge`、重复 iframe 移除逻辑、内联 `access-control-allow-origin` 字符串
  - spec 文件中直接操作频控 localStorage（应用 shared helper）
- **检查方式**：code-review 人工把关（可配 lint 脚本机器检查）

### REQ-T02 E2E route mock 分层策略

- **内容**：按请求角色分三层，agent 自行决策，人可在 review 时批注覆盖。
  - **入口 API**（init 类，建立页面初始状态，只读）：默认 `route.fetch` + 局部改字段；接口探活失败则降级为 `fulfill`
  - **动作 API**（submit/check/create 等，由交互触发，有写操作或依赖前序状态）：一律 `route.fulfill`
  - **已下线 API**（服务端已不响应）：一律 `route.fulfill`
- **为什么动作 API 必须 fulfill**：
  - 依赖前序状态——前序若是 mock 的，真实服务器状态不对应
  - 可能有副作用（创建订单/扣库存），测试环境不该真正执行
  - 返回结果决定 UI 走向，必须完全可控
- **决策权**：agent 按分层规则自行决策；如需推翻默认策略，人可在 test-cases review 时批注覆盖

## 经验

> 阶段性启发（这阶段怎么做更好），还没到强制；信心驱动可升级为规约。只收规定性启发，不收领域事实（那进 module-/tech-knowledge）。

### EXP-T01 E2E route mock CORS headers 两套策略

- **主题**：`page.route` + `route.fulfill` 的 headers 配置
- **根因轴**：test-cases 阶段对 CORS 规范理解不足
- **拦截轴**：test-cases 编写时
- **严重度**：normal
- **内容**：`route.fulfill()` 的 mock 响应必须区分两种 CORS 策略：
  - **需 app 消费的 API**：`access-control-allow-origin` 必须填具体 origin（从 `TEST_BASE_URL` 取），因为 `credentials: include` 场景下，浏览器 CORS 规范禁止 `*` 与 `include` 共存
  - **不需 app 消费的 API**（如已下线的接口）：保持 `*`，让浏览器 CORS 拦截，app 走 catch 分支不 crash
- **如何落地**：使用 `shared/mock-headers.ts` 的两套 headers 常量（`API_RESPONSE_HEADERS` + `API_RESPONSE_HEADERS_CREDENTIALED`），按场景选用；如项目请求层有额外校验头，在项目 `_helpers.ts` 中扩展

### EXP-T02 交互用例 beforeEach 必须处理频控遮挡

- **主题**：引导蒙层 / banner 遮挡导致 click 失败
- **根因轴**：test-cases 阶段遗漏了 UI 叠层关系
- **拦截轴**：test-cases 编写时
- **严重度**：normal
- **内容**：（适用于基于 localStorage 的频控/引导机制）任何需要 click 交互的用例，在 `beforeEach` 中必须用 `suppressFrequencyControl` 预设所有可能遮挡目标元素的频控 key。原因：引导蒙层 z-index 高于楼层内容，Playwright 的 actionability check 会报 `intercepts pointer events`
- **如何落地**：检查测试目标区域上方是否有引导/蒙层组件，按 localStorage key 抑制

### EXP-T03 mock 交互流全链路 API

- **主题**：只 mock init 不 mock 中间步骤导致提交流程走不通
- **根因轴**：test-cases 阶段未梳理完整请求链路
- **拦截轴**：test-cases 编写时
- **严重度**：normal
- **内容**：E2E 如果要验证一个完整交互流（如「选择 → 确认 → 提交 → 看结果」），必须 mock 流程中**所有** API 调用，而不只是 init。遗漏中间 API 会导致请求打到真实服务器被 CORS 拦截或返回异常，后续 UI 状态不变
- **如何落地**：写用例前从入口操作到断言点画出完整请求链路图，每条 API 注册一个 mock route

## 踩坑记录

### PIT-T01 whistle 代理注入全屏透明 iframe

- **主题**：所有 click 报 `<iframe> intercepts pointer events`
- **根因轴**：测试基建层，环境依赖未文档化
- **拦截轴**：run-autotest 执行时
- **严重度**：normal（首次遇到耗时 ~2h 定位）
- **现象**：经 whistle 代理的 Playwright 测试，所有 `click()` 动作超时失败，报 `<iframe></iframe> intercepts pointer events`
- **根因**：whistle 在页面中注入一个 `position:fixed; top:0; left:0; width:100%; height:100%; z-index:2147483647` 的透明 iframe 做请求捕获/界面覆盖，拦截了所有指针事件
- **修复方式**：使用 `shared/proxy-iframe-killer.ts` 的 `installProxyIframeKiller` 在 page.goto 前注入 MutationObserver 持续移除该 iframe
- **教训**：代理工具（whistle/Charles/Fiddler）可能往页面注入不可见 UI 元素，E2E 首次接入代理后应跑一个最简 click 冒烟确认 pointer events 通路

### PIT-T02 CORS wildcard + credentials:include 静默失败

- **主题**：mock 响应被浏览器静默拦截，app 表现为「组件不渲染」而非报错
- **根因轴**：CORS 规范理解偏差
- **拦截轴**：test-cases / run-autotest
- **严重度**：normal（诊断难度高——无显式报错，只有 console error）
- **现象**：`route.fulfill()` 返回了正确 JSON，但 app 行为与「接口失败」一致；组件不渲染、状态不更新
- **根因**：请求带 `credentials: 'include'` 时，CORS 规范要求响应的 `Access-Control-Allow-Origin` **不允许为 `*`**，必须是具体的请求源 origin。设了 `*` → 浏览器拒绝交给 JS 消费 → 进入 catch → 无报错无 UI
- **诊断方法**：`page.on('console', msg => { if (msg.type() === 'error') ... })` 能看到 CORS error；但默认不打印到 stdout
- **教训**：涉及 `credentials: include` 的 mock 响应，`access-control-allow-origin` 必须设为具体 origin（用 `API_RESPONSE_HEADERS_CREDENTIALED`）

### PIT-T03 CSS Module 选择器匹配过宽

- **主题**：`[class*="ComponentName"]` 匹配到组件内部所有子类名
- **根因轴**：CSS Module hash 命名规则理解偏差
- **拦截轴**：test-cases 编写时
- **严重度**：low
- **现象**：`page.locator('[class*="MyComponent"]')` 解析为多个元素，Playwright strict mode 报错
- **根因**：CSS Module 生成的类名格式为 `ComponentFolder_propertyName__hash`。`[class*="MyComponent"]` 会匹配 `MyComponent_root__xxx`、`MyComponent_title__xxx`、`MyComponent_btn__xxx` 等所有子元素
- **修复方式**：始终用更精确的选择器如 `[class*="MyComponent_root"]`，或加 `.first()` / 特定子属性名
- **教训**：CSS Module 项目中的 E2E 选择器必须带属性名后缀精确到具体元素

### PIT-T04 `route.fetch()` 对已下线 API 挂起

- **主题**：对已停用 API 用 `route.fetch()` 导致 Playwright 路由 pipeline 阻塞
- **根因轴**：真实 API 状态与 mock 策略不匹配
- **拦截轴**：run-autotest 执行时
- **严重度**：normal
- **现象**：使用 `route.fetch()` + 修改 body 模式 mock 某接口时，页面整体白屏/卡死
- **根因**：真实 API 已停用/返回异常状态，请求层可能触发重试/死锁，Playwright 的 route 回调一直等不到 `route.fulfill()`
- **修复方式**：已下线/会返回异常的 API 必须用 `route.fulfill()`（直接返回 mock 数据），不能用 `route.fetch()`（转发到真实服务器再改）
- **决策原则**：`route.fetch()` 模式仅用于真实服务器能正常返回的 API；已过期/会报错的 API 一律 `route.fulfill()`
