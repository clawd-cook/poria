<div align="center">
  <img src="public/logo-transparent.png" alt="Poria" width="128" />

  # Poria

  *把行云需求变成已提交合并请求的 AI 交付桌面端。*

  [![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
  [![Node](https://img.shields.io/badge/Node.js-24.20.0-3c873a.svg)](https://nodejs.org)
  [![pnpm](https://img.shields.io/badge/pnpm-11.23.0-f69220.svg)](https://pnpm.io)
  [![Tauri](https://img.shields.io/badge/Tauri-v2-24C8DB.svg)](https://tauri.app)
  [![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/clawd-cook/poria)

  [功能](#功能) · [快速开始](#快速开始) · [使用](#使用) · [发布](#发布) · [故障排查](#故障排查) · [架构](ARCHITECTURE.md)

</div>

---

Poria 是基于 Tauri v2 的 macOS 桌面应用。登录京东 SSO 后，从行云拉取「与我相关」的需求，按固定六阶段流水线完成：导出 JoySpace 文档、评审 PRD、生成前端 TRD、用 Claude CLI 改代码、自动 CR，最后推分支、绑定行云变更并创建 Coding 合并请求。

每一阶段对应一个可插拔 Skill，出口有质量门禁。需要人判断时流水线会停在桌面端，由你选择修复重试、跳过或取消。

分层、状态机和数据目录见 [ARCHITECTURE.md](ARCHITECTURE.md)。给编码代理的契约见 [AGENTS.md](AGENTS.md)。

## 功能

- **端到端流水线** — 初始化 → 需求评审 → 技术设计 → 开发 → 代码审查 → 部署
- **行云 / JoySpace / Coding 打通** — SSO Cookie 登录后拉需求、导出文档、绑 EasyCI 变更、创建 MR
- **桌面端看板** — 未开始任务来自行云；运行中 / 阻塞 / 待合并等列来自本地流水线。侧栏还有渠道、技能、仓库、工作区和设置
- **质量门禁** — CR 评分、覆盖率、CI、安全扫描、diff 规模、冲突；失败策略为拦截 / 警告 / 回退
- **风险分级** — 按变更文件模式和 diff 规模评估 `low` / `medium` / `high` / `critical`
- **人机协同** — 阻塞时暂停并通知：修复并重试、跳过、取消 Pipeline
- **多仓编排** — 开发 / CR / 部署可跨仓库按依赖拓扑执行
- **Agent 资源池** — Claude CLI 并发池、输出护栏（文件范围、diff 大小、依赖校验）、会话追踪
- **失败回滚** — 各阶段可记录 worktree 清理、删分支、关 MR 等回滚指令；门禁回退每个流水线只允许一次

## 快速开始

### 环境

- **Node.js** `24.20.0`（[nvm](https://github.com/nvm-sh/nvm)，不要用 Homebrew Node）
- **pnpm** `11.23.0`
- **Rust** stable，以及 `cargo tauri`（`cargo install tauri-cli`）
- **Claude CLI**（评审 / 设计 / 开发 / CR 会拉起 `claude` 子进程）
- macOS（CI 当前打 Apple Silicon DMG）

> [!IMPORTANT]
> 本地必须用 nvm 的 `v24.20.0` 和 pnpm `11.23.0`。版本不对就停，不要换运行时或包管理器。本仓库没有 `@tauri-apps/cli`，请用 `cargo tauri`，不要用 `pnpm tauri`。

```bash
export NVM_DIR="$HOME/.nvm"
[ -s "/opt/homebrew/opt/nvm/nvm.sh" ] && . "/opt/homebrew/opt/nvm/nvm.sh"
nvm use 24.20.0
export PATH="$HOME/.nvm/versions/node/v24.20.0/bin:$HOME/.cargo/bin:$PATH"

node -v    # 必须是 v24.20.0
pnpm -v    # 必须是 11.23.0

pnpm install
cargo tauri dev
```

窗口标题是 **Poria**，进程名 `poria-desktop`。验证登录、需求、克隆、提交流水线必须用这个窗口。

仅前端（没有 Tauri IPC，登录 / 拉需求不可用）：

```bash
pnpm dev           # http://localhost:1420
```

> [!TIP]
> 端口 **1420 被占用时先释放**（`vite.config.ts` 开启了 `strictPort`）。调试请用 `target/debug/poria-desktop`，不要拿过期的 `/Applications/Poria.app` 当当前分支。

生产包：

```bash
cargo tauri build
cargo tauri build --target aarch64-apple-darwin
```

无 Apple 开发者证书时走 ad-hoc 签名。从网盘或浏览器下载的 unsigned DMG，首次打开前执行：

```bash
xattr -cr /Applications/Poria.app
```

## 使用

1. 打开 Poria，右上角 **登录**（京东 SSO，Cookie 写入 `~/.poria/auth.json`，用于行云 / JoySpace / Coding）。未登录时看板不会拉需求。
2. **仓库** 登记前端与后端仓库，克隆完成后状态为 `ready`。克隆目录为 `~/.poria/repos/<scope>/<name>`。
3. **看板** 未开始列默认是行云「与我相关」；勾选「由我受理」只影响该查询。点任务按向导选择前端仓、后端仓 / 分支、JoySpace PRD 与后端 TRD 链接。也可从需求链接开始。
4. 看板上点进流水线，按阶段 **执行**。阻塞时在卡片上选择「修复并重试 / 跳过 / 取消 Pipeline」。

同一需求键（优先 `demand_code`，否则 `demand_id`）会复用最近一条流水线：`Created` 可更新配置，其它状态直接返回已有 id。提交流水线必须带后端 TRD 链接。

设置页可调门禁阈值与 Claude 路径：

| 项 | 默认 | 说明 |
|---|---|---|
| CR 评分阈值 | `B+` | 代码审查最低通过等级 |
| 测试覆盖率 | `80%` | 部署门禁覆盖率 |
| 最大变更行数 | `500` | diff 规模告警 |
| Agent 超时 | `1800000ms` | Claude 执行超时（30 分钟） |
| 最大重试次数 | `3` | 单阶段重试上限 |
| Claude 路径 | 空 | 留空则登录 shell `which claude`；填写必须是绝对路径 |

常用检查：

```bash
pnpm typecheck
pnpm exec oxfmt .
cargo check --workspace
cargo test --workspace
```

## 发布

打 tag 触发 GitHub Actions。当前只编 **Apple Silicon**（`aarch64-apple-darwin`）DMG。有 `APPLE_*` secret 则公证签名，没有则 ad-hoc 签名并继续出包。

| 类型 | Tag | Workflow | 产物 |
|---|---|---|---|
| Beta | `vX.Y.Z-beta.N` | `pre-publish.yml` | GitHub pre-release |
| 正式版 | `vX.Y.Z` | `publish.yml` | latest Release（非 pre-release） |

```bash
git tag v1.1.1-beta.1 && git push origin v1.1.1-beta.1
git tag v1.1.1 && git push origin v1.1.1
```

> [!NOTE]
> GitHub 跑的是 **打 tag 那次提交上的 workflow**。改了 YAML 之后需要重新打 tag 才会生效。

## 故障排查

| 现象 | 处理 |
|---|---|
| 浏览器打开 `:1420` 登录 / 拉需求失败 | 这是 Vite，没有 Tauri `invoke`。用 `cargo tauri dev` 打开桌面窗口 |
| `pnpm tauri` 报 `command not found` | 本仓库没有 `@tauri-apps/cli`。用 `cargo tauri` |
| 看板报「请先登录」 | `~/.poria/auth.json` 里没有有效 SSO Cookie |
| EasyCI `522721` / 找不到分支 | 远端还没有该分支。必须先 `git push` 再 SELECT 绑定 |
| GraphQL `UnusedVariable` | EasyCI 不接受查询里未使用的变量，不要往 mutation 塞多余字段 |
| `createChange` 失败 | `branchOperateType` 只能是 `SELECT`，且分支必须已在远端 |
| Accessibility 找不到窗口 | 窗口标题是 `Poria`，不是端口号 |
| Claude 阶段立刻失败 | 设置页刷新 Claude 状态；GUI 应用往往没有 Homebrew `PATH`，请填绝对路径或确认登录 shell 的 `which claude` |
