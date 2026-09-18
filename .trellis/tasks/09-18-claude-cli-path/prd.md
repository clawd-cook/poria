# Resolve Claude CLI path and show status in settings

## Goal

桌面端评审 / 设计 / 开发 / CR 必须用本机 `claude` 的绝对路径启动。设置页能看到当前路径是否可用，并允许手填覆盖。避免 macOS GUI 短 PATH 下 `spawn("claude")` 出现 `No such file or directory (os error 2)`。

## Background

需求评审曾失败：`review_prd_failed` / `Agent dispatch failed: No such file or directory (os error 2)`。Init 不走 Agent 所以能过。`ClaudeCliSdk` 默认 `Command::new("claude")`（`crates/poria-resources/src/claude/cli_sdk.rs`）；`AppState` 启动时 `ClaudeAgentPool::new_with_cli(3, None)`（`src-tauri/src/lib.rs`）。本机 CLI 在 `/opt/homebrew/bin/claude`。用户要求用终端 `which` 解析绝对路径；设置页展示链接状态；支持手动输入。链接状态只表示路径存在、可执行、`claude --version` 成功，不做账号登录探测。监视为按需：进入设置、保存、点刷新时探测，不做定时。

设置页已有表单（`src/components/SettingsPage.tsx`），保存走 `update_config`。`AppConfig` / `PoriaConfig` 没有 `claude_path`。`update_config` 写 `~/.poria/config.json`，`load_config` 只读 CWD 的 `poria.config.json`，不打通读写则手填会丢。

## Requirements

- **R1** 未配置覆盖时，用登录 shell 执行 `which claude`（与用户终端一致），拿到绝对路径再 spawn。解析失败时错误写明找不到 claude，不要只报 os error 2。
- **R2** `~/.poria/config.json` 增加可选 `claude_path`。非空则优先用该绝对路径；空则走 R1。
- **R3** 设置页展示 Claude 状态：解析到的路径、是否可执行、`claude --version` 输出或失败原因。允许手填并保存。进入设置、保存成功、点击刷新时各探测一次，无后台定时。
- **R4** `get_config` / `update_config` 必须读写同一份 `~/.poria/config.json`。
- **R5** 路径在每次 Agent spawn 时解析，配置变更后无需重启桌面端。

## Acceptance Criteria

- [ ] AC1: GUI 短 PATH 下，未手填时仍能解析到与终端 `which claude` 相同的绝对路径并成功 spawn。
- [ ] AC2: 手填有效绝对路径并保存后，下一次评审/设计/开发/CR 使用该路径。
- [ ] AC3: 手填清空并保存后，恢复自动 `which`。
- [ ] AC4: 进入设置即可看到当前路径与 `--version` 成败；刷新/保存后更新；失败有可读原因。
- [ ] AC5: 找不到二进制时，阶段失败信息包含路径/`which` 结果，而不是裸 `os error 2`。
- [ ] AC6: 重启应用后，已保存的 `claude_path` 仍在。

## Out of scope

- Claude 账号登录 / `claude auth status`。
- 安装 Claude CLI、改系统 PATH、改 nvm default。
- 后台定时探测、`PipelineWorker`、事件流历史、HITL 持久化。
