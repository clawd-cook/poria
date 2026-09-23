# .workflow/scripts/test/ — Playwright E2E 功能基线测试

功能基线端到端回归测试，由 workflow-run-autotest 节点（`.workflow/node/workflow-run-autotest/NODE.md`）通过 Playwright 执行。
被测服务由人工提前部署到测试环境，本目录只负责模拟用户操作验证核心路径。

## 两种测试模式

| 维度 | test 模式 | local 模式 |
|---|---|---|
| 被测环境 | 已部署的测试环境 | 本地 dev server（通过 whistle 代理） |
| TEST_BASE_URL | 测试环境 URL | 测试环境 URL（whistle 转发到 localhost） |
| TEST_PROXY | 不设置 | whistle 代理地址（如 `http://127.0.0.1:12345`） |
| 前置条件 | 测试环境可达 | whistle 运行 + 本地 dev server 运行 |
| 适用场景 | CI / 验收回归 | 开发中本地调试用例 |

## 目录组织

按任务分桶：

```
.workflow/scripts/test/
├── preflight.sh          # 前置检查+跑测入口脚本
├── playwright.config.ts  # Playwright 配置（baseURL / 浏览器 / reporter）
├── global-setup.ts       # 全局 Setup（登录态注入 → storageState）
├── shared/               # 跨任务公共工具（mock headers / 频控 / 代理 iframe 清理）
│   ├── index.ts
│   ├── proxy-iframe-killer.ts
│   ├── mock-headers.ts
│   ├── frequency-control.ts
│   └── utils.ts
├── package.json          # @playwright/test 依赖
├── README.md
└── tasks/
    └── <task-name>/
        └── *.spec.ts     # 该任务的 E2E 用例
```

> 用例文件长期保留，跨任务回归时按需选择目录跑。

## 快速开始（推荐：使用 preflight.sh）

```bash
# 交互式选择模式，自动完成所有前置检查后跑测
bash .workflow/scripts/test/preflight.sh

# 指定模式 + 任务
bash .workflow/scripts/test/preflight.sh test --task login-flow

# local 模式 + headed 调试
bash .workflow/scripts/test/preflight.sh local --task login-flow -- --headed
```

preflight.sh 会自动：
1. 选择/确认测试模式（test/local）
2. 设置 TEST_BASE_URL / TEST_PROXY 环境变量
3. 检查 Playwright chromium 安装
4. 获取登录态 Cookie（lbcli 自动导出）
5. 验证环境可达性 / whistle + dev server 状态
6. 构建并执行 playwright 命令

## 手动执行（不走 preflight）

```bash
cd .workflow/scripts/test
npm install
npx playwright install chromium

# test 模式
export TEST_BASE_URL=http://test.example.com
npx playwright test tasks/<task>

# local 模式
export TEST_BASE_URL=http://test.example.com
export TEST_PROXY=http://127.0.0.1:12345
npx playwright test tasks/<task>
```

## 环境变量

| 变量 | 必填 | 说明 |
|---|---|---|
| `TEST_BASE_URL` | 是 | 被测服务地址 |
| `TEST_PROXY` | local 模式 | whistle 代理地址（设置后 Playwright 走代理 + 启用 iframe killer） |
| `TEST_COOKIE` | 否 | 登录态 Cookie（不设则 lbcli 自动获取） |

## 认证与环境配置

用例**不直接**读环境变量或 cookie——统一由 `global-setup.ts` 注入 `storageState`。
`global-setup` 按下面优先级获取登录态：

`TEST_COOKIE` 环境变量 > `lbcli getssologin` 自动导出

**默认（推荐）：自动导出本机已登录的 SSO Cookie**，无需手动配——只要本机 Chrome 已登录内网且 Browser Bridge 连通（`lbcli doctor` 显示 `connected`）。

**覆盖（CI / 无 Browser Bridge）**：显式传入 `TEST_COOKIE` 环境变量。

## 执行产物

由 workflow-run-autotest 节点自动调用，等价于：

```bash
cd .workflow/scripts/test
PLAYWRIGHT_JUNIT_OUTPUT_FILE=../../../delivery/<task>/test/junit.xml \
npx playwright test tasks/<task>/ \
    --reporter=junit,html
```

产物：
- `delivery/<task>/test/junit.xml` — AI 解析用
- `delivery/<task>/test/test-results/` — trace / 截图 / HTML 报告，给人看

## 用例编写约定

```typescript
import { test, expect } from "@playwright/test";
import { suppressFrequencyControl, installProxyIframeKiller } from "../shared";

test.describe("功能名称", () => {
  test.beforeEach(async ({ page }) => {
    await installProxyIframeKiller(page);
    // await suppressFrequencyControl(page, ["fc_key_1", "fc_key_2"]);
  });

  test("核心路径描述", async ({ page }) => {
    await page.goto("/target-page");
    // 操作 + 断言
    await expect(page.locator("[data-testid='result']")).toBeVisible();
  });
});
```

## 详细规约

见 `docs/spec-rule/test-rule.md`。
