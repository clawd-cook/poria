#!/usr/bin/env node
/**
 * 前端架构依赖方向约束检查。
 *
 * 扫描 src/ 下 .ts/.tsx/.js/.jsx 的 import/require，
 * 按规则检查模块边界是否被突破。
 *
 * 用法：
 *   node .workflow/scripts/lint/lint-deps.js
 *   node .workflow/scripts/lint/lint-deps.js --root /path/to/project
 */
/* eslint-disable */
"use strict";
const fs = require("fs");
const path = require("path");

// 按 .workflow/ 标记向上找项目根，而非写死层级——脚本落盘深度以后再挪也不会断。
function findRoot(start) {
  let d = start;
  while (d !== path.dirname(d)) {
    if (fs.existsSync(path.join(d, ".workflow"))) return d;
    d = path.dirname(d);
  }
  return path.resolve(start, "../../..");
}

const ROOT = process.argv.includes("--root")
  ? path.resolve(process.argv[process.argv.indexOf("--root") + 1])
  : findRoot(__dirname);

const SRC = path.join(ROOT, "src");

const EXTENSIONS = [".ts", ".tsx", ".js", ".jsx"];

const LINT_IGNORE_RE = /\/\/\s*@lint-ignore/;

// ============================================================
// 规则定义
// ============================================================

const RULES = [
  {
    code: "components-import-pages",
    severity: "error",
    test: (file, importPath) =>
      file.layer === "components" && resolveLayer(importPath) === "pages",
    why: "components 是可复用 UI 组件，不能依赖 pages 上层页面——否则变成单向耦合死循环。",
    how: "将页面特有逻辑留在 pages/，组件只接收 props；如需共享逻辑提取到 hooks/ 或 utils/。",
  },
  {
    code: "utils-import-components",
    severity: "error",
    test: (file, importPath) =>
      file.layer === "utils" && resolveLayer(importPath) === "components",
    why: "utils/ 是纯逻辑层，引入 UI 组件会拖入 React/DOM 依赖，破坏可测试性和复用性。",
    how: "如果逻辑需要 UI 上下文，放到 hooks/ 而非 utils/；utils/ 保持零 UI 依赖。",
  },
  {
    code: "utils-import-pages",
    severity: "error",
    test: (file, importPath) =>
      file.layer === "utils" && resolveLayer(importPath) === "pages",
    why: "utils/ 是底层纯逻辑，不能反向依赖页面层。",
    how: "页面特有逻辑留在 pages/ 或 hooks/，utils/ 只提供通用工具函数。",
  },
  {
    code: "services-import-components",
    severity: "error",
    test: (file, importPath) =>
      file.layer === "services" && resolveLayer(importPath) === "components",
    why: "services/ 负责数据层（API 调用、数据转换），不应依赖 UI 层。",
    how: "UI 相关逻辑放到组件或 hooks 中，services 只返回纯数据。",
  },
  {
    code: "services-import-pages",
    severity: "error",
    test: (file, importPath) =>
      file.layer === "services" && resolveLayer(importPath) === "pages",
    why: "services/ 是数据层，不能反向依赖页面层。",
    how: "页面逻辑留在 pages/，services 只做数据获取与转换。",
  },
  {
    code: "api-import-components",
    severity: "error",
    test: (file, importPath) =>
      file.layer === "api" && resolveLayer(importPath) === "components",
    why: "api/ 是数据层（接口调用、数据转换），不应依赖 UI 层。",
    how: "UI 相关逻辑放到组件或 hooks 中，api/ 只返回纯数据。",
  },
  {
    code: "api-import-pages",
    severity: "error",
    test: (file, importPath) =>
      file.layer === "api" && resolveLayer(importPath) === "pages",
    why: "api/ 是数据层，不能反向依赖页面层。",
    how: "页面逻辑留在 pages/，api/ 只做接口调用与数据转换。",
  },
  {
    code: "cross-feature-internal",
    severity: "warning",
    test: (file, importPath) => {
      if (!file.feature) return false;
      const target = resolveFeature(importPath);
      if (!target || target === file.feature) return false;
      // 允许从 index 导入，禁止深入内部文件
      return /\/[^/]+\/[^/]+/.test(importPath.split(target)[1] || "");
    },
    why: "跨 feature 引用内部文件会产生隐式耦合，feature 内部重构时外部调用方静默失败。",
    how: "跨 feature 只从对方 index 导入公开 API；需要共享的逻辑提升到 shared/ 或 utils/。",
  },
];

// ============================================================
// 辅助
// ============================================================

const LAYERS = ["pages", "views", "components", "hooks", "services", "utils", "stores", "api"];

function resolveLayer(importPath) {
  for (const layer of LAYERS) {
    if (
      importPath.includes(`/${layer}/`) ||
      importPath.includes(`@/${layer}/`) ||
      importPath.startsWith(`./${layer}/`) ||
      importPath.startsWith(`../${layer}/`)
    ) {
      return layer;
    }
  }
  // 也检查 src 相对路径开头
  const match = importPath.match(/(?:^|[\/@])(?:src\/)?(\w+)\//);
  if (match && LAYERS.includes(match[1])) return match[1];
  return null;
}

function resolveFeature(importPath) {
  // features/xxx/ 或 modules/xxx/ 模式
  const match = importPath.match(/(?:features|modules)\/([^/]+)/);
  return match ? match[1] : null;
}

function getFileLayer(relPath) {
  const parts = relPath.split(path.sep);
  for (const part of parts) {
    if (LAYERS.includes(part)) return part;
  }
  return null;
}

function getFileFeature(relPath) {
  const match = relPath.match(/(?:features|modules)[/\\]([^/\\]+)/);
  return match ? match[1] : null;
}

// ============================================================
// 文件扫描
// ============================================================

function walk(dir) {
  const results = [];
  if (!fs.existsSync(dir)) return results;
  const entries = fs.readdirSync(dir, { withFileTypes: true });
  for (const entry of entries) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      if (entry.name === "node_modules" || entry.name === ".next" || entry.name === "dist") continue;
      results.push(...walk(full));
    } else if (EXTENSIONS.includes(path.extname(entry.name))) {
      results.push(full);
    }
  }
  return results;
}

const IMPORT_RE = /(?:import\s+(?:[\s\S]*?)\s+from\s+['"]([^'"]+)['"]|require\s*\(\s*['"]([^'"]+)['"]\s*\))/g;

function extractImports(content) {
  const imports = [];
  let match;
  IMPORT_RE.lastIndex = 0;
  const lines = content.split("\n");
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    if (LINT_IGNORE_RE.test(line)) continue;
    IMPORT_RE.lastIndex = 0;
    while ((match = IMPORT_RE.exec(line)) !== null) {
      const importPath = match[1] || match[2];
      if (importPath.startsWith(".") || importPath.startsWith("@/") || importPath.startsWith("~/")) {
        imports.push({ line: i + 1, path: importPath });
      }
    }
  }
  return imports;
}

// ============================================================
// 主流程
// ============================================================

function main() {
  const files = walk(SRC);
  const findings = [];

  for (const filePath of files) {
    const relPath = path.relative(ROOT, filePath);
    const content = fs.readFileSync(filePath, "utf-8");
    const layer = getFileLayer(relPath);
    const feature = getFileFeature(relPath);
    const imports = extractImports(content);

    const fileInfo = { path: relPath, layer, feature };

    for (const imp of imports) {
      for (const rule of RULES) {
        if (rule.test(fileInfo, imp.path)) {
          findings.push({
            severity: rule.severity,
            code: rule.code,
            path: relPath,
            line: imp.line,
            detail: `第 ${imp.line} 行：${layer || "unknown"}/ 文件 import 了 ${resolveLayer(imp.path) || imp.path}`,
            importPath: imp.path,
            why: rule.why,
            how: rule.how,
          });
        }
      }
    }
  }

  // 输出
  for (const f of findings) {
    const marker = f.severity === "error" ? "ERROR" : "WARN";
    console.log(`[${marker}] ${f.code}: ${f.path}:${f.line}`);
    console.log(`  WHAT: ${f.detail}`);
    console.log(`  WHY:  ${f.why}`);
    console.log(`  HOW:  ${f.how}`);
    console.log();
  }

  const errorCount = findings.filter((f) => f.severity === "error").length;
  const warnCount = findings.filter((f) => f.severity === "warning").length;
  console.log(
    `lint-deps summary: scanned ${files.length} source files, ${errorCount} error(s), ${warnCount} warning(s).`
  );

  process.exit(errorCount > 0 ? 1 : 0);
}

main();
