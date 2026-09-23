---
name: bootstrap-lint-script
description: "lint 脚本的判定与生成器。用户提诉求 → 先判可机判性（不配就拒并指路），过了再写成 .workflow/scripts/lint/ 下的 JS 脚本。触发：「加个 lint」「写个校验脚本」「这条规约能不能自动查」。"
---

# bootstrap-lint-script：规约 → lint 脚本

lint 是规约的**机械化执行层**：把「人盯着才守得住」的规则变成编码流转门上的硬闸（见 workflow-lint 节点）。

## 第一步：判可机判性（不配就拒）

一条诉求要做成 lint，必须同时过三关：

| 关 | 问题 | 不过 → 去哪 |
|---|---|---|
| **静态可机判** | 扫源码文本/结构就能给出确定的 yes/no，不需要理解语义、上下文或运行起来看？ | 语义/设计判断 → code-rule；运行时才暴露的 → test-plan / autotest |
| **低误报** | 写得出**接近零误报**的判定式？ | 只能给「嫌疑」给不了「违规」→ 留给人/AI 评审 |
| **不重复** | ESLint / 现有 `lint-*.js` 尚未覆盖？ | 已覆盖 → 指出现有覆盖位置，拒绝 |

- **拒绝 ≠ 只说不行**：说明卡在哪一关 + 指路该去的地方。
- **灰色地带剥核**：诉求常是「可机判核 + 语义壳」。剥出能机判的子集做成脚本，剩余指路。

## 第二步：写脚本（过了才写）

参照 `.workflow/scripts/lint/lint-deps.js`（最小样板），硬约定：

- **路径与命名**：`.workflow/scripts/lint/lint-<规则域>.js`。同域规则聚一个文件——能并入现有脚本的规则列表就并入，别为每条规则开新文件。
- **零依赖**：Node.js 标准库 only，`#!/usr/bin/env node`。workflow-lint 在裸环境跑，不装包。
- **出口语义**：
  - exit 0：通过（含 warning only 和目标不存在自动跳过）
  - exit 1：error 级 finding ≥1
  - exit 2：环境/配置问题（如 src/ 完全不存在）
- **目标不存在时的行为**：内置通用规则应 exit 0 跳过（可能不适用当前项目）；不要 exit 2，否则会阻断不需要该规则的项目。
- **输出格式**：每条 finding 打 WHAT / WHY / HOW 三行，结尾一行 summary。
- **豁免口**：支持 `// @lint-ignore`（与现有脚本一致）。
- **自测**：判定式复杂时附 `test_<名>.js`；workflow-lint 排除 `test_*.js`。

写完对全量代码跑一遍验证：**误报为零，或逐条确认确属真违规**，才算交付。新脚本无需任何登记——workflow-lint 按字典序自动 pick up。
