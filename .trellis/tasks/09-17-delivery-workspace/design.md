# Design: Delivery workspace

## Boundaries

| Surface | Owns | Does not own |
| --- | --- | --- |
| React shell | Four tabs, kanban, demand table, repo table, settings page, start wizard | SSO protocol, 7-stage skill implementations |
| Tauri commands | IPC for repos, demands, submit payload | Direct JACP HTTP (goes through channels) |
| `poria-infrastructure` | `registered_repos` SQLite + clone status | Git protocol details |
| `poria-channels` Xingyun | `listDemands` + existing `resolvePrdLink` | Clone, worktree |
| `poria-channels` git_url | `repo_search_path_from_git_url` | Filesystem |
| `poria-resources` terminal | `git clone` / `git fetch` / `git checkout` | Persistence |
| `09-17-sso-login` | `start_login` / `logout` / `auth.json` | Demand UI |

## Information architecture

```
Header: [首页] [需求列表] [仓库列表] [设置]          [AuthStatus]
Home:   Kanban columns → click card → PipelineDetail + back
Demand: Table + search + 开始 → modal wizard
Repos:  Grouped by scope + register URL
Settings: Existing config form (no longer a Modal overlay on Pipeline)
```

`ViewType`: `"home" | "demands" | "repos" | "settings"`. Remove `"pipeline" | "skills" | "channels"` from nav. Keep `PipelineDetail` mounted under Home.

Auth stays in the header on every tab so 需求列表 can prompt login without opening 设置.

## Data flow

### Register repo

```
UI git URL
  → parse scope/name via repo_search_path_from_git_url
  → reject if path missing or duplicate normalized URL / scope/name
  → insert registered_repos (status=cloning)
  → git clone into ~/.poria/repos/{scope}/{name}
  → status=ready | failed + error
  → emit repo:updated
```

### Demand list

```
SSO cookie + ERP
  → Xingyun listDemands { receiver: erp, keyword, current, pageSize }
  → table
  → 开始: getDemand + resolvePrdLink (best-effort prefill)
```

### Start pipeline

```
demand + frontendRepoId + backendRepoId + backendBranch + prdUrl
  → validate: both repos ready, ids different, prdUrl non-empty, backendBranch non-empty
  → build Pipeline:
       repos: [frontend RepoConfig] only
       config.backend_context: { git_url, local_path, branch, scope, name }
       config.prd_url: JoySpace URL
       raw_link: Xingyun view URL from demandId/code
       operator: ERP
  → persist + emit pipeline:created
  → switch view to home, select new id
```

Frontend `RepoConfig.base_branch` = current checkout of the managed clone (`git rev-parse --abbrev-ref HEAD`). Feature branch name remains workspace-skill responsibility.

Backend branch list: local `git branch -r` / `git branch -a` on the managed clone after fetch. EasyCI `listBranches` is fallback if local refs are empty, not the primary path.

## Persistence

New table (schema_version 2):

```sql
CREATE TABLE IF NOT EXISTS registered_repos (
  id TEXT PRIMARY KEY,
  git_url TEXT NOT NULL,
  normalized_url TEXT NOT NULL UNIQUE,
  scope TEXT NOT NULL,
  name TEXT NOT NULL,
  local_path TEXT NOT NULL,
  clone_status TEXT NOT NULL, -- cloning | ready | failed
  error TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  UNIQUE (scope, name)
);
```

`PipelineConfig` JSON (additive, stored in `pipelines.config`):

```ts
{
  gates: GateRule[],
  trd_scope: string[],
  repos: RepoConfig[],          // length 1: frontend
  prd_url?: string,
  backend_context?: {
    git_url: string,
    local_path: string,
    branch: string,
    scope: string,
    name: string
  }
}
```

Do not append backend to `repos`. Executor uses `pipeline.repos.len() > 1` for multi-repo delivery.

## IPC (Tauri)

| Command | Role |
| --- | --- |
| `register_repo` / `list_repos` / `retry_clone` | Inventory + clone |
| `list_repo_branches` | Backend branch picker |
| `list_demands` | Assigned-to-me page |
| `preview_demand_prd` | Wizard prefill via `resolvePrdLink` |
| `submit_pipeline` | Replace link-only payload with structured input |

Keep `list_pipelines` / `get_pipeline` / human-loop / config / auth commands.

Events: `repo:updated`, existing `pipeline:created` / `pipeline:updated` / `auth:status-changed`.

## Frontend composition

- `Shell`: four tabs + header `AuthStatus`
- `HomeBoard`: antd columns from `pipelines`; selected id shows `PipelineDetail`
- `DemandListPage`: table; wizard is antd `Modal` + `Steps` (FE → BE+branch → PRD)
- `RepoListPage`: register input + grouped list + retry
- `SettingsPage`: current `SettingsPanel` form without Modal chrome

Pipeline/skills/channels pages are unused in nav; do not delete skill/channel commands this round.

## Compatibility

- Existing pipelines with empty `repos` still list on the kanban
- Old `submit_pipeline(link)` is removed from UI; command may accept structured input only
- SSO task must land `start_login` before demand list is usable; repo tab does not require login unless clone needs Coding credentials (SSH keys on the machine are enough for `git@coding.jd.com`)

## Trade-offs

- Managed clone vs reuse daily working copy: isolation over convenience (PRD).
- Backend required: simpler wizard and better FE codegen context; no skip path.
- Local git branches vs EasyCI: works offline after clone; may miss unfetched remotes — `git fetch` on opening the BE step.

## Rollback

- Drop `registered_repos` via reversing schema_version 2; leave cloned dirs on disk (do not auto-delete `~/.poria/repos`)
- `PipelineConfig` extra keys are ignored by old readers if they only deserialize `gates` / `trd_scope` / `repos` with `deny_unknown_fields` off (current serde default)
