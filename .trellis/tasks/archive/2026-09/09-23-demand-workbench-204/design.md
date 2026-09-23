# Design: 需求工作台（2.0.4）

**Delivery**: one Trellis task covers P0+P1; implement in Slices A→E (see `implement.md`). Optional split into sequential PRs for rollback, not separate Trellis children.

## Architecture

前端为主；默认不改 Rust 阶段语义。可选：仅当现有 `human:request` 载荷不够驱动导航时，再最小扩展事件字段（优先前端用 `pipelineId` + `list_pipelines` 派生）。

```text
Tauri events / invoke
        ↓
store (pipelines, selectedPipelineId, humanRequest, auth, repos, config, ui)
        ↓
derivePipelineViewModel(state)   ← 纯函数，无 React
        ↓
Shell / DemandList / Workbench chrome（mode + primaryCta + badges）
        ↓
Workbench tabs → 现有 PipelineDetail 块、文档抽屉、Workspace、HumanLoop 收敛为切面
```

## Boundaries

| Layer | Owns | Does not own |
|-------|------|--------------|
| `src/lib/pipelineViewModel.ts`（新） | mode / CTA / badges / 建议 workbenchTab | IPC、持久化 |
| `src/state/*` | 保留领域真相；扩展 `ui.workbenchTab`、`hitlAutoNavigate`、待办派生所需字段 | 各页面私有「下一步」逻辑 |
| `DemandList`（可由 HomeBoard 列表态抽出） | 发现、筛选、进入工作台 | 阶段运行 UI |
| `DemandWorkbench`（新容器） | 顶栏 + Tab + 挂载切面 | 改后端状态机 |
| `Shell` | 收敛导航项 | 工作台内部 Tab |

## Data flow

1. **进入工作台**：`pipelineSelected` 或 `submit_pipeline` 成功 → `selectedPipelineId` 有值 → Shell/Home 渲染 Workbench 而非列表。
2. **返回列表**：清「工作台展示」或单独 `ui.surface=list`，保留 `selectedPipelineId` 可选（推荐保留选中高亮，列表模式）。
3. **HITL**：`humanRequest` action → 更新角标；若 `hitlAutoNavigate` → `viewChanged(home)` + 选中 pipeline + `workbenchTab=confirm`。
4. **文档**：复用 `list_demand_project_files` / `read_demand_project_file`（现有 drawer IPC）；确认切面内编辑仍走 `write_demand_project_file`。
5. **工作区**：读 `pipelineDetail.workspace_path` / worktree；`opener` 打开目录（现有能力）。

## Compatibility / migration

- `ViewType`：移除或弃用主航道 `channels` / `skills` / `workspace`；`demands` 继续 coerce → `home`。
- 旧用户无迁移数据；仅 UI 结构变化。
- `PersistentTab`：列表与工作台可同属 `home` 视图内切换，减少跨 Tab 状态丢失。

## Trade-offs

| 选择 | 利 | 弊 |
|------|----|----|
| ViewModel 纯前端派生 | 无后端改动、可测 | 多源不一致时需纪律 |
| 工作台挂在 `home` 内 | 改动面小于新路由 | HomeBoard 需拆分，文件变大需拆组件 |
| HITL 默认自动跳转 | 满足「好用」 | 可能打断设置操作 → 需开关 |

## Rollback

- 功能开关：暂不需要后端 flag；git revert 前端 PR 即可。
- 若分 PR：先合 ViewModel（无 UI 行为变化）→ 再合 Workbench → 再合壳收敛。

## Risky files

- `src/components/Shell.tsx`, `HomeBoard.tsx`, `PipelineDetail.tsx`
- `src/state/store.tsx`, `actions.ts`, `src/lib/types.ts`
- `HumanLoopCard.tsx`, `DemandProjectDrawer.tsx`, `WorkspacePage.tsx`
