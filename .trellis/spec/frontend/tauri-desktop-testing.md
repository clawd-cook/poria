# Tauri Desktop Verification

> How to exercise Poria IPC in the real desktop window. Captured 2026-09-17 after registering Coding git repos through the warehouse tab.

## Scenario: Verify Tauri invoke / clone / demand list

### 1. Scope / Trigger

Use this spec whenever the change calls `invoke`, `listen`, git clone, Xingyun, or SSO. Cursor's browser tab on Vite is not a Tauri webview.

### 2. Signatures

Launch (this machine, 2026-09-17):

```bash
export NVM_DIR="$HOME/.nvm"
. "/opt/homebrew/opt/nvm/nvm.sh"
nvm use 24.20.0
export PATH="$HOME/.nvm/versions/node/v24.20.0/bin:$HOME/.cargo/bin:$PATH"
# free Vite's strict port first
lsof -iTCP:1420 -sTCP:LISTEN
cargo tauri dev
```

- CLI binary: `~/.cargo/bin/cargo-tauri` (`tauri-cli 2.11.4`)
- Process / window: `poria-desktop` / title `Poria`
- Debug binary: `target/debug/poria-desktop`
- Do **not** use `/Applications/Poria.app` for current-branch UI tests (stale bundle)

Repo IPC (camelCase from TS):

```typescript
invoke<RegisteredRepo>("register_repo", { gitUrl });
invoke<RegisteredRepo[]>("list_repos");
invoke<RegisteredRepo>("retry_clone", { id });
```

Demand list IPC:

```typescript
invoke<DemandPage>("list_demands", { acceptedByMe, current, keyword, pageSize });
```

| `acceptedByMe` | JACP `/openapi/v3/demands/query` body | Row receiver when the record has no `receiver` object |
| --- | --- | --- |
| omitted / `false` (default) | omit `receiver` and `processor` (related-to-me via cookie / `optErp`) | `null` → UI `—`. Do not stamp the logged-in ERP |
| `true` (checkbox 「由我受理」) | `receiver` = current ERP; still omit `processor` | may fall back to the query ERP |

Flow: `DemandListPage` → `listDemands({ acceptedByMe })` → Tauri `list_demands` → `DemandListQuery.accepted_by_me`. Logged-out UI must not call this command.

Clone dest: `~/.poria/repos/<scope>/<name>` from `repo_scope_and_name_from_git_url`.

### 3. Contracts

| Surface | Has Tauri IPC? |
| --- | --- |
| `poria-desktop` WKWebView | Yes (`invoke` / `listen` / `emit`) |
| Cursor browser → `http://localhost:1420` | No. Vite only. |

`tauri.conf.json`: `devUrl` `http://localhost:1420`, `strictPort: true`, `beforeDevCommand` `pnpm dev`.

macOS AX (osascript) after granting Accessibility to the calling app:

| Control | AX role | AX name |
| --- | --- | --- |
| Home | `AXMenuItem` | `home 首页` |
| Demands | `AXMenuItem` | `unordered-list 需求列表` |
| Accepted-by-me | `AXCheckBox` | `由我受理` |
| Repos | `AXMenuItem` | `folder 仓库列表` |
| Settings | `AXMenuItem` | `setting 设置` |
| Register | `AXButton` | `登 记` (antd inserts a space) |
| Git URL | `AXTextField` | often unnamed |
| Clone status | `AXStaticText` | `进行中` / `成功` / `失败` |

Tree: `window 1` → `group 1` → web area. Search `entire contents`; do not assume `menu 1 of group 1 of window 1`.

WKWebView inputs: click the field, `⌘A`, paste clipboard. `set value` does not always fire React `onChange`.

Clone is async: UI `进行中` then `成功`/`失败`. `.git` existing is not done; wait for `git rev-parse --verify HEAD` **and** AX `成功`.

SSH: `ssh -o BatchMode=yes -T git@coding.jd.com` must authenticate before warehouse clone.

Register does **not** store frontend/backend role. Role is chosen later in the start wizard.

Known 2026-09-17 fixtures:

| Role (wizard) | URL | Path |
| --- | --- | --- |
| Backend | `git@coding.jd.com:ls/ls-entrance.git` | `~/.poria/repos/ls/ls-entrance` |
| Frontend | `git@coding.jd.com:LT_H5/fragment-clean.git` | `~/.poria/repos/LT_H5/fragment-clean` |

### 4. Validation & Error Matrix

| Condition | What you see |
| --- | --- |
| `pnpm tauri dev` without a `tauri` on PATH | `sh: tauri: command not found` (`package.json` script is `"tauri": "tauri"`; `@tauri-apps/cli` is not a workspace dep) |
| Port 1420 already used by leftover `pnpm dev` | Vite/Tauri fail (`strictPort`) |
| Cursor browser on 1420 calls `listen`/`invoke` | `TypeError: Cannot read properties of undefined (reading 'transformCallback')` in `@tauri-apps/api/core.js` |
| osascript without Accessibility | `-25211` `osascript不允许辅助访问` |
| Click `menu 1 of group 1 of window 1` | `-1719` invalid index; search by AX name instead |
| Duplicate git URL / `scope/name` | register rejected |
| Clone still packing objects | AX `进行中`, HEAD missing |
| Coding SSH not set up | clone `失败` |

### 5. Good / Base / Bad Cases

- **Good**: `cargo tauri dev` → AX click `folder 仓库列表` → paste URL → `登 记` → wait until `成功` and `HEAD` exists. Demand tab default list is larger than 「由我受理」; checking the box refetches page 1 with `receiver`.
- **Base**: layout-only check in Cursor browser on 1420 (tabs, empty states). Do not claim clone/login/submit passed.
- **Bad**: treat Cursor browser invoke failure as an app bug; treat `/Applications/Poria.app` as the current branch; or always send JACP `receiver` / stamp the logged-in ERP on every row.

### 6. Tests Required

- Unit: `cargo test -p poria-infrastructure -- registered_repo`; git URL `scope_and_name` / path-escape tests; `cargo test -p poria-channels -- demand_list` (omit `receiver`/`processor` by default; `accepted_by_me` adds `receiver` only).
- Manual E2E (this spec): two distinct ready repos, AX statuses `成功`, paths under `~/.poria/repos`.
- Assert: `pipeline.repos` stays frontend-only when later starting a pipeline (backend is `backend_context`).

### 7. Wrong vs Correct

#### Wrong

```bash
pnpm tauri dev
# then drive Cursor browser at http://localhost:1420 and click 登记
```

#### Correct

```bash
nvm use 24.20.0
export PATH="$HOME/.nvm/versions/node/v24.20.0/bin:$HOME/.cargo/bin:$PATH"
cargo tauri dev
# drive process poria-desktop (AX), not the Cursor browser tab
```

#### Wrong (demand list)

```json
{ "current": 1, "pageSize": 20, "status": [2, 13, 3, 5, 6, 7, 8, 9, 10, 11, 12], "receiver": "<erp>" }
```

Always sending `receiver` hides related-to-me rows. Stamping that ERP onto records that have no `receiver` object also lies in the 接收人 column.

#### Correct (demand list)

Default body omits `receiver` and `processor`. Checkbox 「由我受理」 / `acceptedByMe: true` is the only path that adds `"receiver": "<erp>"`. Unchecked rows with no receiver object render `—`.

## Common Mistake: Vite tab vs desktop window

**Symptom**: 1420 shows the UI but 登记/登录/需求列表 fail or the store listener throws `transformCallback`.

**Cause**: `@tauri-apps/api` needs the Tauri webview inject. Cursor's browser is a normal Chrome tab.

**Fix**: Use the `poria-desktop` window. Keep 1420 checks for CSS/layout only.

## Common Mistake: osascript menu path

**Symptom**: `-1719` cannot get `menu 1 of group 1 of window 1`.

**Cause**: WKWebView nests the antd Menu; the first group is not a native menu.

**Fix**: Iterate `entire contents of group 1 of window 1` and `click` the `AXMenuItem` whose name is `folder 仓库列表`.
