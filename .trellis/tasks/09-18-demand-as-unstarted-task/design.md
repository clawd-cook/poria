# 看板统一需求与任务 — 设计

## Boundaries

| Layer | Change |
| --- | --- |
| `poria-infrastructure` `pipeline_repo` | `find_latest_for_demand(code, id)`；`list_all` 仍返回全表供内部用；给 UI 的列表在 command 层去重 |
| `poria-desktop` `submit_pipeline` / `list_pipelines` | 复用最新行；`list_pipelines` 按任务键去重后再给前端 |
| Frontend `HomeBoard` | 合并未开始 + 本地列；吃掉 `DemandListPage` 的查询/向导入口 |
| `Shell` | 去掉「需求」菜单项 |
| SQLite schema | **不改**。不 UNIQUE、不删旧行 |

内部类型仍叫 pipeline。产品文案用任务/看板。

## Task key

```text
key = demand_code.trim() nonempty ? demand_code : format!("id:{demand_id}")
```

Xingyun 无 code 的行用 `id:{id}`，避免和真实 code 碰撞。

## Data flow

```text
Xingyun list_demands (related-to-me | 由我受理)
        → DemandListItem[] (page)
SQLite list_all
        → pick latest per key (updated_at, then id)
        → BoardTask[] (have pipeline_id + PipelineStatus)

Board:
  未开始 = demands whose key not in local map  ∪  local status==created
  运行中/阻塞/待合并/完成/失败/已取消 = local map by status
          (created 只出现在未开始，不重复)
```

Click:

```text
no pipeline_id → StartPipelineWizard(demand)
has pipeline_id → pipelineSelected(id) → PipelineDetail
```

`resolveDemandLink` 仍回到向导；若该键已有任务则直接 `pipelineSelected`，不打开向导。

## submit_pipeline reuse

1. Compute key from args.
2. `find_latest_for_demand`.
3. None → 现有创建路径（`create_pipeline_id` + insert + emit created）。
4. Some(p) if `Created` → 更新 config（仓、`prd_url`、`backend_trd_url`、`backend_context`）、`updated_at`，返回 `p.id`。
5. Some(p) otherwise → 原样返回 `p.id`，不改 status/stages。

Wizard 成功后的前端行为不变：`listPipelines` + 选中 id。因为去重，看板卡片数不增加。

## list_pipelines contract

返回 `PipelineSummary[]`，同一 key 只留 `updated_at` 最大的一条。前端看板本地列只信这份列表。

未开始列的 Xingyun 卡 **不** 塞进 `state.pipelines`，只放在 `HomeBoard` 局部 state，避免和 hydrate 打架。

## Compatibility

- 旧库多行：去重后可见最新；旧 workspaces 仍在磁盘，只能从那条 pipeline id 进（UI 不再列出）。
- `ViewType` `"demands"`：菜单删除；进入时重定向 `home`。
- 过滤/SSO/`backendTrdUrl` 契约不改。

## Trade-offs

- 未开始只覆盖当前 Xingyun 页（默认 20 条），不是全量。换搜索词会换一页未开始。接受，避免全量同步。
- 不加 UNIQUE：提交并发仍可能插两条；list 去重仍只显示一条。可接受。
- 终态复用打开详情而不是重跑：避免误开第二条流水线。

## Rollback

还原 `Shell` 需求入口和 `DemandListPage`；`submit_pipeline` 去掉 find 分支。SQLite 无需迁移回滚。
