# workflow-test-cases

本节点**只负责本次特性的测试用例**（`.workflow/scripts/test/tasks/<task>/`）。核心契约：oracle 只来自 test-plan、authoring 隔离、撞洞踢回上游。

## 两段流水线(核心)

生成分**两段连续执行**,无人审阻塞。CLI 通过 `--phase` 参数区分,不写默认全量执行（段 1 + 段 2 连跑）。

> **设计原则**：所有需要人决策的事项（oracle、mock 策略、物料、PRD 规则冲突）已在 test-plan 闭环。本节点只做技术准备 + 机械翻译，不产生新的人审阻塞点。遇到无法自行解决的 oracle 缺口，走 `issue-add` 回流 test-plan，不在本节点等待。

### 段 1:technical-prep

前置输入: TRD design.md 第 2/3 章 nodeId 映射表、PRD、`test-plan.md`(含**维度标签** + **Mock 策略决策表** + **PRD 规则冲突扫描结果**,test-plan 已锁死,本段不重决策)。静态样式断言值从设计稿按 nodeId 拉取（设计变量值 = 断言值）。

产 **2** 份技术准备产物到 `delivery/<task>/test/l3-intermediate/`:

| 产物 | 内容 | 规则 |
|---|---|---|
| `states.md` | 动态样式态清单 | 只列设计稿中有效帧对应的态,不猜 |
| `mock-variants.md` | Mock 响应变体（仅对 api 维度的 plan-N） | 依 TRD 接口定义 + test-plan oracle 推正常/空/异常等变体；同时含 ui 维度时**必须显式标注一条 `[UI-driver]` variant**——保证 UI 元素能挂载出来供 CSS 断言用 |

段 1 完成后**直接进入段 2**，不挂 pending、不等人审。

### 跨维度桥接：api + ui 共存时的 UI-driver 约束

当 plan-N 同时标 `api` + `ui` 两个维度时，以下约束贯穿段 1 → 段 2：

- **段 1**：`mock-variants.md` 必须为该 plan-N 标注一条 `[UI-driver]` variant（一个能让 UI 元素正常挂载的 mock 响应）
- **段 2**：ui spec 的第一步必须 `page.route()` 挂这条 `[UI-driver]` variant，再执行 CSS 断言
- **遗漏后果**：元素不挂载 → `toHaveCSS` 因定位失败静默 skip → 假绿

段 1 产物 checklist 自查：凡 test-plan 中 `api ∩ ui ≠ ∅` 的 plan-N，`mock-variants.md` 必须有对应 `[UI-driver]` 行，缺了不出段 1。

### 段 2:translate-to-cases

前置输入:段 1 两份技术准备产物 + test-plan.md（oracle + **Mock 策略决策表** + **PRD 规则冲突扫描结果**）。

翻译规则:

- **静态样式用例** → 从设计稿变量值直翻断言,无阻塞
- **PRD 业务规则** → 从 test-plan.md oracle 直翻（冲突已在 test-plan 阶段裁决完毕）
- **states 里的态** → 落对应快照用例

**分维度模板**（每个 plan-N 按维度选，可组合）：

| 维度 | 模板要点 |
|---|---|
| `api` | `page.route()` 内联 mock-variants 中的 variant → 断业务结果（不是 200） |
| `ui` | `toBeVisible()` gate → `toHaveCSS` 主判（值来自设计稿变量） → `testInfo.attach()` 附实际截图（留痕供人 review）。若同时含 api 维度，见§跨维度桥接 |
| `tracking` | 按 test-plan「埋点校验策略」分叉——`local-intercept` 就在 spec 内加 `page.on('request')` 断言 URL + 参数；`easytrack-verify` 就生成 `.workflow/scripts/test/tasks/<task>/easytrack-triggers.ts`；双开就两个都出；`skip` 不生成 |
| `logic` | 标准 E2E（操作序列 → 业务结果断言），沿用现有模板 |

ui 用例样例：
```typescript
test("plan-3: 弹窗样式", async ({ page }, testInfo) => {
  await page.goto("/order");
  await page.click("[data-testid='open-dialog']");
  const dialog = page.locator("[data-testid='dialog']");
  await expect(dialog).toBeVisible();  // gate：区分"缺失" vs "样式错"
  await expect(dialog).toHaveCSS("background-color", "rgb(255, 255, 255)");
  await expect(dialog).toHaveCSS("border-radius", "12px");
  await testInfo.attach("actual", { body: await dialog.screenshot(), contentType: "image/png" });
});
```

tracking 用例样例（`local-intercept` 策略）：
```typescript
test("plan-4: 列表曝光埋点", async ({ page }) => {
  const trackingReqs: URL[] = [];
  page.on('request', req => {
    const url = req.url();
    if (url.includes('/log.gif') || url.includes('tunnel-ihub')) trackingReqs.push(new URL(url));
  });
  await page.goto("/order");
  // 等待埋点请求实际发出，而非硬等待
  await expect.poll(() =>
    trackingReqs.find(u => u.searchParams.get('event_id') === 'orderList.exposure')
  , { timeout: 5000, message: '预期曝光埋点已上报' }).toBeTruthy();
  const expo = trackingReqs.find(u => u.searchParams.get('event_id') === 'orderList.exposure')!;
  expect(expo.searchParams.get('spm')).toBe('orderList.exposure.default');
  expect(expo.searchParams.get('page')).toBe('order');
});
```
`easytrack-verify` 策略不写 spec 内断言,只生成 `easytrack-triggers.ts` 由 run-autotest 消费;双开策略两个产物都出。

`easytrack-triggers.ts` 结构契约：

```typescript
// .workflow/scripts/test/tasks/<task>/easytrack-triggers.ts
import type { EasytrackTrigger } from '@/test-utils/easytrack';

export const triggers: EasytrackTrigger[] = [
  {
    planId: 'plan-4',
    eventId: 'orderList.exposure',
    spm: 'orderList.exposure.default',
    requiredParams: { page: 'order' },        // 必须精确匹配的参数 KV
    optionalParams: ['timestamp'],             // 只校验存在性
    description: '列表曝光埋点（新增点位）',
  },
];
```

**Mock 物料分发**（按 test-plan 决策表逐条执行）：

主策略（覆盖 95%+ 场景，段 2 直接执行，不阻塞出口）：

| 策略 | 段 2 动作 | 产物 |
|---|---|---|
| `local-route`（**默认**） | 内联 `page.route()` 到 spec | `test_<area>.spec.ts` |
| `real` | 无动作 | — |

进阶策略（仅当 test-plan 决策表显式选用时才走，执行链路长、有外部依赖）：

| 策略 | 段 2 动作 | 产物 | 适用场景 |
|---|---|---|---|
| `local-mock` | 改项目 mock 文件 + 必要时补 whistle/proxy 规则 | `src/mock.js` 等 + `whistle-rules.txt`（如需） | 项目已有 mock 基础设施且 local-route 不适用 |
| `easymock-http` | `lbcli easymock http --action template-add` | materials.md 记 template-id | 后端状态变更需真实链路 |
| `easymock-jsf` | `lbcli easymock jsf --action template-add` | 同上 | JSF RPC 场景 |
| `easymock-color` | `lbcli easymock color --action template-add` | 同上 | Color 网关场景 |

进阶策略的出口处理：
- EasyMock 上传在段 2 翻译完成后、出口前统一执行
- **重试 1 次**后仍失败 → 自动降级为 `local-route`（改写对应 spec 为 `page.route()` 内联，标注 `// degraded from easymock`），降级后正常出口
- `local-mock` 改写失败 → 同上降级为 `local-route`
- **不允许进阶策略未就绪就直接 advance**——run-autotest 无 mock 必红，浪费一轮部署

段 2 出口:`test_<area>.{spec|cy}.ts` + `test-cases.md` 索引(继承后端契约) + `easytrack-triggers.ts`（有 tracking 维度且选了 `easytrack-verify` 或双开策略时）→ `advance <task> --step workflow-test-cases`。

## 边界与纪律

- **设计稿变量值是静态样式的唯一 oracle**。变量值 = 断言值,不问 LLM 推断。
- **不做决策、不等人审**。oracle / 策略 / 物料 / 规则冲突全部在 test-plan 闭环。遇 oracle 缺口唯一出路：`issue-add` 回流 test-plan。
- **设计没画的态不测**。业务不要求 = 不测,不是"我猜可能漏"。
- 不本地跑测试（执行在 run-autotest,须先部署）
- 不越权改代码焊绿（见 PROTOCOL 红线）

## 撞到 oracle 缺口

沿用后端节点契约,段 1/段 2 都可触发 `issue-add --from workflow-test-cases`。典型 issue:

- **oracle 模糊/缺失**（翻译时发现 test-plan 某 plan-N 的预期映射不够明确） → issue-add,verdict 回流 test-plan 补 oracle。
- **多态稿设计缺失但业务需要多态测试** → issue-add 到设计,verdict 回流设计师补稿后重新拉取。
- **mock 策略不可行**（如发现接口有 cookie 依赖无法 local-route） → issue-add 回流 test-plan 修正策略。

**本节点不等待 issue 裁决**——issue-add 后跳过该 plan-N 继续翻译其余用例，出口标记哪些 plan-N 因 oracle 缺口被 skip。issue resolve 后重跑本节点补齐。

### 补跑协议（issue resolve 后重入）

issue 在不同时间 resolve 后会触发补跑。为避免多次部分产出拼装导致碎片化，补跑遵循以下 merge 协议：

1. **读现有产物再动手**：补跑前先 Read 已产出的 `_helpers.ts` 和 `test-cases.md`，沿用已有 helper 函数、mock builder、编号体系
2. **编号续编不插入**：新用例编号从现有 `test-cases.md` 最大 TC-XX + 1 续编，不回填中间空位
3. **helper 只追加不改签名**：若需新增 helper 函数，追加到 `_helpers.ts` 末尾；不改已有函数的签名或行为（避免影响已产出 spec）
4. **更新 pending manifest**：出口的 `l3-intermediate/pending-plans.json` 记录当前仍被 skip 的 plan-N 及对应 issue-id；补跑完成后移除已补条目
5. **全部补齐后清理**：所有 plan-N 补齐后删除 `pending-plans.json`，正常 advance

`pending-plans.json` 结构：
```json
{
  "skippedPlans": [
    { "planId": "plan-3", "issueId": "issue-7", "reason": "oracle缺失: 弹窗关闭后状态不确定", "phase": "段2" }
  ]
}
```

## 产物索引

```
delivery/<task>/test/l3-intermediate/states.md          # 段 1 产
delivery/<task>/test/l3-intermediate/mock-variants.md   # 段 1 产
delivery/<task>/test/l3-intermediate/pending-plans.json # 有 skip 时产，全补齐后删除
.workflow/scripts/test/tasks/<task>/test_<area>.{spec|cy}.ts # 段 2 产
delivery/<task>/test/test-cases.md                      # 段 2 产,三层索引合并
delivery/<task>/test/materials.md                       # test-plan 创建，段 2 追加（EasyMock template-id 等）
```
