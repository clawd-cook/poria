import { Page } from "@playwright/test";

/**
 * 注入 MutationObserver 持续移除 whistle 代理注入的全屏透明 iframe（z-index: 2147483647）。
 * 仅在 TEST_PROXY 环境变量存在时生效（即 local 模式走 whistle 代理时）。
 * 必须在 page.goto 之前调用。
 */
export async function installProxyIframeKiller(page: Page): Promise<void> {
  if (!process.env.TEST_PROXY) return;
  await page.addInitScript(() => {
    const kill = () => {
      document.querySelectorAll('iframe').forEach(f => {
        const s = f.getAttribute('style') || '';
        if (s.includes('2147483647')) {
          f.remove();
        }
      });
    };
    if (document.body) {
      kill();
      new MutationObserver(kill).observe(document.body, { childList: true, subtree: true });
    }
    document.addEventListener('DOMContentLoaded', () => {
      kill();
      new MutationObserver(kill).observe(document.body, { childList: true, subtree: true });
    });
  });
}

/**
 * post-load 兜底清理代理 iframe。在 page.goto + waitForTimeout 之后调用。
 * 仅在 TEST_PROXY 环境变量存在时执行。
 */
export async function removeProxyIframeIfNeeded(page: Page): Promise<void> {
  if (!process.env.TEST_PROXY) return;
  await page.evaluate(() => {
    document.querySelectorAll('iframe').forEach(f => {
      if ((f.getAttribute('style') || '').includes('2147483647')) f.remove();
    });
  });
}
