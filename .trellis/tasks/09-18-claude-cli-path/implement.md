# Claude CLI path — implement

## Checklist

1. `poria-infrastructure`：`claude_path` 字段；`get_config_file_path`；`load_config` 读 `~/.poria/config.json`。单测：缺字段 default、空字符串当未配置。
2. `poria-resources`：`resolve_claude_path` + `probe_claude_cli`；`ClaudeCliSdk::query` 用绝对路径 spawn；失败信息带路径/`which` 输出。单测：override 绝对路径、相对路径拒绝、解析 `which` stdout。
3. `src-tauri`：`AppConfig` 增加 `claude_path`；`update_config` 写回该字段；注册 `probe_claude`。
4. 前端：`types` / `tauri.ts` / `SettingsPage`（输入、状态、刷新；进入设置与保存后探测）。
5. `pnpm typecheck`；`cargo test -p poria-infrastructure -- config`；`cargo test -p poria-resources -- claude`。桌面窗口：设置页刷新、清空路径、短 PATH 场景用手填。

## Validation

```bash
export NVM_DIR="$HOME/.nvm"
. "/opt/homebrew/opt/nvm/nvm.sh"
nvm use 24.20.0
pnpm typecheck
cargo test -p poria-infrastructure -- config
cargo test -p poria-resources -- claude
cargo fmt -p poria-desktop -p poria-resources -p poria-infrastructure
```

桌面：`cargo tauri dev`，在 **Poria** 窗口打开设置，确认状态与终端 `which claude` 一致；不要用浏览器 1420。

## Rollback

还原 `claude_path` 字段与 spawn 逻辑；用户可删 `~/.poria/config.json` 里的 `claude_path`。

## Risky files

- `crates/poria-infrastructure/src/config.rs` — 改加载路径会影响所有 `get_config`
- `crates/poria-resources/src/claude/cli_sdk.rs` — 所有 Agent 阶段
- `src-tauri/src/commands/config.rs` — 设置保存
- `src/components/SettingsPage.tsx`
