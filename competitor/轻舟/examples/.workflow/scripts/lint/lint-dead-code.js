#!/usr/bin/env node
/**
 * 死代码检测。
 *
 * 当前检测项：
 * - 死组件：src/components/ 下定义但未被任何文件 import 的组件目录
 *
 * 后续可扩展：死 hooks、死 utils、死 stores 等。
 *
 * 用法：
 *   node .workflow/scripts/lint/lint-dead-code.js
 *   node .workflow/scripts/lint/lint-dead-code.js --root /path/to/project
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
const COMPONENTS_DIR = path.join(SRC, "components");

const EXTENSIONS = [".ts", ".tsx", ".js", ".jsx"];

const LINT_IGNORE_RE = /\/\/\s*@lint-ignore/;

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

function getComponentDirs() {
  if (!fs.existsSync(COMPONENTS_DIR)) return [];
  return fs.readdirSync(COMPONENTS_DIR, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => entry.name);
}

// ============================================================
// 主流程
// ============================================================

function main() {
  const componentNames = getComponentDirs();

  if (componentNames.length === 0) {
    console.error("lint-dead-code: src/components/ 不存在或为空，跳过。");
    process.exit(0);
  }

  const srcFiles = walk(SRC);

  if (srcFiles.length === 0) {
    console.error("lint-dead-code: src/ 下未找到任何源文件，无法判定引用关系。");
    process.exit(2);
  }

  const importLines = [];
  for (const filePath of srcFiles) {
    const content = fs.readFileSync(filePath, "utf-8");
    const lines = content.split("\n");
    for (const line of lines) {
      if (LINT_IGNORE_RE.test(line)) continue;
      importLines.push(line);
    }
  }
  const importText = importLines.join("\n");

  const findings = [];

  for (const name of componentNames) {
    const indexFile = path.join(COMPONENTS_DIR, name, "index.tsx");
    const indexFileAlt = path.join(COMPONENTS_DIR, name, "index.ts");
    if (!fs.existsSync(indexFile) && !fs.existsSync(indexFileAlt)) continue;

    const patterns = [
      `components/${name}`,
      `components/${name}/`,
      `components/${name}'`,
      `components/${name}"`,
    ];

    const isImported = patterns.some((pattern) => importText.includes(pattern));

    if (!isImported) {
      const relPath = path.relative(ROOT, path.join(COMPONENTS_DIR, name, "index.tsx"));
      findings.push({ name, path: relPath });
    }
  }

  for (const f of findings) {
    console.log(`[ERROR] dead-component: ${f.path}`);
    console.log(`  WHAT: 组件 ${f.name} 定义在 src/components/${f.name}/ 但未被项目中任何文件 import。`);
    console.log(`  WHY:  死组件增加维护成本，迷惑后续开发者，浪费测试生成精力；若被误用会产生重复实现并存。`);
    console.log(`  HOW:  确认功能已被其他组件承载后删除该目录；若仍需使用，在父组件中正确 import 渲染。`);
    console.log();
  }

  console.log(
    `lint-dead-code summary: scanned ${componentNames.length} component dirs, ${findings.length} dead component(s) found.`
  );

  process.exit(findings.length > 0 ? 1 : 0);
}

main();
