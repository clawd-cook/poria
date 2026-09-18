# 看板统一需求与任务 — 实现计划

## Checklist

1. **Store**：`pipeline_repo` 增加 `find_latest_for_demand`（code 优先，空 code 用 demand_id）。单测：两行同 code，返回 `updated_at` 较新的。
2. **IPC**：`list_pipelines` 去重后再序列化。`submit_pipeline` 按 design 复用/创建。单测复用不 insert。
3. **Board 数据**：`HomeBoard`（或抽出 `BoardToolbar`）在登录且看板可见时 `listDemands`；未登录不调。局部 state 存 demand page + filters。
4. **列**：D10 列；未开始混合 Xingyun 无任务卡 + `created`；其它列只用去重后的 pipelines。
5. **点击**：无 id → 向导；有 id → 详情。链接解析：已有任务则选中，否则向导。
6. **Shell**：去掉需求菜单；`demands` view 落到 `home`。把 `DemandListPage` 的登录空态/错误提示迁到未开始列，页面文件可删或改成仅被看板使用的碎片。
7. **文案**：卡片不展示 `status_label`。未开始 Xingyun 卡可用「未开始」。
8. **Spec**：更新 `tauri-desktop-testing.md` 需求入口改为看板；`AGENTS.md` 需求列表段落。

## Validation

```bash
export NVM_DIR="$HOME/.nvm"
. "/opt/homebrew/opt/nvm/nvm.sh"
nvm use 24.20.0
pnpm typecheck
cargo test -p poria-infrastructure -- pipeline
cargo test -p poria-desktop --lib -- --test-threads=1
```

Poria 窗口（不是 `:1420`）：登录后看板未开始有卡；开始后进运行中且不新增第二张同编号卡；由我受理只影响未开始；未登录不打 Xingyun。

## Risk / rollback

- `submit_pipeline` 是高风险：复用时不要把 running 的 stages 重置。
- `DemandListPage` 删除前确认向导、文档抽屉仍挂在看板。
- 出问题可只回退 command 复用，看板仍去重展示。

## Before `task.py start`

- [x] `prd.md` 收敛（无未决 Open questions）
- [x] `design.md` / `implement.md`
- [ ] 用户审过规划
- jsonl 已填 spec 路径
