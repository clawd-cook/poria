import { chromium, Browser, Page } from '@playwright/test';
import { spawn, spawnSync, ChildProcess } from 'child_process';
import * as fs from 'fs';
import * as path from 'path';
import * as https from 'https';
import * as http from 'http';
import { compareImages, DiffResult } from './diff';
import { generateReport, VisualRegressionReport } from './report';

interface VisualCheckConfig {
  devServer: { cmd: string; port: number; readySignal?: string };
  pageUrl: string;
  viewport: { width: number; height: number };
  /**
   * 渲染目标，决定采集后端。缺省 `browser`（Playwright 截 H5/浏览器 devServer）。
   * `weapp-devtool`：小程序走微信开发者工具 CDP（经 @jd/wxa-debugger）采集；
   * 引擎判定/比对/报告与 browser 完全一致，只换采集这一层。
   * `visual-only`：无自动采集，纯目视复核（本 runner 不处理，NODE.md step 4 承担）。
   */
  renderTarget: 'browser' | 'weapp-devtool' | 'visual-only';
  /** renderTarget=weapp-devtool 时用：微信开发者工具能打开的项目根（**构建产物目录**，含 app.json，如 dist/weapp；传 `wxad init --project`）与开发者工具 CDP 端口。runner 不做构建，须先手动构建好 dist。
   * pageMode：目标页由谁到位——
   *   'manual'（缺省）：用户先手动在 IDE 里把模拟器停在**已编译**的目标页，runner 只 attach + 截；
   *   'auto'：runner 用 `wxad goto <pageRoute>` 自动导航到目标页（wx.reLaunch）。auto 时 `pageRoute` 必填。
   * pageRoute：auto 模式的目标页路由 + 参数，如 `pages/x/index?a=1&b=2`（wxad goto 的 ROUTE 参数）。
   * readyOnNetwork：可选，goto 就绪信号——目标页首屏关键接口 URL 子串（如 `functionId=wxHbGroupMainPage`），
   *   goto 会 race 到该请求返回再算就绪，比盲等可靠。两种 pageMode 都要求**先编译好**（runner 不构建）。 */
  weapp?: {
    projectPath?: string;
    port?: number;
    pageMode?: 'manual' | 'auto';
    pageRoute?: string;
    readyOnNetwork?: string;
  };
  /**
   * 可选：整页截图锚定的容器 selector。
   * 缺省（未配）时保持原行为 `fullPage: true`；配了则只截该容器，
   * 用于宿主页含其他楼层/无关内容时把整页对比收敛到被测楼层根节点。
   */
  pageSelector?: string;
  /**
   * 以下三项仅 `renderTarget=browser` 生效（weapp 走 wxad、不经 Playwright）：
   * - browserChannel：'chrome' 用本机系统 Chrome（兜内网 CDN 依赖在 Playwright 自带 chromium 加载不出的情况）；
   *   缺省用 Playwright chromium。
   * - proxy：代理 server（如 whistle `http://127.0.0.1:8899`）；配它时 pageUrl 通常指真实线上 URL，
   *   由代理把被开发楼层的资源映射进线上页（迭代只做部分楼层时用）。
   * - storageState：Playwright storageState JSON 路径，注入登录态（衔接区跑真宿主、接口依赖 cookie 时用）。
   */
  browserChannel?: string;
  proxy?: string;
  storageState?: string;
  /**
   * 设计稿基准图的导出倍率（`export_image` 的 scale），默认 2。
   * 截图的 deviceScaleFactor 与像素比对宽度都按此对齐——基准图与实际截图
   * 倍率不一致时，降采样会让文字抗锯齿永远对不齐、相似度被系统性压低。
   */
  designScale: number;
  designBaseline: {
    pageNodeId: string;
    components: Array<{ name: string; nodeId: string; selector: string }>;
  };
  /**
   * 产物根目录：delivery/<task>/visual-check/
   * 下含 baselines/design/、baselines/actual/、baselines/diff/、reports/
   */
  visualCheckDir: string;
  threshold: { component: number; page: number };
  /**
   * 判定为「显著偏差块」的最小像素面积（比对图坐标系，即已按 designScale 放大后）。
   * 相似度是全局比例，小元素偏差会被稀释；本字段把「局部错位/缺元素」变成可机判项。
   * 缺省按 12 CSS px 见方推导：`(12 * designScale)^2`——小图标/角标量级。
   */
  regionMinArea: number;
  /**
   * 判定为「显著偏差块」的最小填充率（偏差像素数 / bbox 面积）。
   * 与 regionMinArea 是「且」关系。单看面积无法区分小图标与文字抗锯齿——
   * 24px 标题的轮廓 bbox 面积与小图标相当，故必须叠加填充率：
   * 元素缺失/错位是实心块（通常 >0.5），抗锯齿是稀疏轮廓（通常 <0.2）。
   */
  regionMinFillRatio: number;
  maxFixRounds: number;
}

function parseConfig(contextMdPath: string): VisualCheckConfig | null {
  if (!fs.existsSync(contextMdPath)) return null;

  const content = fs.readFileSync(contextMdPath, 'utf-8');
  const section = content.split('## 视觉校验配置')[1];
  if (!section) return null;

  const getField = (name: string): string => {
    // 锚定行首并排除注释行，避免误匹配 "# name: value" 形式的注释
    const match = section.match(new RegExp(`^\\s*${name}:\\s*(.+)`, 'm'));
    return match ? match[1].trim() : '';
  };

  const cmd = getField('cmd').replace(/`/g, '');
  const port = parseInt(getField('port')) || 3000;
  const readySignal = getField('readySignal').replace(/"/g, '') || undefined;
  const pageUrl = getField('pageUrl').replace(/"/g, '') || `http://localhost:${port}`;

  const viewportMatch = section.match(/viewport:\s*(\d+)\s*x\s*(\d+)/);
  const viewport = viewportMatch
    ? { width: parseInt(viewportMatch[1]), height: parseInt(viewportMatch[2]) }
    : { width: 375, height: 812 };

  const pageSelector = getField('pageSelector').replace(/"/g, '').replace(/'/g, '') || undefined;

  // 渲染目标：缺省 browser；weapp-devtool 走开发者工具 CDP 采集
  const rawRenderTarget = getField('renderTarget').replace(/["']/g, '');
  const renderTarget: VisualCheckConfig['renderTarget'] =
    rawRenderTarget === 'weapp-devtool' || rawRenderTarget === 'visual-only'
      ? rawRenderTarget
      : 'browser';
  const weappProjectPath = getField('weappProjectPath').replace(/["']/g, '') || undefined;
  const rawWeappPort = getField('weappPort');
  const weappPort = rawWeappPort !== '' ? parseInt(rawWeappPort) : undefined;
  const rawPageMode = getField('pageMode').replace(/["']/g, '');
  const pageMode = rawPageMode === 'auto' ? 'auto' : 'manual';
  const pageRoute = getField('pageRoute').replace(/["']/g, '') || undefined;
  const readyOnNetwork = getField('readyOnNetwork').replace(/["']/g, '') || undefined;
  // browser 采集的可选增强：系统 Chrome / 代理 / 登录态（仅 renderTarget=browser 生效）
  const browserChannel = getField('browserChannel').replace(/["']/g, '') || undefined;
  const proxy = getField('proxy').replace(/["']/g, '') || undefined;
  const storageState = getField('storageState').replace(/["']/g, '') || undefined;
  // 基准图导出倍率：与 context.md 的「CDN 图片导出倍率」同源；未配则按 explore 阶段默认 2x
  const designScale = parseFloat(getField('designScale')) || 2;
  const pageNodeId = getField('pageNodeId').replace(/"/g, '');

  const componentMatches = [...section.matchAll(
    /- (\w+):\s*\{\s*nodeId:\s*"([^"]+)",\s*selector:\s*"([^"]+)"\s*\}/g,
  )];
  const components = componentMatches.map((m) => ({
    name: m[1],
    nodeId: m[2],
    selector: m[3],
  }));

  const thresholdMatch = section.match(/threshold:\s*\{\s*component:\s*([\d.]+),\s*page:\s*([\d.]+)\s*\}/);
  const threshold = thresholdMatch
    ? { component: parseFloat(thresholdMatch[1]), page: parseFloat(thresholdMatch[2]) }
    : { component: 0.95, page: 0.92 };

  const maxFixRounds = parseInt(getField('maxFixRounds')) || 3;
  // 未配则按 12 CSS px 见方推导（随 designScale 缩放，保证换倍率时判定强度不变）
  const rawArea = getField('regionMinArea');
  const regionMinArea = rawArea !== '' ? parseInt(rawArea) : Math.round((12 * designScale) ** 2);
  const rawFillRatio = getField('regionMinFillRatio');
  const regionMinFillRatio = rawFillRatio !== '' ? parseFloat(rawFillRatio) : 0.35;

  // 产物落 delivery/<task>/visual-check/，与 context.md 同级
  const visualCheckDir = path.resolve(path.dirname(contextMdPath), 'visual-check');

  return {
    devServer: { cmd, port, readySignal },
    pageUrl,
    viewport,
    renderTarget,
    weapp: { projectPath: weappProjectPath, port: weappPort, pageMode, pageRoute, readyOnNetwork },
    browserChannel,
    proxy,
    storageState,
    pageSelector,
    designScale,
    designBaseline: { pageNodeId, components },
    visualCheckDir,
    threshold,
    regionMinArea,
    regionMinFillRatio,
    maxFixRounds,
  };
}

async function waitForServer(url: string, timeoutMs: number = 60000): Promise<boolean> {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    try {
      await new Promise<void>((resolve, reject) => {
        const client = url.startsWith('https') ? https : http;
        const req = client.get(url, (res) => {
          res.resume();
          resolve();
        });
        req.on('error', reject);
        req.setTimeout(2000, () => { req.destroy(); reject(new Error('timeout')); });
      });
      return true;
    } catch {
      await new Promise((r) => setTimeout(r, 1000));
    }
  }
  return false;
}

/**
 * 启动 dev server 并等待就绪。
 *
 * 就绪判定策略（按优先级）：
 * 1. 配了 readySignal：监听进程 stdout/stderr，匹配到关键词后再做一次 HTTP 连通确认
 * 2. 未配 readySignal：直接 HTTP 轮询，适合输出不固定的 server
 *
 * 返回已就绪的子进程；就绪失败时 kill 进程并返回 null。
 */
async function startAndWaitForServer(
  devServer: VisualCheckConfig['devServer'],
  pageUrl: string,
  timeoutMs: number = 60000,
): Promise<ChildProcess | null> {
  const [command, ...args] = devServer.cmd.split(' ');
  const child = spawn(command, args, {
    stdio: 'pipe',
    shell: true,
    cwd: path.resolve(__dirname, '../../../..'),
  });

  if (devServer.readySignal) {
    const signal = devServer.readySignal;
    const signalDetected = await new Promise<boolean>((resolve) => {
      const timer = setTimeout(() => resolve(false), timeoutMs);
      const onData = (chunk: Buffer) => {
        if (chunk.toString().includes(signal)) {
          clearTimeout(timer);
          child.stdout?.off('data', onData);
          child.stderr?.off('data', onData);
          resolve(true);
        }
      };
      child.stdout?.on('data', onData);
      child.stderr?.on('data', onData);
      child.on('exit', () => { clearTimeout(timer); resolve(false); });
    });

    if (!signalDetected) {
      child.kill();
      return null;
    }
    // signal 命中后再做一次 HTTP 确认，防止 server 输出日志时端口还未 listen
    const ready = await waitForServer(pageUrl, 5000);
    if (!ready) { child.kill(); return null; }
  } else {
    const ready = await waitForServer(pageUrl, timeoutMs);
    if (!ready) { child.kill(); return null; }
  }

  return child;
}

async function captureScreenshots(
  config: VisualCheckConfig,
  filterComponents?: string[],
): Promise<{ pagePath: string; componentPaths: Map<string, string> }> {
  const browser: Browser = await chromium.launch({
    headless: true,
    // 顽固内网 CDN 依赖在 Playwright 自带 chromium 加载不出时，切系统 Chrome
    ...(config.browserChannel && config.browserChannel !== 'chromium'
      ? { channel: config.browserChannel }
      : {}),
    // 走本机代理（如 whistle）：pageUrl 指真实线上 URL、由代理映射被开发楼层资源
    ...(config.proxy ? { proxy: { server: config.proxy } } : {}),
  });
  const context = await browser.newContext({
    viewport: config.viewport,
    // 与设计稿基准图导出倍率对齐（config 的 designScale，默认 2）：
    // 倍率不一致时降采样会让文字抗锯齿永远对不齐、相似度被系统性压低、掩盖真实缺陷。
    deviceScaleFactor: config.designScale,
    // 注入登录态（衔接区跑真宿主、接口依赖 cookie 时）：storageState JSON 存在才用
    ...(config.storageState && fs.existsSync(config.storageState)
      ? { storageState: config.storageState }
      : {}),
  });
  const page: Page = await context.newPage();

  await page.goto(config.pageUrl, { waitUntil: 'networkidle' });
  await page.waitForTimeout(1000);
  // remove ifloor debug iframes that overlay floor content in dev mode
  await page.evaluate(() => document.querySelectorAll('iframe').forEach((el) => el.remove()));
  // trigger IntersectionObserver for lazy-loaded images: headless Playwright never scrolls
  await page.evaluate(() => window.scrollTo(0, document.body.scrollHeight));
  await page.waitForTimeout(500);
  await page.evaluate(() => window.scrollTo(0, 0));
  await page.waitForTimeout(300);

  const actualDir = path.join(config.visualCheckDir, 'baselines', 'actual');
  fs.mkdirSync(actualDir, { recursive: true });

  const pagePath = path.join(actualDir, '_page.png');
  // 配了 pageSelector 就只截该容器（宿主页含无关楼层时用），否则保持原 fullPage 行为
  if (config.pageSelector) {
    try {
      const pageLocator = page.locator(config.pageSelector).first();
      await pageLocator.waitFor({ state: 'visible', timeout: 10000 });
      await pageLocator.screenshot({ path: pagePath });
    } catch (err) {
      console.warn(
        `[WARN] pageSelector "${config.pageSelector}" not found, falling back to fullPage: ${err}`,
      );
      await page.screenshot({ path: pagePath, fullPage: true });
    }
  } else {
    await page.screenshot({ path: pagePath, fullPage: true });
  }

  const componentPaths = new Map<string, string>();
  const targetComponents = filterComponents
    ? config.designBaseline.components.filter((c) => filterComponents.includes(c.name))
    : config.designBaseline.components;

  for (const comp of targetComponents) {
    const compPath = path.join(actualDir, `${comp.name}.png`);
    try {
      const locator = page.locator(comp.selector).first();
      await locator.waitFor({ state: 'visible', timeout: 5000 });
      await locator.screenshot({ path: compPath });
      componentPaths.set(comp.name, compPath);
    } catch (err) {
      console.warn(`[WARN] Cannot capture ${comp.name} (selector: ${comp.selector}): ${err}`);
      componentPaths.set(comp.name, '');
    }
  }

  await browser.close();
  return { pagePath, componentPaths };
}

/**
 * weapp 采集后端·页面级（renderTarget=weapp-devtool）。组件级见 captureWeappComponents。
 *
 * 通过 @jd/wxa-debugger 的 `wxad` CLI（纯 CDP，非 miniprogram-automator）驱动微信开发者工具：
 *   1. `wxad init` —— 未起则起 IDE、已起则复用（--project 指构建产物目录），等模拟器 ready；
 *   2. `wxad screenshot --full` —— 截「当前打开页」的真整页（Page.getLayoutMetrics + captureBeyondViewport）。
 *
 * 产出 baselines/actual/_page.png，交给同一套 runComparison——比对/判定/报告与 browser 路径共用。
 * 前置（见 NODE.md「weapp 前置准备」）：先手动构建好 dist + 开发者工具开自动化端口 + 模拟器停在被测页（无编程导航）。
 */
/**
 * 定位 `wxad` 的真实 `.mjs` 入口。**优先全局安装**（与 wxa-debugger skill 共用同一份、版本单一源），
 * 本地 `node_modules` 仅离线兜底；都无 → 报错给安装指引。
 * 返回 `.mjs` 路径（非 bin symlink）——调用侧用 `process.execPath`（跑 runner 的这个 node，够新）执行它，
 * **不让 shebang 挑 PATH 里的 node**：新版 wxad 用可选链等语法，PATH node 若老（如 nvm v12）会启动即崩。
 */
let _wxadMjsCache: string | null = null;
function resolveWxadMjs(): string {
  if (_wxadMjsCache) return _wxadMjsCache;
  const toMjs = (binPath: string): string | null => {
    try {
      const real = fs.realpathSync(binPath); // bin 多为 symlink → 解到真实 .mjs
      return real.endsWith('.mjs') ? real : binPath;
    } catch { return null; }
  };
  // 1) 全局/PATH：`command -v wxad`（登录 shell 含 nvm 等 PATH）
  try {
    const probe = spawnSync('sh', ['-lc', 'command -v wxad'], { encoding: 'utf-8' });
    const p = (probe.stdout || '').trim();
    if (probe.status === 0 && p) { const m = toMjs(p); if (m) { _wxadMjsCache = m; return m; } }
  } catch { /* fall through */ }
  // 2) 本地兜底
  const local = path.resolve(__dirname, 'node_modules', '.bin', 'wxad');
  if (fs.existsSync(local)) { const m = toMjs(local); if (m) { _wxadMjsCache = m; return m; } }
  throw new Error(
    'wxad 未找到。视觉校验(weapp-devtool)依赖全局 wxa-debugger，请先安装：\n' +
    '  npm install -g @jd/wxa-debugger --registry http://registry.m.jd.com\n' +
    '装好后重跑（`command -v wxad` 应能定位到）。');
}

// wxad 命令默认超时（ms）。一般命令给中值；init 冷启+首次编译较久，另给大值（见 WXAD_INIT_TIMEOUT_MS）。
const WXAD_TIMEOUT_MS = 60_000;
// init 专用超时：冷启 IDE + 首次编译产物可能数分钟，远超一般命令；用默认 60s 会在首编时误判超时。
const WXAD_INIT_TIMEOUT_MS = 300_000;

/**
 * 统一执行 wxad：`process.execPath <wxad.mjs> <args>`（node-pinning，见 resolveWxadMjs）。
 * 带超时——**wxad 有「命令完成但进程不退出」的已知行为**（尤其 screenshot：图已写盘却挂住），
 * 无超时会永久 wedge 住 runner。超时到即 SIGKILL 客户端（daemon 另活、不受影响）。
 * capture=true 时管道取 stdout（供 eval 读 JSON）。返回 {stdout, timedOut, status}。
 */
function wxadSpawn(
  args: string[],
  opts: { capture?: boolean; timeoutMs?: number } = {},
): { stdout: string; timedOut: boolean; status: number | null } {
  const mjs = resolveWxadMjs();
  const res = spawnSync(process.execPath, [mjs, ...args], {
    cwd: __dirname,
    stdio: opts.capture ? ['ignore', 'pipe', 'inherit'] : 'inherit',
    encoding: 'utf-8',
    timeout: opts.timeoutMs ?? WXAD_TIMEOUT_MS,
    killSignal: 'SIGKILL',
    maxBuffer: 16 * 1024 * 1024,
  });
  const timedOut = !!(res.error && (res.error as NodeJS.ErrnoException).code === 'ETIMEDOUT');
  return { stdout: res.stdout || '', timedOut, status: res.status };
}

function runWxad(args: string[], timeoutMs?: number): void {
  const r = wxadSpawn(args, timeoutMs ? { timeoutMs } : {});
  if (r.timedOut) throw new Error(`wxad ${args[0]} 超时（${timeoutMs ?? WXAD_TIMEOUT_MS}ms）未返回`);
  if (r.status !== 0) throw new Error(`wxad ${args[0]} 失败（exit ${r.status}）`);
}

/**
 * 同 runWxad，但捕获 stdout（供 `wxad eval` 取 JSON 值）。eval 给较短超时——
 * 卡住的 eval 不该拖住整轮（target 未就绪时 eval 会 hang，见轮询就绪）。
 */
function runWxadCapture(args: string[], timeoutMs = 15_000): string {
  const r = wxadSpawn(args, { capture: true, timeoutMs });
  if (r.timedOut) throw new Error(`wxad ${args[0]} 超时（${timeoutMs}ms）`);
  if (r.status !== 0) throw new Error(`wxad ${args[0]} 失败（exit ${r.status}）`);
  return r.stdout;
}

/**
 * 截图专用：先删旧文件再截，**超时后按「文件是否写出」判成败**——wxad screenshot 常「图写盘了但进程不退出」，
 * 超时 kill 客户端后只要文件在即视为成功（daemon 不退是 wxad 已知行为，不阻断校验）。
 */
function runWxadScreenshot(outPath: string, args: string[], timeoutMs = 45_000): void {
  try { fs.rmSync(outPath, { force: true }); } catch { /* ignore */ }
  const r = wxadSpawn(args, { timeoutMs });
  const written = fs.existsSync(outPath) && fs.statSync(outPath).size > 0;
  if (written) {
    if (r.timedOut) console.warn(`[WARN] wxad screenshot 图已写出但进程未在 ${timeoutMs}ms 内退出（wxad 已知行为），按文件成功继续`);
    return;
  }
  if (r.timedOut) throw new Error(`wxad screenshot 超时（${timeoutMs}ms）且无文件写出——target 可能未就绪`);
  throw new Error(`wxad screenshot 失败（exit ${r.status}）无文件写出`);
}

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

/**
 * 通用 eval（异步）——**wxad eval 同样有「打印结果后进程不退出」通病**：结果 `printJson` 到 stdout、
 * `eval ok` 到 stderr，但进程挂住不退。用 spawnSync 等退出必然等满超时（即使结果早已 stdout 就绪，
 * 这是 waitTargetReady 空转、组件取矩形超时的根因）。改为流式读 stdout——stdout 是纯结果 JSON
 * （printJson pretty 多行），累积到能 `JSON.parse` 即视为就绪、立刻 resolve 并 SIGKILL eval 客户端
 * （一次性命令客户端，不持有 IDE，kill 安全，不影响 daemon）。返回结果 JSON 字符串（调用方自行 parse）。
 */
function wxadEvalAsync(
  expr: string,
  opts: { target?: string; portArgs?: string[]; timeoutMs?: number } = {},
): Promise<string> {
  const mjs = resolveWxadMjs();
  const args = ['eval', ...(opts.target ? ['--target', opts.target] : []), expr, ...(opts.portArgs ?? [])];
  const timeoutMs = opts.timeoutMs ?? 15_000;
  return new Promise((resolve, reject) => {
    const child = spawn(process.execPath, [mjs, ...args], { cwd: __dirname });
    let out = '';
    let settled = false;
    const finish = (fn: () => void) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      try {
        child.kill('SIGKILL');
      } catch {
        /* ignore */
      }
      fn();
    };
    const timer = setTimeout(
      () => finish(() => reject(new Error(`wxad eval 超时（${timeoutMs}ms）`))),
      timeoutMs,
    );
    child.stdout?.on('data', (d: Buffer) => {
      out += d.toString();
      const t = out.trim();
      if (!t) return;
      try {
        JSON.parse(t); // stdout 是纯结果 JSON，能完整解析即拿到结果
        finish(() => resolve(t));
      } catch {
        /* JSON 未累积完整，继续等 */
      }
    });
    child.on('error', (e) => finish(() => reject(e)));
    child.on('exit', (code) =>
      finish(() => {
        const t = out.trim();
        try {
          JSON.parse(t);
          resolve(t);
          return;
        } catch {
          /* fall through */
        }
        reject(new Error(`wxad eval 退出（exit ${code}）无有效结果`));
      }),
    );
  });
}

/**
 * 单次探测模拟器逻辑层是否已就绪（getCurrentPages().length ≥ 1）。
 * 用于 init 前预检：IDE 已在跑且页面已挂上时，复用它、跳过重复 `wxad init` 冷启——
 * 重复 init 既慢又可能与已存活的 daemon 抢通道。走 wxadEvalAsync（规避 eval 不退出通病）。
 */
async function isSimulatorReady(portArgs: string[]): Promise<boolean> {
  try {
    const out = await wxadEvalAsync('getCurrentPages().length', { portArgs, timeoutMs: 8_000 });
    return parseInt(out.trim(), 10) >= 1;
  } catch {
    return false;
  }
}

/**
 * 起 wxad init（异步）——**wxad init 有「打印就绪 JSON 后进程不退出」的已知行为**
 * （daemon 常驻挂住 stdout）。用 spawnSync 会一直等到超时才误判失败（实测 init 11s 就 ready，
 * 却让 spawnSync 空等 300s 再抛超时）。改为流式读 stdout：一旦匹配到就绪信号
 * （`simulator ready` / `"pageframe"` / `"daemon": true`）即视为成功、立刻返回，不等进程退出——
 * init 客户端留后台无害，daemon 独立存活，后续命令按端口连它。全程无就绪信号才超时 reject。
 */
function wxadInitAsync(args: string[], timeoutMs = WXAD_INIT_TIMEOUT_MS): Promise<void> {
  const mjs = resolveWxadMjs();
  return new Promise((resolve, reject) => {
    const child = spawn(process.execPath, [mjs, ...args], { cwd: __dirname });
    let buf = '';
    let settled = false;
    const finish = (fn: () => void) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      fn();
    };
    const timer = setTimeout(
      () => finish(() => reject(new Error(`wxad init 超时（${timeoutMs}ms）未见就绪信号`))),
      timeoutMs,
    );
    const onData = (d: Buffer) => {
      const s = d.toString();
      process.stdout.write(s); // 保留可见性
      buf += s;
      if (/simulator ready|"pageframe"|"daemon"\s*:\s*true/.test(buf)) {
        finish(resolve); // 就绪信号出现即成功；init 进程留后台，不 kill（kill 会连带杀 IDE）
      }
    };
    child.stdout?.on('data', onData);
    child.stderr?.on('data', onData);
    child.on('error', (e) => finish(() => reject(e)));
    child.on('exit', (code) =>
      finish(() =>
        code === 0 ? resolve() : reject(new Error(`wxad init 退出（exit ${code}）未见就绪信号`)),
      ),
    );
    child.unref(); // 不让 init 客户端阻塞 runner 事件循环退出
  });
}

/**
 * 轮询逻辑层 target 就绪：`wxad init` 返回 ≠ 模拟器逻辑层已挂上（冷启+编译要数分钟，未就绪时
 * eval 报 `target not found` / 截图得空白图）。截图前先等 `getCurrentPages().length ≥ 1`。
 * 每条 eval 自带超时（卡住的 eval 不阻塞），最长轮询 maxWaitMs。返回是否就绪。
 */
async function waitTargetReady(portArgs: string[], maxWaitMs = 240_000): Promise<boolean> {
  const deadline = Date.now() + maxWaitMs;
  let n = 0;
  while (Date.now() < deadline) {
    n++;
    try {
      const out = await wxadEvalAsync('getCurrentPages().length', { portArgs, timeoutMs: 10_000 });
      if (parseInt(out.trim(), 10) >= 1) {
        console.log(`[INFO] 模拟器 target 已就绪（轮询 ${n} 次）`);
        return true;
      }
    } catch { /* target 未就绪时 eval 超时/报错，继续轮询 */ }
    await sleep(5_000);
  }
  console.warn('[WARN] 等 target 就绪超时——可能 IDE 未编译完/未停在页面，截图可能为空白');
  return false;
}

async function captureWeappPage(config: VisualCheckConfig): Promise<void> {
  const actualDir = path.join(config.visualCheckDir, 'baselines', 'actual');
  fs.mkdirSync(actualDir, { recursive: true });
  const pagePath = path.join(actualDir, '_page.png');

  const portArgs = config.weapp?.port ? ['--port', String(config.weapp.port)] : [];

  // 产物新鲜度校验（runner 不构建，仅核验 + 提示）：weappProjectPath 须指向已编译产物目录（含 app.json）。
  // 打印 app.json 最后编译时间供人核对，避免拿旧产物截图（编译完成确认见 NODE.md 前置）。
  if (config.weapp?.projectPath) {
    const appJson = path.join(config.weapp.projectPath, 'app.json');
    if (!fs.existsSync(appJson)) {
      throw new Error(
        `weappProjectPath 下无 app.json（${appJson}）——请确认它指向构建产物目录（非源码根）且已编译。`,
      );
    }
    console.log(
      `[INFO] 构建产物 app.json 最后编译时间：${fs.statSync(appJson).mtime.toLocaleString()}` +
        `（runner 不构建，请确认这是含最新改动的编译产物）`,
    );
  }

  const mode = config.weapp?.pageMode ?? 'manual';

  // init 前预检：IDE 已在跑且模拟器就绪则复用，跳过重复 `wxad init` 冷启（省时 + 不与已存活 daemon 抢通道）。
  if (await isSimulatorReady(portArgs)) {
    console.log('[INFO] 检测到模拟器 target 已就绪，复用当前 IDE，跳过 wxad init');
  } else {
    const initArgs = ['init', ...portArgs];
    if (config.weapp?.projectPath) initArgs.push('--project', config.weapp.projectPath);
    console.log(
      `[INFO] wxad init（pageMode=${mode}；就绪信号检测,超时 ${WXAD_INIT_TIMEOUT_MS}ms 兜底；runner 不构建）...`,
    );
    await wxadInitAsync(initArgs);
  }

  if (mode === 'auto') {
    // 自动模式：runner 用 wxad goto 导航到目标页（wx.reLaunch）。先等 app 起到任意页（reLaunch 需 appservice 就绪）。
    const route = config.weapp?.pageRoute;
    if (!route) throw new Error('pageMode=auto 需配置 pageRoute（目标页路由+参数，如 pages/x/index?a=1）。改用 manual 则手动开好目标页。');
    console.log('[INFO] 等 app 就绪后 goto 目标页...');
    await waitTargetReady(portArgs);
    const gotoArgs = ['goto', route, ...portArgs];
    if (config.weapp?.readyOnNetwork) gotoArgs.push('--ready-on-network', config.weapp.readyOnNetwork);
    console.log(`[INFO] wxad goto ${route}${config.weapp?.readyOnNetwork ? `（race 就绪信号 ${config.weapp.readyOnNetwork}）` : ''}...`);
    runWxad(gotoArgs);
  } else {
    console.log('[INFO] manual 模式：使用你已在 IDE 打开的目标页（须已编译、停在被测页）');
  }

  console.log('[INFO] 等模拟器 target 就绪...');
  await waitTargetReady(portArgs);

  console.log('[INFO] wxad screenshot --full：截当前页真整页...');
  runWxadScreenshot(pagePath, ['screenshot', '--full', '--out', pagePath, ...portArgs]);
}

/**
 * eval 表达式：把组件滚进视口，返回其**视口相对** rect（webview 有 DOM）。
 * skyline 页无 HTML DOM（canvas 渲染）→ querySelector 返回 null → 整体返回 null → 上层跳过。
 * scrollIntoView 后立刻取 rect（同步反映新位置），供 `screenshot --crop` 视口裁剪。
 */
function scrollAndRectExpr(selector: string): string {
  const sel = JSON.stringify(selector);
  return (
    `(()=>{const el=document.querySelector(${sel});if(!el)return null;` +
    `el.scrollIntoView({block:'start',inline:'nearest'});const r=el.getBoundingClientRect();` +
    `return{x:Math.round(r.x),y:Math.round(r.y),w:Math.round(r.width),h:Math.round(r.height)};})()`
  );
}

/**
 * weapp 组件级采集（P2）：逐组件 eval 取矩形 → `screenshot --crop` 裁出组件区域。
 *
 * - 走 pageframe（渲染层）的 DOM；CDP `--crop` 传 CSS px、由 CDP 按 dpi scale，无需手算 dpr。
 * - **skyline 兜底**：无 DOM / selector 未命中（如 skyline 不认 `[class*=]`）→ eval 返回 null →
 *   跳过该组件、记入 skipped，交 step 4 目视，不当作引擎失败。
 * 返回未能自动采集、需目视兜底的组件名列表。
 */
async function captureWeappComponents(
  config: VisualCheckConfig,
  filterComponents?: string[],
): Promise<string[]> {
  const actualDir = path.join(config.visualCheckDir, 'baselines', 'actual');
  fs.mkdirSync(actualDir, { recursive: true });
  const portArgs = config.weapp?.port ? ['--port', String(config.weapp.port)] : [];

  const targets = filterComponents
    ? config.designBaseline.components.filter((c) => filterComponents.includes(c.name))
    : config.designBaseline.components;

  const skipped: string[] = [];
  for (const comp of targets) {
    let rect: { x: number; y: number; w: number; h: number } | null = null;
    try {
      const out = await wxadEvalAsync(scrollAndRectExpr(comp.selector), { target: 'pageframe', portArgs });
      const val = JSON.parse(out.trim());
      if (val && val.w > 0 && val.h > 0) rect = val;
    } catch (err) {
      console.warn(`[WARN] 取组件 ${comp.name} 矩形失败：${err}`);
    }
    if (!rect) {
      console.warn(
        `[WARN] 组件 ${comp.name}（selector: ${comp.selector}）未取到矩形——` +
          `skyline 页无 DOM/不认 [class*=]，或元素未渲染；跳过组件级 diff，交 step 4 目视`,
      );
      skipped.push(comp.name);
      continue;
    }
    const compPath = path.join(actualDir, `${comp.name}.png`);
    runWxadScreenshot(compPath, ['screenshot', '--crop', `${rect.x},${rect.y},${rect.w},${rect.h}`, '--out', compPath, ...portArgs]);
  }
  return skipped;
}

async function runComparison(
  config: VisualCheckConfig,
  filterComponents?: string[],
): Promise<VisualRegressionReport> {
  const designDir = path.join(config.visualCheckDir, 'baselines', 'design');
  const diffDir = path.join(config.visualCheckDir, 'baselines', 'diff');

  const pageDesignPath = path.join(designDir, '_page.png');
  const pageActualPath = path.join(config.visualCheckDir, 'baselines', 'actual', '_page.png');
  const pageDiffPath = path.join(diffDir, '_page.png');

  const pageResult = await compareImages(pageDesignPath, pageActualPath, pageDiffPath, {
    targetWidth: Math.round(config.viewport.width * config.designScale),
  });

  const targetComponents = filterComponents
    ? config.designBaseline.components.filter((c) => filterComponents.includes(c.name))
    : config.designBaseline.components;

  const componentResults: Array<{
    name: string;
    selector: string;
    result: DiffResult;
    designImage: string;
    actualImage: string;
  }> = [];

  for (const comp of targetComponents) {
    const designPath = path.join(designDir, `${comp.name}.png`);
    const actualPath = path.join(config.visualCheckDir, 'baselines', 'actual', `${comp.name}.png`);
    const diffPath = path.join(diffDir, `${comp.name}.png`);

    const result = await compareImages(designPath, actualPath, diffPath, {
      targetWidth: Math.round(config.viewport.width * config.designScale),
    });

    componentResults.push({
      name: comp.name,
      selector: comp.selector,
      result,
      designImage: designPath,
      actualImage: actualPath,
    });
  }

  const reportPath = path.join(config.visualCheckDir, 'reports', 'visual-regression-report.json');
  return generateReport(
    { ...pageResult, designImage: pageDesignPath, actualImage: pageActualPath },
    componentResults,
    {
      threshold: config.threshold,
      regionMinArea: config.regionMinArea,
      regionMinFillRatio: config.regionMinFillRatio,
      viewport: `${config.viewport.width}x${config.viewport.height}`,
      pageUrl: config.pageUrl,
    },
    reportPath,
  );
}

async function main() {
  const args = process.argv.slice(2);
  const contextMdPath = args.find((a) => a.startsWith('--context='))?.split('=')[1]
    || path.resolve(__dirname, '../../../../delivery/<task>/context.md');

  const filterArg = args.find((a) => a.startsWith('--components='));
  const filterComponents = filterArg ? filterArg.split('=')[1].split(',') : undefined;

  const config = parseConfig(contextMdPath);
  if (!config) {
    console.error('[ERROR] No visual check config found in context.md. Run with "init" to generate.');
    process.exit(1);
  }

  // weapp-devtool：开发者工具 CDP 采集（页面级 + 组件级），不起 dev server
  if (config.renderTarget === 'weapp-devtool') {
    console.log('[INFO] renderTarget=weapp-devtool：经 @jd/wxa-debugger 采集');
    await captureWeappPage(config);
    const skipped = await captureWeappComponents(config, filterComponents);
    // 只对成功采集到的组件跑引擎；skyline 未命中的交 step 4 目视，不当引擎失败
    const requested = filterComponents ?? config.designBaseline.components.map((c) => c.name);
    const captured = requested.filter((n) => !skipped.includes(n));
    if (skipped.length) {
      console.log(`[INFO] ${skipped.length} 个组件未自动采集、交目视：${skipped.join(', ')}`);
    }
    console.log('[INFO] Running comparison...');
    const report = await runComparison(config, captured);
    console.log(`[INFO] Result: ${report.passed ? 'PASSED' : 'FAILED'}`);
    console.log(
      `[INFO] Page similarity: ${(report.page.similarity * 100).toFixed(1)}%` +
        `, significant diff regions: ${report.page.significantRegionCount}`,
    );
    for (const comp of report.components) {
      const status = comp.passed ? '✓' : '✗';
      console.log(`  ${status} ${comp.name}: ${(comp.similarity * 100).toFixed(1)}%${comp.error ? ` (${comp.error})` : ''}`);
    }
    if (!report.passed) process.exit(1);
    return;
  }

  // Check if server is already running before spawning a new one
  const alreadyRunning = await waitForServer(config.pageUrl, 3000);
  let server: ChildProcess | null = null;

  if (alreadyRunning) {
    console.log('[INFO] Dev server already running, reusing existing instance');
  } else {
    console.log(`[INFO] Starting dev server: ${config.devServer.cmd}`);
    server = await startAndWaitForServer(config.devServer, config.pageUrl);
    if (!server) {
      const hint = config.devServer.readySignal
        ? `readySignal "${config.devServer.readySignal}" not seen in stdout/stderr within 60s`
        : `HTTP not reachable at ${config.pageUrl} within 60s`;
      console.error(`[ERROR] Dev server not ready — ${hint}`);
      process.exit(1);
    }
    console.log('[INFO] Dev server ready');
  }

  try {
    console.log('[INFO] Capturing screenshots...');
    await captureScreenshots(config, filterComponents);

    console.log('[INFO] Running comparison...');
    const report = await runComparison(config, filterComponents);

    console.log(`[INFO] Result: ${report.passed ? 'PASSED' : 'FAILED'}`);
    console.log(
      `[INFO] Page similarity: ${(report.page.similarity * 100).toFixed(1)}%` +
        `, significant diff regions: ${report.page.significantRegionCount}`,
    );
    for (const comp of report.components) {
      const status = comp.passed ? '✓' : '✗';
      const regionNote = comp.significantRegionCount > 0
        ? `, ${comp.significantRegionCount} region(s): ` +
          comp.significantRegions
            .slice(0, 3)
            .map((r) => `${r.width}x${r.height}@(${r.x},${r.y}) fill=${(r.fillRatio * 100).toFixed(0)}%`)
            .join('; ')
        : '';
      console.log(`  ${status} ${comp.name}: ${(comp.similarity * 100).toFixed(1)}%${regionNote}${comp.error ? ` (${comp.error})` : ''}`);
    }
    console.log(
      `[INFO] Fail = similarity below threshold OR any diff region with` +
        ` area >= ${config.regionMinArea}px² and fill >= ${(config.regionMinFillRatio * 100).toFixed(0)}%` +
        ` (small-element drift does not move similarity)`,
    );

    if (!report.passed) {
      process.exit(1);
    }
  } finally {
    // Only kill the server if we started it ourselves
    server?.kill();
  }
}

main().catch((err) => {
  console.error('[FATAL]', err);
  process.exit(1);
});
