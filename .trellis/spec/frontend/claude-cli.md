# Claude CLI Path and Settings Probe

> Resolve the local `claude` binary for Agent stages. Captured 2026-09-18 after ReviewPrd failed with `No such file or directory (os error 2)` under a macOS GUI PATH.

## 1. Scope / Trigger

Use this spec when changing Agent spawn, `PoriaConfig`, settings IPC, or anything that reads `claude_path`.

macOS GUI / `Poria.app` PATH often lacks Homebrew. Do **not** `Command::new("claude")` and hope `PATH` contains it. Do **not** probe or save from the Cursor browser tab on `:1420`.

## 2. Signatures

Rust:

- `poria_infrastructure::config::load_config` → `~/.poria/config.json` first; CWD `poria.config.json` only if the user file is missing
- `get_config_file_path(user_root: Option<&Path>) -> PathBuf` → `{user_root|~/.poria}/config.json`
- `PoriaConfig.claude_path: Option<String>` (`#[serde(default)]`; blank/whitespace = unset)
- `PoriaConfig::effective_claude_path() -> Option<&str>`
- `resolve_claude_path(path_override: Option<&str>) -> Result<ResolvedClaudePath, String>`
- `probe_claude_cli(path_override: Option<&str>) -> ClaudeProbeResult`
- Tauri: `get_config`, `update_config`, `probe_claude(pathOverride?: string | null)`

Frontend (`src/lib/tauri.ts`): `getConfig`, `updateConfig`, `probeClaude(pathOverride?: string | null)`.

## 3. Contracts

`AppConfig.claude_path`: `string | null`. Empty string from the form is saved as unset (auto `which`).

`ClaudeProbeResult` (camelCase JSON):

| Field | Type | Meaning |
| --- | --- | --- |
| `ok` | boolean | `--version` succeeded |
| `resolvedPath` | string \| null | Absolute path after resolve |
| `version` | string \| null | `--version` stdout (or stderr if stdout empty) |
| `error` | string \| null | Human-readable failure |
| `source` | `"which"` \| `"config"` \| null | How the path was chosen |

Resolve order:

1. Non-empty override → must be an existing absolute executable (relative rejected).
2. Else `$SHELL -lc 'which claude'` (`SHELL` empty → `/bin/zsh`). First stdout line starting with `/`.
3. No `-i`. No Homebrew/nvm silent fallback.

`--version` timeout: 5s. Settings probes on **enter Settings tab**, **after save**, **Refresh** — no timer. Settings stays mounted in `PersistentTab`; gate probe on `view === "settings"`.

Agent stages (ReviewPrd / Design / Dev / Cr) call `resolve_claude_path` on every spawn via `ClaudeAgentPool::new_with_path_provider` reading `load_config(None).effective_claude_path()`.

## 4. Validation & Error Matrix

| Condition | What you see |
| --- | --- |
| GUI short PATH, field empty, `which` finds Homebrew claude | Probe `ok`, `source: which`, spawn uses that absolute path |
| Relative override `claude` / `./claude` | Error: 路径必须是绝对路径 |
| Missing absolute file | Error contains 找不到 claude |
| `which` prints nothing useful | Stage/probe error includes `which` stdout/stderr/status, not a bare os error 2 |
| Cursor browser on 1420 calls `probeClaude` | `transformCallback` TypeError — not an app bug |

## 5. Good / Base / Bad Cases

- **Good**: Poria window → Settings → status matches terminal `which claude`; save a valid absolute path; next ReviewPrd uses it; clear field → auto `which` again.
- **Base**: Old `~/.poria/config.json` without `claude_path` deserializes as `None` and auto-resolves.
- **Bad**: `spawn("claude")` on Finder-launched app; probe only at app start while Settings is `display: none`; `get_config` reading CWD `poria.config.json` while `update_config` writes `~/.poria/config.json`.

## 6. Tests Required

- `cargo test -p poria-infrastructure -- config` — missing field default, empty string unset, user file wins over CWD
- `cargo test -p poria-resources -- claude` — parse `which` stdout, reject relative, absolute override, missing file
- Manual: `cargo tauri dev`, Poria window Settings vs terminal `which claude` (not Vite `:1420`)

## 7. Wrong vs Correct

#### Wrong

```rust
Command::new("claude") // GUI PATH often has no Homebrew
```

`load_config` reads `poria.config.json` in CWD; `update_config` writes `~/.poria/config.json`.

Probe Claude in `useEffect([])` on a `PersistentTab` that stays mounted hidden.

#### Correct

```rust
let resolved = resolve_claude_path(override)?;
Command::new(&resolved.path)
```

`get_config` / `update_config` / Agent provider all use `get_config_file_path` → `~/.poria/config.json`.

Probe when `state.ui.view === "settings"`, after save, and on Refresh.
