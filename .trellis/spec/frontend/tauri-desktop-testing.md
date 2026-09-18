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

Demand list IPC (看板未开始列, camelCase from TS):

```typescript
invoke<DemandPage>("list_demands", { acceptedByMe, current, keyword, pageSize });
```

| `acceptedByMe` | JACP `/openapi/v3/demands/query` body | Records with no `receiver` object |
| --- | --- | --- |
| omitted / `false` (default) | omit `receiver` and `processor` (related-to-me via cookie / `optErp`) | Do not stamp the logged-in ERP |
| `true` (checkbox 「由我受理」) | `receiver` = current ERP; still omit `processor` | may fall back to the query ERP |

Flow: `HomeBoard` (logged in + 看板 visible) → `listDemands({ acceptedByMe })` → Tauri `list_demands` → `DemandListQuery.accepted_by_me`. Logged-out 看板 must not call this command. 「由我受理」 only changes the Xingyun query for the 未开始 column; local pipeline columns are unchanged. Cards do not render Xingyun `status_label`.

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
| Board | `AXMenuItem` | `home 看板` |
| Accepted-by-me | `AXCheckBox` | `由我受理` |
| Keyword search | `AXTextField` | `搜索任务名称或编号` |
| Link start | `AXTextField` | `粘贴行云需求链接` |
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
| ReviewPrd `os error 2` / Settings Claude probe | See [Claude CLI Path](./claude-cli.md); GUI PATH often lacks Homebrew `claude` |
| osascript without Accessibility | `-25211` `osascript不允许辅助访问` |
| Click `menu 1 of group 1 of window 1` | `-1719` invalid index; search by AX name instead |
| Duplicate git URL / `scope/name` | register rejected |
| Clone still packing objects | AX `进行中`, HEAD missing |
| Coding SSH not set up | clone `失败` |

### 5. Good / Base / Bad Cases

- **Good**: `cargo tauri dev` → AX click `folder 仓库列表` → paste URL → `登 记` → wait until `成功` and `HEAD` exists. 看板 default 未开始 (related-to-me) is larger than 「由我受理」; checking the box refetches page 1 with `receiver` and does not move local task cards.
- **Base**: layout-only check in Cursor browser on 1420 (tabs, empty states). Do not claim clone/login/submit passed.
- **Bad**: treat Cursor browser invoke failure as an app bug; treat `/Applications/Poria.app` as the current branch; or always send JACP `receiver` / stamp the logged-in ERP on every row.

### 6. Tests Required

- Unit: `cargo test -p poria-infrastructure -- pipeline`; git URL `scope_and_name` / path-escape tests; `cargo test -p poria-channels -- demand_list` (omit `receiver`/`processor` by default; `accepted_by_me` adds `receiver` only); `cargo test -p poria-desktop --lib -- --test-threads=1` (submit reuse + list_pipelines dedupe).
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

Default body omits `receiver` and `processor`. Checkbox 「由我受理」 / `acceptedByMe: true` is the only path that adds `"receiver": "<erp>"`. Do not stamp the logged-in ERP onto Xingyun records that have no `receiver` object.

## Scenario: Start pipeline with backend TRD URL

### 1. Scope / Trigger

Use this spec when changing the 看板 unstarted-card wizard, `submit_pipeline`, `PipelineConfig.backend_trd_url`, or `gen_trd` / `gen_code` prompt injection. Cursor's browser tab on Vite cannot invoke submit. Clicking a card with no pipeline opens the wizard; a card with a pipeline id opens detail. `resolve_demand_link` selects the existing pipeline when that demand key already has a task.

### 2. Signatures

```typescript
invoke<string>("submit_pipeline", {
  demandId,          // number
  frontendRepoId,    // registered ready repo id
  backendRepoId,     // different ready repo id
  backendBranch,     // local git branch after fetch
  prdUrl,            // JoySpace
  backendTrdUrl,     // JoySpace, required, must differ from prdUrl
  demandCode?,
  demandName?,
});
```

Wizard last step fields (AX):

| Control | AX role | AX name |
| --- | --- | --- |
| PRD URL | `AXTextField` | `JoySpace PRD` |
| Backend TRD URL | `AXTextField` | `JoySpace 后端 TRD` |
| Submit | `AXButton` | `创 建 流 水 线` (antd may insert spaces) |

Rust persistence (snake_case in `pipelines.config` JSON):

```ts
{
  repos: [/* frontend RepoConfig only */],
  prd_url?: string,
  backend_trd_url?: string,  // JoySpace; FE coding aid; not frontend TRD.md
  backend_context?: { git_url, local_path, branch, scope, name }
}
```

Short `claude -p` names `gen-trd` / `gen-code` and includes demand code, workspace root, frontend/backend dirs, and a non-empty `backend_trd_url`. Procedure lives in bundled `SKILL.md`. Agents read optional `BACKEND_TRD.md` via the workspace symlink.

### 3. Contracts

| Layer | Owns |
| --- | --- |
| Wizard | Both URLs required + distinct; PRD prefill via `preview_demand_prd`; backend TRD is paste-only |
| `submit_pipeline` | Persist `config.backend_trd_url`; `pipeline.repos` length 1 (frontend). Frontend `base_branch` = registered `default_branch`. Backend lives in `backend_context` only. Reuse the latest row for the demand key: `Created` updates config; any other status returns the existing id without resetting stages |
| Feature artifacts | Frontend design is `TRD.md`. Optional export is `BACKEND_TRD.md`. Never overwrite `TRD.md` with backend TRD |
| `gen_trd` / `gen_code` | Workspace-root `claude -p` short prompt + bundled `SKILL.md`; read optional `BACKEND_TRD.md`; do not modify the backend repo |

JoySpace live Markdown export is Init: SSO Cookie + `POST /v1/pages/content`, files land in `~/.poria/projects/<demand_code>/` (`PRD.md`, `BACKEND_TRD.md`, later `PRD_REVIEW.md` / `TRD.md`). 看板卡片 「文档」 reads that folder. Init then syncs hosted clones and creates `~/.poria/workspaces/<pipeline_id>/` (doc + skill symlinks, frontend feature + backend detached worktrees) before ReviewPrd. After Init, `backend_context.local_path` is the backend worktree. ReviewPrd/Design/Dev/Cr `claude -p` cwd is the workspace root. A non-empty `backend_trd_url` is still injected so later agents can open the link. See [Pipeline Init Workspace](./pipeline-workspace.md).

### 4. Validation & Error Matrix

| Condition | What you see |
| --- | --- |
| Empty / non-JoySpace PRD | Wizard submit disabled; command `请填写 JoySpace PRD 链接` / `PRD 必须是 JoySpace 链接` |
| Empty backend TRD | Wizard submit disabled; command `请填写 JoySpace 后端 TRD 链接` |
| Backend TRD not JoySpace | `后端 TRD 必须是 JoySpace 链接` |
| `backendTrdUrl` equals `prdUrl` (after trim) | `后端 TRD 不能与 PRD 使用相同链接` |
| Frontend == backend repo | `前端仓库与后端仓库不能相同` |
| Missing ready clone | `前端仓库尚未克隆完成` / `后端仓库尚未克隆完成` |

### 5. Good / Base / Bad Cases

- **Good**: two distinct ready repos → wizard step 文档 → paste different JoySpace PRD and backend TRD → 创建流水线 → home kanban selects the new id. SQLite config has `prd_url` + `backend_trd_url` + `backend_context`; `repos` is only the frontend. Design/dev prompts include the backend URL and `禁止修改后端仓`.
- **Base**: layout-only in Cursor browser: last wizard step shows both URL fields; submit stays disabled until both are distinct JoySpace URLs.
- **Bad**: put the backend repo into `pipeline.repos`; write backend TRD into `TRD.md`; omit `backendTrdUrl` from invoke; treat Vite-tab submit as proof.

### 6. Tests Required

- Unit: `cargo test -p poria-core -- pipeline_config`; `cargo test -p poria-core -- feature_context`; `cargo test -p poria-skills -- backend_aid`; `cargo test -p poria-desktop -- require_prd_and_backend_trd`.
- Manual E2E (this spec): drive `poria-desktop`, not Cursor browser on 1420. Confirm no backend MR / backend feature branch.

### 7. Wrong vs Correct

#### Wrong

```typescript
invoke("submit_pipeline", { prdUrl, frontendRepoId, backendRepoId, backendBranch });
// backendTrdUrl missing; or repos: [frontend, backend]
```

```text
write backend TRD into features/<id>/TRD.md
```

#### Correct

```typescript
invoke("submit_pipeline", {
  prdUrl: "https://joyspace.jd.com/pages/prd",
  backendTrdUrl: "https://joyspace.jd.com/pages/backend-trd",
  frontendRepoId,
  backendRepoId,
  backendBranch,
  demandId,
});
// config.backend_trd_url set; pipeline.repos = [frontend]
// optional artifact BACKEND_TRD.md; frontend TRD.md unchanged
```

## Scenario: Init exports JoySpace markdown

### 1. Scope / Trigger

Use when changing Init, JoySpace `POST /v1/pages/content`, or 看板 「文档」. Verify in `poria-desktop`, not Vite `:1420`.

### 2. Signatures

```typescript
invoke("execute_stage", { pipelineId });
invoke<DemandProject>("list_demand_project", { demandCode, demandId? });
invoke<DemandProjectFileContent>("read_demand_project_file", { demandCode, fileName, demandId? });
```

Auth: `~/.poria/auth.json` Cookie + `x-team-id: 00046419`. Files: `~/.poria/projects/<demand_code>/PRD.md` and `BACKEND_TRD.md`.

### 3. Tests Required

- Unit: `cargo test -p poria-channels -- joyspace`; `cargo test -p poria-infrastructure -- demand_project`; `cargo test -p poria-core -- feature_context`.
- Manual: pipeline Init 执行 on R2026082156824, then 看板 「文档」 shows exported markdown.

## Scenario: SSO cookie expires mid-pipeline

### 1. Scope / Trigger

Use when changing Xingyun / JoySpace / Coding 401 handling, `ensure_sso_credentials`, `HumanLoopCard`, or login-after-block resume. Verify in `poria-desktop`, not Vite `:1420`.

### 2. Signatures

401 / `请先登录` / `登录已过期` classify as `IssueClass::AuthExpired`. Desktop `fail_or_block_stage` sets pipeline + stage `blocked` with `issue.class = auth_expired` (does not cancel or delete worktrees). Init/Deploy probe Xingyun (`probe_sso`) before mutating git remotes. Silent refresh only re-reads `~/.poria/auth.json` when the cookie bytes changed; JD SSO is not minted locally.

```typescript
invoke("start_login");
listen("auth:status-changed", ...); // cookie_valid: false on block; true after callback
invoke("human_loop_respond", { pipelineId, action: "resume" });
```

After SSO callback, `resume_auth_blocked_after_login` enqueues every `blocked` pipeline whose stage issue is auth-expired.

### 3. Contracts

| Event | Pipeline | Stage | UI |
| --- | --- | --- | --- |
| 401 during Init/Deploy (or missing cookie) | `blocked` | `blocked` | HumanLoopCard 「重新登录」; sidebar cookie warning |
| Login succeeds | `running` (resume current stage) | `running` | card dismissed on `pipeline:updated` |
| User cancels | `cancelled` | unchanged / skipped | card dismissed |

Do **not** `Failed` an auth miss after `git push` / EasyCI bind; resume retries Deploy (push is idempotent; bind/MR are find-or-create).

### 4. Tests Required

- Unit: `cargo test -p poria-commands -- classify`; `cargo test -p poria-commands -- auth_expired`; `cargo test -p poria-commands -- stage_error_outcome`; `cargo test -p poria-infrastructure -- ensure_valid`; `cargo test -p poria-core -- pending_to_blocked`.
- Manual: start a pipeline, expire/remove cookie, confirm Deploy/Init hangs as 阻塞 not 已失败; log in; stage continues.

## Common Mistake: Vite tab vs desktop window

**Symptom**: 1420 shows the UI but 登记/登录/需求列表 fail or the store listener throws `transformCallback`.

**Cause**: `@tauri-apps/api` needs the Tauri webview inject. Cursor's browser is a normal Chrome tab.

**Fix**: Use the `poria-desktop` window. Keep 1420 checks for CSS/layout only.

## Common Mistake: osascript menu path

**Symptom**: `-1719` cannot get `menu 1 of group 1 of window 1`.

**Cause**: WKWebView nests the antd Menu; the first group is not a native menu.

**Fix**: Iterate `entire contents of group 1 of window 1` and `click` the `AXMenuItem` whose name is `folder 仓库列表`.
