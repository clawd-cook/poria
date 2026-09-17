# Research: demand list + repo path

## Demand list

Poria Xingyun channel (`crates/poria-channels/src/xingyun`) exposes `getDemand`, card attachments, `resolvePrdLink`, communicate, accept. It does **not** list demands.

h2o-plugin `demands.list` (`submodules/h2o-plugin/skills/qingyan/reference.md`):

| Field | Default | Meaning |
| --- | --- | --- |
| `assignedToMe` | `true` | Receiver = current ERP |
| `keyword` | | Title / code search |
| `current` | `1` | Page |
| `pageSize` | `20` | Page size |
| `receiver` | logged-in ERP | Override receiver |

Implementation sketch in `submodules/h2o-plugin/src/agent/actions.ts` `listDemands`: `queryDemands(credentials, { current, pageSize, keyword, receiver })` with `receiver` defaulting to `credentials.username` when `assignedToMe` is true.

2026-09-17 live JACP `/openapi/v3/demands/query` (cookie + `optErp`, in-progress statuses):

| Body | Meaning | Sample total |
| --- | --- | --- |
| omit `receiver` / `processor` | Related-to-me (workbench default) | 28 |
| `receiver: erp` | Assigned to me (h2o `assignedToMe`) | 18 |
| `processor: erp` | I am processor (subset of receiver) | 16 |

Unknown extra fields (`relatedErp`, `searchType`, …) are ignored; totals stay at the omit-receiver set. List records have `id/demandCode/name/status/statusName` and typically **no** `receiver` object.

UI: default omit receiver; checkbox 「由我受理」 sends `receiver`. Do not invent receiver as the current ERP unless that filter is on.

PRD prefill: existing `resolvePrdLink` / `resolve_prd_from_attachments` on the selected demand.

## Repo clone path

`repo_search_path_from_git_url("git@coding.jd.com:ls/ls-entrance.git")` → `"ls/ls-entrance"`.

Managed clone: `~/.poria/repos/{scope}/{repo}` = `~/.poria/repos/` + that path.

Do not put the backend repo into `pipeline.repos`. `poria-commands` executor treats `pipeline.repos.len() > 1` as multi-repo delivery.
