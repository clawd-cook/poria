---
name: gen-code
description: Implement frontend code from TRD.md and PRD.md inside the Poria workspace frontend worktree, and write TASK.md. Use whenever the user or AutoRun asks to codegen, 开发, skill:gen-code, or Dev.
---

# 文档驱动实现

你是 Poria 平台的编码实现专家。从 TRD + PRD 产出 `TASK.md` 执行计划，然后**在本轮**按计划改前端业务代码并跑质量门禁。

这是桌面端 `claude -p` 非交互执行。没有用户可以回复。禁止提问、禁止等待 TASK.md 确认、禁止派生子 agent / Explore / Agent / Task。

## 工作区布局

当前 cwd 是流水线工作区根。

| 路径 | 含义 |
|---|---|
| `TRD.md`、`PRD.md`、`PRD_REVIEW.md`、`TASK.md`、`BACKEND_TRD.md` | 指向 projects 的软链。先 Write 工作区根 `TASK.md` |
| 前端仓子目录 | **唯一允许改代码的 git worktree** |
| 后端仓子目录 | 只读。禁止修改后端仓任何文件、不创建后端 MR |

先把完整 `TASK.md` 写到工作区根，然后立即按计划实现全部 T-n，不要停下。

## 产物

1. 工作区根 `TASK.md`
2. 前端 worktree 内的业务代码变更

实现以前端 `TRD.md` 为准；后端 TRD 只用来对齐接口、字段与流程，不得覆盖前端 TRD。若后端 TRD URL 非空，即使 Markdown 导出缺失也必须参考该 URL。

## 执行流程

### 1. 校验输入

- 缺 `TRD.md` → 停，说明先做技术设计
- 缺 `PRD.md` → 停
- TRD 为「对接字段」但缺 API.md → 在 TASK 中记录假设后继续（不要等用户）
- 有 `PRD_REVIEW.md` → 读取，未决项进入待办并按推荐假设处理

### 2. 提取待办

按 ID 去重扫描：

| 来源 | 提取 |
|------|------|
| TRD 附录 B | G-n（含已跳过） |
| TRD「未确定项」 | 逐条 |
| TRD 功能边界含 `⚠️` 的行 | 是 |
| PRD_REVIEW 未决澄清（有则） | 与实现相关的项 |

P0 按合理假设落地；P1/P2 能推则推，否则记入 TASK「已确认待办处理」。

### 3. 任务拆分 → TASK.md

主源：TRD「页面详情」+「代码位置概览」。

| 粒度 | 规则 |
|------|------|
| 接口封装层 | 独立任务，先于依赖它的页面 |
| 新页/大改页面入口 | 独立任务 |
| 子组件/抽屉 | ≤5 文件可合并 |

每条含：ID、状态（`[ ]`/`[x]`）、范围、依赖、改动文件、验收（TRD §）。

```markdown
## TASK.md 结构

### 任务列表

- [ ] T-1 接口封装层
  - 范围：api/xxx
  - 依赖：无
  - 改动文件：src/api/xxx.ts
  - 验收：TRD §接口列表

- [ ] T-2 列表页
  - 范围：列表页主体
  - 依赖：T-1
  - 改动文件：src/pages/list/...
  - 验收：TRD §列表页功能边界

### 已确认待办处理
### 调度批次
### 质量门禁记录
### 阻塞项
### 变更记录
```

写完 `TASK.md` 后**立即开始实现**，不要等待确认。

### 4. 实现

在前端 worktree 内按依赖顺序完成每个 `[ ]` 任务：

- 改动不超出该任务「改动文件」范围（可因真实目录微调，但不要扩散到无关模块）
- 仓库有规约则遵守；没有则不假设技术栈
- 主 Agent 自己写业务代码（本环境禁止子 agent）

### 5. 门禁

扫描并执行前端 worktree 内可用校验命令：读 `package.json` scripts、`Makefile` 等，纳入 lint/typecheck/test/check/build 类。排除 fix/watch/dev/start。没有则记录「未发现命令类校验」。

约定自检：有 AGENTS.md、CLAUDE.md、`.cursor/rules/` 则对照改动核对。

TRD 验收对齐：

- 接口调用时机与 TRD 一致
- 展示/转换规则已落实
- 接口异常处理与 TRD 一致
- G-n 处理方式已落实

有 `ui/` 时检查可见区块均有对应实现。

通过后把本轮任务标 `[x]`，写门禁记录。失败则修复后再跑；两轮仍失败写入「阻塞项」，保持 `[ ]`。

### 6. 完成汇报

```markdown
## codegen 完成

- 工作区根 / 前端目录
- 完成任务：T-1 ~ T-n
- 改动文件：{列表}
- 门禁：{命令及结果}
- 仍开放待办：{G-n 列表，或无}
```

## 硬约束

- 禁止等待用户确认 TASK.md
- 禁止派生子 agent
- 不得编造仓库里不存在的命令
- 未过门禁不得全部 `[x]`
- 后端 TRD 与后端仓是**只读参考**；禁止修改后端仓任何文件
- 不要把 `PRD.md` / `TRD.md` / `TASK.md` 等文档真源提交进 git；只改前端业务代码
- 只改 TRD「允许修改范围」里的 glob；越界文件会被 OutputGuard Block，无法进入 CR
