import { Page } from "@playwright/test";

/**
 * 在 page.goto 之前调用，预设频控 localStorage key 为"已展示"，
 * 使页面加载时引导蒙层/banner 等不会弹出遮挡交互目标。
 *
 * @param page - Playwright Page 实例
 * @param keys - 完整的 localStorage key 列表（由项目自行构造，含日期后缀等）
 * @param value - 写入的值，默认 "1"
 */
export async function suppressFrequencyControl(
  page: Page,
  keys: string[],
  value = "1"
): Promise<void> {
  await page.addInitScript(([ks, v]: [string[], string]) => {
    for (const k of ks) {
      localStorage.setItem(k, v);
    }
  }, [keys, value] as [string[], string]);
}

/**
 * 在 page.goto 之前调用，清除频控 key 使页面加载时会展示引导/蒙层。
 * 用于测试"首次展示"场景。
 *
 * @param page - Playwright Page 实例
 * @param keys - 完整的 localStorage key 列表
 */
export async function clearFrequencyControl(page: Page, keys: string[]): Promise<void> {
  await page.addInitScript((ks: string[]) => {
    for (const k of ks) {
      localStorage.removeItem(k);
    }
  }, keys);
}
