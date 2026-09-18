# 贡献指南

感谢参与 Poria。产品说明见 [README.md](README.md)；分层与流水线见 [ARCHITECTURE.md](ARCHITECTURE.md)；给编码代理的契约见 [AGENTS.md](AGENTS.md)。参与即表示同意 [行为准则](CODE_OF_CONDUCT.md)。

## 报告问题

- **安全漏洞**：不要开公开 Issue。按 [SECURITY.md](SECURITY.md) 私下披露。
- **缺陷**：先搜现有 Issue，再用 [缺陷报告表单](.github/ISSUE_TEMPLATE/bug_report.yml)。写清 Poria / Node / pnpm / Rust 版本、复现步骤、期望与实际结果。登录、需求列表、克隆、提交流水线必须在 **Poria 桌面窗口**里复现，不要用浏览器打开 `http://localhost:1420`。
- **功能建议**：用 [功能建议表单](.github/ISSUE_TEMPLATE/feature_request.yml)。较大改动请先开 Issue 再写代码。
- **使用或开发疑问**：用 [问题表单](.github/ISSUE_TEMPLATE/question.yml)。空白 Issue 已关闭，请不要绕过模板。
- **还不成熟的想法**：用 [Ideas Discussion](https://github.com/clawd-cook/poria/discussions/new?category=ideas)，表单见 [ideas.yml](.github/DISCUSSION_TEMPLATE/ideas.yml)。能说清问题和方案后再开功能建议 Issue。

不要在 Issue、Discussion 或 PR 里粘贴 SSO Cookie、`~/.poria/auth.json`、`~/.poria/config.json`、Apple / Tauri 签名密钥。

## 开发环境

本地只使用：

- Node.js **24.20.0**（nvm，不要用 Homebrew Node）
- pnpm **11.23.0**
- Rust **stable**
- macOS（CI 当前打 **Apple Silicon** DMG；本机仍可交叉编 `x86_64-apple-darwin`）
- 跑流水线 Agent 阶段时需要本机 `claude` CLI（设置页可填绝对路径；不要假设 GUI `PATH` 里有 `claude`）

```bash
export NVM_DIR="$HOME/.nvm"
[ -s "/opt/homebrew/opt/nvm/nvm.sh" ] && . "/opt/homebrew/opt/nvm/nvm.sh"
nvm use 24.20.0

node -v    # 必须是 v24.20.0
pnpm -v    # 必须是 11.23.0

pnpm install
```

不要用 `npm` / `yarn` / `bun` 装依赖，也不要改 `package.json` 里的 `packageManager`。

本仓库 **没有** `@tauri-apps/cli` 依赖。`package.json` 的 `"tauri": "tauri"` 只有系统 `PATH` 上已有 `tauri` 二进制时才可用。本地请用 `cargo tauri`（见下）。不要为了跑通脚本去加 `@tauri-apps/cli`，除非维护者明确要求。

更完整的产品说明见 README「快速开始」。分层以 [ARCHITECTURE.md](ARCHITECTURE.md) 为准；编码代理契约以 [AGENTS.md](AGENTS.md) 为准。

## 日常开发

```bash
export NVM_DIR="$HOME/.nvm"
. "/opt/homebrew/opt/nvm/nvm.sh"
nvm use 24.20.0
export PATH="$HOME/.nvm/versions/node/v24.20.0/bin:$HOME/.cargo/bin:$PATH"

cargo tauri dev    # 打开桌面窗口（进程 poria-desktop，标题 Poria）
pnpm typecheck
pnpm exec oxfmt .
cargo check --workspace
cargo test --workspace
```

`tauri.conf.json` 的 `beforeDevCommand` 是 `pnpm dev`，所以跑 `cargo tauri dev` 前必须已 `nvm use 24.20.0`。端口 **1420 被占用时先释放**（`strictPort: true`）。

> [!IMPORTANT]
> Cursor / 浏览器打开 Vite（`:1420`）**没有** Tauri `invoke`。验证登录、需求、仓库克隆、提交流水线必须用 `cargo tauri dev` 起的窗口。不要用过期的 `/Applications/Poria.app` 测当前分支；用 `target/debug/poria-desktop`。细节见 [.trellis/spec/frontend/tauri-desktop-testing.md](.trellis/spec/frontend/tauri-desktop-testing.md)。

IPC 封装只放在 `src/lib/tauri.ts`。前端参数用 camelCase（`pipelineId`、`backendTrdUrl`），对应 Rust snake_case。`#[tauri::command]` 只写在 `src-tauri/src/commands/`，并在 `src-tauri/src/lib.rs` 的 `generate_handler!` 里注册。

前端没有 ESLint / Tailwind。布局用 Ant Design 6 + `src/theme.ts` 的 `poriaTheme`，样式以 token 和 `src/styles.css` 重置为准。格式化用 `pnpm exec oxfmt .`。主题约定见 [.trellis/spec/frontend/visual-theme.md](.trellis/spec/frontend/visual-theme.md)。

改某一层之前读 `.trellis/spec/` 对应索引。跨层字段（需求筛选、`backendTrdUrl`、前后端 TRD）先看 [.trellis/spec/guides/cross-layer-thinking-guide.md](.trellis/spec/guides/cross-layer-thinking-guide.md)。Init 工作区 / worktree / Skill 软链见 [.trellis/spec/frontend/pipeline-workspace.md](.trellis/spec/frontend/pipeline-workspace.md)。Claude 路径见 [.trellis/spec/frontend/claude-cli.md](.trellis/spec/frontend/claude-cli.md)。

流水线是 **六个阶段**（`Init` → `ReviewPrd` → `Design` → `Dev` → `Cr` → `Deploy`）。没有独立的 Workspace 阶段；worktree 在 Init 里创建。不要把 `Workspace` 加回 `STAGE_ORDER`。侧栏「技能」只列出随包 `skills/*/SKILL.md`（`review-prd` / `gen-trd` / `gen-code` / `code-review`）；Init / Deploy 是 Rust 阶段实现，不是那份清单。

## 提交与 Pull Request

1. 从最新 `main` 开分支。
2. 只改与问题相关的文件。不要改 `submodules/` 里的内容；实现应写在本仓库的 crate / `src/`。
3. Rust 改动补 `#[cfg(test)]` / `#[tokio::test]`。前端没有测试框架，至少 `pnpm typecheck`，涉及 IPC 时在桌面窗口走一遍主路径。
4. 提交说明写「为什么」，一两句即可。无强制前缀。流水线部署生成的提交是 `feat(<需求编号>): <需求名>`，人工提交不必模仿。
5. 打开 PR。GitHub 会套用 [默认模板](.github/PULL_REQUEST_TEMPLATE.md)；功能 / 修复 / 热修可改用 [`feature.md`](.github/PULL_REQUEST_TEMPLATE/feature.md) / [`bugfix.md`](.github/PULL_REQUEST_TEMPLATE/bugfix.md) / [`hotfix.md`](.github/PULL_REQUEST_TEMPLATE/hotfix.md)（创建 PR 时 URL 加 `?template=feature.md`、`?template=bugfix.md` 或 `?template=hotfix.md`）。写清动机，以及是否在 `poria-desktop` 窗口里测过。路径变更会由 [labeler](.github/workflows/labeler.yml) 自动打标签；[CODEOWNERS](.github/CODEOWNERS) 会请求 `@clawd-cook` 评审。

合并前本地至少：

| 改动范围 | 检查 |
|---|---|
| `src/` | `pnpm typecheck`；IPC / UI 在桌面窗口验证；`pnpm exec oxfmt .` |
| `crates/` 或 `src-tauri/` | `cargo test -p <crate>` 或 `cargo test --workspace`；涉及 Init / worktree 时再跑 `cargo test -p poria-core -- stage_order`、`cargo test -p poria-skills` |
| 格式 | 前端 oxfmt；Rust 用默认 `rustfmt`（仓库没有 `rustfmt.toml`） |

不要提交：

- `~/.poria/`、`workspace/db/`、Cookie、证书、私钥
- 仅因本地 checkout 变脏的 submodule 指针（`*-dirty`）

## 分层（不要反转依赖）

```
poria-core
  ├── poria-infrastructure
  ├── poria-resources
  └── poria-channels → poria-skills
poria-commands          # core + infrastructure
src-tauri (poria-desktop)
```

领域类型进 `poria-core`；HTTP / 行云 / JoySpace / Coding 进 `poria-channels`；Claude / git worktree 进 `poria-resources`；阶段行为进 `poria-skills`；编排与回滚进 `poria-commands`。细节以 [AGENTS.md](AGENTS.md) 为准。

EasyCI `createChange` 只能 `SELECT` 远端已存在的分支：**先 push 再绑定**。

## 发布（维护者）

打 tag 触发 GitHub Actions。当前 `pre-publish.yml` / `publish.yml` 只编 **`aarch64-apple-darwin`** DMG。有 `APPLE_*` secret 则公证签名，没有则 ad-hoc 签名并继续出包。

| Tag | Workflow |
|---|---|
| `vX.Y.Z-beta.N` | `.github/workflows/pre-publish.yml`（pre-release） |
| `vX.Y.Z` | `.github/workflows/publish.yml`（latest） |

Workflow 以 **tag 所在提交** 为准。修 YAML 后需要新 tag，对旧 tag 点 Re-run 不会用到 `main` 上的新文件。CI 会在构建时把 tag 版本写入 `package.json`、`src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml`。
