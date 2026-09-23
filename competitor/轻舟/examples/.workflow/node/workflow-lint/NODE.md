# workflow-lint：机器规约闸门

两层检查，性质不同、处理方式不同：

| 层 | 内容 | 性质 | 失败处理 |
|---|---|---|---|
| **A. 代码风格 lint** | tsc / ESLint / Prettier / Stylelint | 机械性（格式/类型/命名） | 原地 autofix + 手动修，不退回 implement |
| **B. 架构 lint** | `.workflow/scripts/lint/lint-*.js` 约束脚本 | 设计层面（模块边界/依赖方向） | 失败 state 不动，提示修后重跑本节点 |

它是**闸门，不是判断**——失败不沉淀踩坑（机器问题不是认知问题），不走 feedback-loop。

---

## A 层：代码风格 lint（探测 → 修复 → 验证）

### A1. 探测项目 lint 能力

检查项目根目录，确定可用的 lint 工具：

| 探测项 | 判据 | 工具 |
|---|---|---|
| TypeScript | 存在 `tsconfig.json` | `npx tsc --noEmit` |
| ESLint | 存在 `.eslintrc*` 或 `eslint.config.*` 或 `package.json` 有 `eslint` 依赖 | `npx eslint` |
| Prettier | 存在 `.prettierrc*` 或 `package.json` 有 `prettier` 依赖 | `npx prettier` |
| Stylelint | 存在 `.stylelintrc*` 或 `package.json` 有 `stylelint` 依赖 | `npx stylelint` |
| package.json lint 脚本 | `scripts.lint` 存在 | 参考其内容了解项目 lint 组合 |

探测结果决定下面执行哪些层。没有的工具直接跳过（不报错）。

### A2. 自动修复（--fix）

对探测到的工具，先跑 auto-fix 修掉能自动修的问题：

1. **Prettier**（如有）：`npx prettier --write <本次变更文件>`
2. **ESLint**（如有）：`npx eslint --fix <本次变更文件>`
3. **Stylelint**（如有）：`npx stylelint --fix <本次变更文件>`

「本次变更文件」= `git diff --name-only HEAD` 涉及的源码文件（`.ts/.tsx/.js/.jsx/.vue/.css/.scss/.less`）。只改动关联文件，不全量扫。

### A3. 验证

依次执行，任一层有错则**当场修复后重跑该层**：

1. **类型检查**（如有 tsconfig）：`npx tsc --noEmit`
2. **ESLint**（如有）：`npx eslint <本次变更文件>` —— 无 `--fix`，纯校验
3. **Stylelint**（如有）：`npx stylelint <本次变更文件>` —— 无 `--fix`，纯校验

每层失败时：读报错 → 定位文件 → 修复 → 重跑该层。循环直到该层通过再进下一层。

A 层全过后，代码已通过项目配置的所有代码风格规则——后续 git commit 时 husky/lint-staged 不会再拦。

---

## B 层：架构 lint（.workflow/scripts/lint/ 约束脚本）

按字典序逐个跑 `.workflow/scripts/lint/lint-*.js`（**排除 `test_*.js` 自测文件**），任一非零退出即停、收集失败输出。

架构约束涉及模块边界、依赖方向等设计决策，失败时：
- state 不动，提示修后重跑本节点（重新 `enter --step workflow-lint` 并按本文执行）
- 如果被非本次改动引起的存量问题拦截，可询问用户意见临时跳过

后续在 `.workflow/scripts/lint/` 新增脚本本节点无需改——字典序自动 pick up。

当前内置规则（随 init 分发）：

| 脚本 | 规则域 |
|---|---|
| `lint-deps.js` | 分层依赖方向 |
| `lint-dead-code.js` | 死代码检测（死组件等） |

新增全局通用规则：写好脚本 → 放入模板 `frontend/_scripts/lint/` → 下次 init 分发到新项目。

---

## 出口

A + B 全部通过 → `advance <task> --step workflow-lint`，照 CLI 打印的闸门指令当场接续（指令是给你执行的，不是转述给用户手敲的）。

无文件产物。
