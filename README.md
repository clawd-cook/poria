<div align="center">
  <img src="public/logo-transparent.png" alt="Poria" width="128" />

  # Poria

  *把行云需求变成已提交合并请求的 AI 交付桌面端。*

  [![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
  [![Node](https://img.shields.io/badge/Node.js-24.20.0-3c873a.svg)](https://nodejs.org)
  [![pnpm](https://img.shields.io/badge/pnpm-11.23.0-f69220.svg)](https://pnpm.io)
  [![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/clawd-cook/poria)

  [功能](#功能) · [架构](#架构) · [快速开始](#快速开始) · [使用](#使用) · [开发](#开发) · [发布](#发布) · [贡献](#贡献)

</div>

---

Poria 是基于 Tauri v2 的 macOS 桌面应用。登录京东 SSO 后，从行云拉取「与我相关」的需求，按固定七阶段流水线完成：导出 JoySpace 文档、评审 PRD、生成 TRD、创建 git worktree、用 Claude CLI 改代码、自动 CR，最后推分支、绑定行云变更并创建 Coding 合并请求。

每一阶段对应一个可插拔 Skill，出口有质量门禁；需要人判断时流水线会停在桌面端，由你选择修复重试、跳过或取消。

## 功能

- **端到端流水线** — 初始化 → 需求评审 → 技术设计 → 工作区 → 开发 → 代码审查 → 部署
- **行云 / JoySpace / Coding 打通** — SSO Cookie 登录后拉需求、导出文档、绑 EasyCI 变更、创建 MR
- **桌面端看板** — 首页看板、需求列表、仓库托管、设置；阶段进度、事件流、人工介入卡片
- **质量门禁** — CR 评分、覆盖率、CI、安全扫描、diff 规模、冲突；失败策略为拦截 / 警告 / 回退
- **风险分级** — 按变更文件模式和 diff 规模评估 low / medium / high / critical
- **人机协同** — 阻塞时暂停并通知：修复并重试、跳过、取消 Pipeline
- **多仓编排** — 开发 / CR / 部署可跨仓库按依赖拓扑执行
- **Agent 资源池** — Claude CLI 并发池、输出护栏（文件范围、diff 大小、依赖校验）、会话追踪
- **失败回滚** — 各阶段记录 worktree 清理、删分支、关 MR 等回滚指令

## 架构

```
poria/
├── src/                      # React 前端（Ant Design 6 + Ant Design X）
├── src-tauri/                # Tauri v2 壳与 IPC 命令
├── crates/
│   ├── poria-core            # 领域类型、状态机、门禁、风险分级
│   ├── poria-commands        # 流水线执行、错误分类、回滚
│   ├── poria-skills          # 七阶段内置 Skill
│   ├── poria-resources       # 终端、worktree、Claude Agent 池
│   ├── poria-channels        # Coding、JoySpace、行云、JME、缺陷
│   └── poria-infrastructure  # SQLite、鉴权、配置、日志、指标
└── public/                   # 静态资源
```

### 流水线状态

```
Created → Running → WaitingMerge → Completed
              ↓          ↓
           Blocked     Failed
              ↓          ↓
           Running    Cancelled
```

阶段自身走 `Pending → Running → Completed`，也可进入 `Failed` / `Blocked` / `Skipped`。失败可按配置重试；阻塞则等人处理。回退每个流水线只允许一次，第二次回退失败会升级为拦截。

### 七个阶段

| 阶段 | Skill | 产出 |
|---|---|---|
| 初始化 | `InitSkill` | 从 JoySpace 导出 `PRD.md`、`BACKEND_TRD.md` 到 `~/.poria/projects/<需求编号>/` |
| 需求评审 | `ReviewPrdSkill` | `PRD_REVIEW.md`（P0 / P1 / P2 澄清项） |
| 技术设计 | `GenTrdSkill` | `TRD.md` 与允许修改范围 |
| 工作区 | `WorkspaceSkill` | git worktree + `feature_<需求编号>` 分支 |
| 开发 | `GenCodeSkill` | 按 TRD 改代码，写出 `TASK.md` |
| 代码审查 | `CodeReviewSkill` | `CR.md`，过评分 / 安全扫描门禁 |
| 部署 | `DeploySkill` | 提交 → 推送 → 绑定行云 EasyCI 变更 → 创建或复用 Coding MR |

> [!NOTE]
> 评审、设计、开发、CR 通过 `claude -p` 非交互执行。部署顺序是 **先 push 再 SELECT 绑定**：EasyCI `createChange` 只能选择远端已存在的分支。

## 快速开始

### 环境

- **Node.js** `24.20.0`（[nvm](https://github.com/nvm-sh/nvm)，不要用 Homebrew Node）
- **pnpm** `11.23.0`
- **Rust** stable
- **Claude CLI**（桌面阶段会拉起 `claude` 子进程）
- macOS（当前发布产物是 Apple Silicon / Intel DMG）

> [!IMPORTANT]
> 本地开发必须用 nvm 的 `v24.20.0` 和 pnpm `11.23.0`。版本不对就停，不要换运行时或包管理器。

```bash
export NVM_DIR="$HOME/.nvm"
[ -s "/opt/homebrew/opt/nvm/nvm.sh" ] && . "/opt/homebrew/opt/nvm/nvm.sh"
nvm use 24.20.0

node -v    # 必须是 v24.20.0
pnpm -v    # 必须是 11.23.0

pnpm install
```

### 运行

```bash
pnpm tauri dev     # 编译 Rust 后端 + Vite（:1420）+ 打开桌面窗口
```

仅前端（无法调用 Tauri 命令，登录 / 拉需求不可用）：

```bash
pnpm dev           # http://localhost:1420
```

生产包：

```bash
pnpm tauri build
pnpm tauri build --target aarch64-apple-darwin
pnpm tauri build --target x86_64-apple-darwin
```

无 Apple 开发者证书时走 ad-hoc 签名。从网盘或浏览器下载的 unsigned DMG，首次打开前执行：

```bash
xattr -cr /Applications/Poria.app
```

## 使用

1. 打开 Poria，右上角 **登录**（京东 SSO，Cookie 用于行云 / JoySpace / Coding）。
2. **仓库列表** 登记前端（及可选后端）仓库，克隆完成后状态为 `ready`。
3. **需求列表** 打开行云需求，按向导选择前端仓、后端仓 / 分支、JoySpace PRD（及后端 TRD）链接。
4. **首页** 看板上点进流水线，按阶段 **执行**。阻塞时在卡片上选择「修复并重试 / 跳过 / 取消 Pipeline」。

设置页可调门禁阈值：

| 项 | 默认 | 说明 |
|---|---|---|
| CR 评分阈值 | `B+` | 代码审查最低通过等级 |
| 测试覆盖率 | `80%` | 部署门禁覆盖率 |
| 最大变更行数 | `500` | diff 规模告警 |
| Agent 超时 | `300000ms` | Claude 执行超时 |
| 最大重试次数 | `3` | 单阶段重试上限 |

## 开发

```bash
pnpm typecheck              # tsc -b
pnpm exec oxfmt .           # 前端格式化
cargo check --workspace
cargo test --workspace
cargo test -p poria-core -- state_machine
```

| Crate | 职责 |
|---|---|
| `poria-core` | 领域类型、状态机、门禁、风险分级、多仓拓扑 |
| `poria-commands` | 流水线执行器、错误分类、回滚 |
| `poria-skills` | Init / ReviewPRD / GenTRD / Workspace / GenCode / CodeReview / Deploy |
| `poria-resources` | 终端、worktree、Claude Agent 池、输出护栏 |
| `poria-channels` | Coding、JoySpace、行云、JME、缺陷 |
| `poria-infrastructure` | SQLite、鉴权、配置、日志、指标 |

| 层 | 技术 |
|---|---|
| 后端 | Rust 2021、Tokio、SQLite（rusqlite）、Serde |
| 前端 | React 19、TypeScript、Vite 8、Ant Design 6、Ant Design X、Tailwind CSS 4 |
| 桌面 | Tauri v2 |
| Agent | Claude CLI 子进程池 |

> [!WARNING]
> `submodules/` 是 git submodule，不要直接改里面的文件。

## 发布

GitHub Actions 打 macOS DMG（`aarch64` + `x86_64`）。有 `APPLE_*` secret 则公证签名，没有则 ad-hoc 签名并继续出包。

| 类型 | Tag | Workflow | 产物 |
|---|---|---|---|
| Beta | `vX.Y.Z-beta.N` | `pre-publish.yml` | GitHub pre-release |
| 正式版 | `vX.Y.Z` | `publish.yml` | latest Release（非 pre-release） |

```bash
git tag v1.1.1-beta.1 && git push origin v1.1.1-beta.1
git tag v1.1.1 && git push origin v1.1.1
```

## 贡献

开发环境、分层约定和提交检查见 [CONTRIBUTING.md](CONTRIBUTING.md)。缺陷、功能建议和开发疑问请用 GitHub Issue 表单；安全问题见 [SECURITY.md](SECURITY.md)，不要开公开 Issue。
