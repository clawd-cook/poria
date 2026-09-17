# 贡献指南

感谢参与 Poria。产品说明见 [README.md](README.md)；给编码代理的分层与契约见 [AGENTS.md](AGENTS.md)。参与即表示同意 [行为准则](CODE_OF_CONDUCT.md)。

## 报告问题

- **安全漏洞**：不要开公开 Issue。按 [SECURITY.md](SECURITY.md) 私下披露。
- **缺陷**：先搜现有 Issue，再用 [缺陷报告表单](.github/ISSUE_TEMPLATE/bug_report.yml)。写清 Poria / Node / pnpm / Rust 版本、复现步骤、期望与实际结果。登录、需求列表、克隆、提交流水线必须在 **Poria 桌面窗口**里复现，不要用浏览器打开 `http://localhost:1420`。
- **功能建议**：用 [功能建议表单](.github/ISSUE_TEMPLATE/feature_request.yml)。较大改动请先开 Issue 再写代码。
- **使用或开发疑问**：用 [问题表单](.github/ISSUE_TEMPLATE/question.yml)。空白 Issue 已关闭，请不要绕过模板。

不要在 Issue 或 PR 里粘贴 SSO Cookie、`~/.poria/auth.json`、Apple / Tauri 签名密钥。

## 开发环境

本地只使用：

- Node.js **24.20.0**（nvm，不要用 Homebrew Node）
- pnpm **11.23.0**
- Rust **stable**
- macOS（当前发布产物是 Apple Silicon / Intel DMG）
- 跑流水线 Agent 阶段时需要本机 `claude` CLI

```bash
export NVM_DIR="$HOME/.nvm"
[ -s "/opt/homebrew/opt/nvm/nvm.sh" ] && . "/opt/homebrew/opt/nvm/nvm.sh"
nvm use 24.20.0

node -v    # 必须是 v24.20.0
pnpm -v    # 必须是 11.23.0

pnpm install
```

不要用 `npm` / `yarn` / `bun` 装依赖，也不要改 `package.json` 里的 `packageManager`。

更完整的命令见 README「快速开始」。

## 日常开发

```bash
pnpm tauri dev     # 打开桌面窗口（进程 poria-desktop，标题 Poria）
pnpm typecheck
pnpm exec oxfmt .
cargo check --workspace
cargo test --workspace
```

> [!IMPORTANT]
> Cursor / 浏览器打开 Vite（`:1420`）**没有** Tauri `invoke`。验证登录、需求、仓库克隆、提交流水线必须用 `pnpm tauri dev` 起的窗口。端口 1420 被占用时先释放。

IPC 封装只放在 `src/lib/tauri.ts`。前端参数用 camelCase（`pipelineId`、`backendTrdUrl`），对应 Rust snake_case。`#[tauri::command]` 只写在 `src-tauri/src/commands/`。

改某一层之前读 `.trellis/spec/` 对应索引。跨层字段（需求筛选、`backendTrdUrl`、前后端 TRD）先看 `.trellis/spec/guides/cross-layer-thinking-guide.md`。

## 提交与 Pull Request

1. 从最新 `main` 开分支。
2. 只改与问题相关的文件。不要改 `submodules/` 里的内容；实现应写在本仓库的 crate / `src/`。
3. Rust 改动补 `#[cfg(test)]` / `#[tokio::test]`。前端没有测试框架，至少 `pnpm typecheck`，涉及 IPC 时在桌面窗口走一遍主路径。
4. 提交说明写「为什么」，一两句即可。无强制前缀。流水线部署生成的提交是 `feat(<需求编号>): <需求名>`，人工提交不必模仿。
5. 打开 PR。GitHub 会套用 [默认模板](.github/PULL_REQUEST_TEMPLATE.md)；功能 / 修复可改用 [`feature.md`](.github/PULL_REQUEST_TEMPLATE/feature.md) / [`bugfix.md`](.github/PULL_REQUEST_TEMPLATE/bugfix.md)（创建 PR 时 URL 加 `?template=feature.md` 或 `?template=bugfix.md`）。写清动机，以及是否在 `poria-desktop` 窗口里测过。路径变更会由 [labeler](.github/workflows/labeler.yml) 自动打标签；[CODEOWNERS](.github/CODEOWNERS) 会请求 `@clawd-cook` 评审。

合并前本地至少：

| 改动范围 | 检查 |
|---|---|
| `src/` | `pnpm typecheck`；IPC / UI 在桌面窗口验证 |
| `crates/` 或 `src-tauri/` | `cargo test -p <crate>` 或 `cargo test --workspace` |
| 格式 | `pnpm exec oxfmt .`（前端）；Rust 用默认 `rustfmt` |

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

打 tag 触发 GitHub Actions，产物为 macOS `aarch64` + `x86_64` DMG。

| Tag | Workflow |
|---|---|
| `vX.Y.Z-beta.N` | `.github/workflows/pre-publish.yml`（pre-release） |
| `vX.Y.Z` | `.github/workflows/publish.yml`（latest） |

Workflow 以 **tag 所在提交** 为准。修 YAML 后需要新 tag，对旧 tag 点 Re-run 不会用到 `main` 上的新文件。
