# Claude CLI path — design

## Boundaries

| Layer | Owns |
| --- | --- |
| `poria-resources` | `which` 解析、`--version` 探测、spawn 用绝对路径 |
| `poria-infrastructure` | `PoriaConfig.claude_path` + 读写 `~/.poria/config.json` |
| `src-tauri` commands | `get_config` / `update_config` 带上 `claude_path`；`probe_claude` 给设置页 |
| Settings UI | 手填路径、状态展示、刷新；不自己 spawn |

Agent 阶段（ReviewPrd / Design / Dev / Cr）继续走现有 `ClaudeAgentPool`，不在 skill 里各自 `which`。

## Resolve order

1. 配置里 `claude_path` 去空白后非空 → 必须是已有绝对路径，否则错误（禁止相对路径）。
2. 否则跑登录 shell：`$SHELL -lc 'which claude'`，`SHELL` 空则 `/bin/zsh`。取 stdout 中第一条以 `/` 开头的行。
3. 不用 `-i`，避免 oh-my-zsh 交互卡住。不做 Homebrew/nvm 静默兜底，避免和终端 `which` 结果不一致。

探测与 spawn 共用同一套 resolve。`claude --version` 用解析后的绝对路径执行，超时建议 ≤ 5s。

## Config

`PoriaConfig` 增加 `#[serde(default)] claude_path: Option<String>`。空字符串按未配置处理。

`load_config` 优先读 `get_user_root() / config.json`（与 `update_config`、AGENTS.md 一致）。CWD `poria.config.json` 仅作测试/覆盖层，且不得盖掉已加载的用户文件里的 `claude_path` unless 测试需要 — 推荐：用户文件为主；CWD 文件仅当用户文件不存在时使用，保留现有单测对 default 的假设。

`get_config_file_path` 放在 `poria-infrastructure`，与 `get_auth_file_path` 并列。

## Runtime

`ClaudeCliSdk` 在每次 `query` 时 resolve，不把路径冻在 `new_with_cli(None)`。`lib.rs` 可继续 `new_with_cli(3, None)`。

`probe_claude(path_override: Option<String>)` 返回：

```json
{
  "ok": true,
  "resolvedPath": "/opt/homebrew/bin/claude",
  "version": "2.x.x ...",
  "error": null,
  "source": "which" | "config"
}
```

`path_override` 为设置页当前输入（可尚未保存），便于保存前预检。

## Frontend

`AppConfig.claude_path: string | null`。设置页增加：

- Input：Claude 路径，placeholder 可为最近一次自动解析结果
- 状态：ok / 失败原因 + 解析路径 + version
- 刷新按钮；`useEffect` 在设置页有 `config` 后探测一次

IPC 封装在 `src/lib/tauri.ts`。验证只在 **Poria 窗口**，不要用浏览器 1420。

## Compatibility / rollback

- 旧 `config.json` 无字段 → serde default `None` → 自动 `which`。
- 回滚：清空 `claude_path` 即回自动解析；代码回退则恢复 `spawn("claude")` 的短 PATH 问题。

## Trade-offs

| 选择 | 原因 |
| --- | --- |
| 每次 spawn 解析 | 保存后不用重启；`which` 成本远低于一次 Agent |
| 登录 shell `which`，无静默 PATH 拼接 | 与用户明确要求一致 |
| 不探测登录态 | 命令不稳定，且修不了 os error 2 |
