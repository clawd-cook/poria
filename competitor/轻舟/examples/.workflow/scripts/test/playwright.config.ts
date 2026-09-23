/* eslint-disable */
import { defineConfig, devices } from "@playwright/test";

/**
 * Playwright E2E 配置。
 * 对标后端 pytest.ini：定义测试目录、浏览器矩阵、reporter。
 *
 * 环境变量：
 *   TEST_BASE_URL — 被测服务地址（必填）
 *   TEST_PROXY    — whistle 代理地址（local 模式，如 http://127.0.0.1:12345）
 */
export default defineConfig({
  testDir: "./tasks",
  testMatch: "**/*.spec.ts",

  timeout: 30_000,
  retries: 1,
  workers: 4,

  use: {
    baseURL: process.env.TEST_BASE_URL || "http://localhost:3000",
    storageState: "./storage-state.json",
    trace: "on-first-retry",
    screenshot: "only-on-failure",
    ignoreHTTPSErrors: true,
    ...(process.env.TEST_PROXY ? { proxy: { server: process.env.TEST_PROXY } } : {}),
  },

  projects: [
    { name: "chromium", use: { browserName: "chromium" } },
    // 移动端设备模拟示例（按需取消注释）：
    // { name: "mobile", use: { ...devices["iPhone 12"] } },
  ],

  reporter: [
    ["list"],
    ["junit", { outputFile: "../../../delivery/__task__/test/junit.xml" }],
    ["html", { outputFolder: "../../../delivery/__task__/test/test-results", open: "never" }],
  ],

  globalSetup: "./global-setup.ts",
});
