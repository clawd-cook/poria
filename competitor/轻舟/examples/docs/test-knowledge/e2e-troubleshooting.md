# E2E 失败诊断流程

用例失败时按以下流程排查，从最常见原因开始逐步收窄。

## 诊断决策树

```
用例失败
├── 1. "intercepts pointer events" 报错
│   ├── 含 <iframe> → 代理 iframe 遮挡（whistle/Charles/Fiddler 注入）
│   │   └── 修复：确认 installProxyIframeKiller 已调用，或切 test 模式不走代理
│   └── 含业务组件类名 → UI 层叠遮挡（引导蒙层/弹窗/banner）
│       ├── z-index 更高的蒙层 → suppressFrequencyControl 抑制对应 key
│       └── 同级 clone 遮挡 → 用 dispatchEvent("click") 绕过 actionability check
│
├── 2. 组件不渲染 / 状态不更新（无显式报错）
│   ├── 检查 console error（CORS 报错往往只在 console）
│   │   └── "...has been blocked by CORS" → mock headers 问题
│   │       └── credentials:include 场景必须用 API_RESPONSE_HEADERS_CREDENTIALED
│   ├── 检查项目请求层特有的校验头（如有）
│   │   └── 请求层可能要求特定响应头字段才把 data 交给业务层
│   └── 检查 mock 数据字段完整性
│       └── 业务组件依赖某字段做条件渲染，mock 数据没给 → 渲染分支不命中
│
├── 3. click 后无后续反应
│   ├── 检查是否漏了中间 API mock
│   │   └── 操作触发了一个请求，但没注册 route → 打到真实服务器被 CORS 拦截
│   ├── 检查 route.fetch 是否卡住
│   │   └── 目标 API 已下线/返回异常 → 改用 route.fulfill
│   └── 检查事件是否真正触发了
│       └── 层叠遮挡导致事件打到错误元素 → 回到问题 1
│
├── 4. 断言值不符
│   ├── 字段类型不匹配：源码用 === 严格比较但 mock 给了不同类型（如 number vs string）
│   ├── 数值精度：源码做了格式化（toFixed/round）但 mock 给了原始值
│   └── 枚举值拼写：大小写/下划线/连字符差异
│
└── 5. 超时 / 白屏 / 页面崩溃
    ├── 框架 Runtime Error → 某个 API 返回了代码无法处理的数据结构
    ├── route.fetch 挂起 → 真实服务器无响应或返回后触发请求层重试死循环
    └── 页面 JS 加载失败 → 检查 CDN 可达性 / baseURL 是否正确
```

## 常用诊断命令

### 查看 Console 错误（CORS 问题最有效）

```typescript
page.on("console", msg => {
  if (msg.type() === "error") console.log("[page error]", msg.text());
});
```

### 查看网络请求状态

```typescript
page.on("response", res => {
  if (res.status() >= 400) {
    console.log(`[${res.status()}] ${res.url()}`);
  }
});
```

### 截图定位视觉状态

```typescript
await page.screenshot({ path: "debug.png", fullPage: true });
```

### 检查元素层叠关系

```typescript
const element = page.locator("[data-testid='target']");
const box = await element.boundingBox();
// 在该坐标执行 elementFromPoint 看谁在最上面
const topElement = await page.evaluate(
  ([x, y]) => {
    const el = document.elementFromPoint(x, y);
    return el ? { tag: el.tagName, class: el.className } : null;
  },
  [box!.x + box!.width / 2, box!.y + box!.height / 2]
);
console.log("topElement:", topElement);
```

## route.fetch vs route.fulfill 决策表

| 场景 | 选择 | 原因 |
|---|---|---|
| 接口正常在线，只改一两个字段 | `route.fetch` + 改 body | 保留真实结构，减少 mock 维护 |
| 接口已下线 / 活动已结束 | `route.fulfill` | fetch 会卡住或返回异常触发重试 |
| 需要完全控制响应 | `route.fulfill` | 确定性最强 |
| 不确定接口状态 | `route.fulfill` | 安全兜底 |

## 频控/蒙层遮挡快速定位

当 click 被拦截，Playwright 报错信息会包含遮挡元素的 selector。常见遮挡模式：

| 遮挡类型 | 典型特征 | 处置 |
|---|---|---|
| 代理工具注入 iframe | `z-index: 2147483647`，`position: fixed`，全屏 | `installProxyIframeKiller` / 切 test 模式 |
| 引导蒙层 | 高 z-index（通常 9000+），`position: fixed/absolute` | `suppressFrequencyControl` 抑制对应 localStorage key |
| 浮动 banner | 高 z-index，覆盖目标区域 | `suppressFrequencyControl` 或等动画结束 |
| 同级 clone 节点 | 同层级相同位置的复制节点 | `dispatchEvent("click")` 绕过 Playwright actionability |

**诊断步骤**：
1. 从报错信息提取遮挡元素类名/tag
2. 用「检查元素层叠关系」代码片段确认坐标处的顶层元素
3. 判断遮挡源类型 → 选择对应处置方案
