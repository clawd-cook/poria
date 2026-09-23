import { chromium, FullConfig } from "@playwright/test";
import { execSync } from "child_process";
import * as fs from "fs";
import * as path from "path";

/**
 * 全局 Setup：登录态注入。
 *
 * 对标后端 conftest.py 的 auth_cookie fixture：
 * 优先级：TEST_COOKIE 环境变量 > lbcli getssologin 自动导出。
 *
 * 产出 storage-state.json 供所有测试共享登录态。
 */
const STORAGE_STATE_PATH = path.join(__dirname, "storage-state.json");

async function globalSetup(config: FullConfig) {
  const baseURL = config.projects[0]?.use?.baseURL || process.env.TEST_BASE_URL;
  if (!baseURL) {
    throw new Error("TEST_BASE_URL 未配置：请设置 TEST_BASE_URL 环境变量");
  }

  const cookie = getCookie();
  if (!cookie) {
    throw new Error(
      "登录态获取失败：设置 TEST_COOKIE 环境变量，或确保 lbcli doctor 为 connected"
    );
  }

  // 将 Cookie 字符串解析为 Playwright 格式并写入 storageState
  const cookies = parseCookieString(cookie, baseURL);

  const browser = await chromium.launch();
  const context = await browser.newContext();
  await context.addCookies(cookies);
  await context.storageState({ path: STORAGE_STATE_PATH });
  await browser.close();
}

function getCookie(): string | null {
  // 优先环境变量
  if (process.env.TEST_COOKIE) {
    return process.env.TEST_COOKIE.trim();
  }
  // 自动导出：lbcli getssologin
  try {
    const result = execSync("lbcli getssologin getssologin -f plain", {
      encoding: "utf-8",
      timeout: 60_000,
    });
    return result.trim() || null;
  } catch {
    return null;
  }
}

function parseCookieString(
  cookieStr: string,
  baseURL: string
): Array<{ name: string; value: string; domain: string; path: string }> {
  const url = new URL(baseURL);
  const parts = url.hostname.split(".");
  const domain = parts.length > 2 ? "." + parts.slice(-2).join(".") : url.hostname;
  return cookieStr.split(";").map((pair) => {
    const [name, ...rest] = pair.trim().split("=");
    return {
      name: name.trim(),
      value: rest.join("=").trim(),
      domain,
      path: "/",
    };
  });
}

export default globalSetup;
