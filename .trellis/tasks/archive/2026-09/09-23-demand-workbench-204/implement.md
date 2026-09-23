# Implement plan: 09-23-demand-workbench-204

**Scope decision**: single task = product PRD **P0+P1** (Slices A–E). No parent/child split. P2 polish optional after AC.

Do **not** run `task.py start` until user approves these planning artifacts.

## Ordered checklist

### Slice A — ViewModel + store 扩展（低风险）

1. [x] 新增 `src/lib/pipelineViewModel.ts`
2. [x] 扩展 `ui`：`workbenchTab`, `hitlAutoNavigate`, `surface`
3. [x] 修正 `pipelineSelected` / HITL 保留策略
4. [x] `pnpm typecheck`

### Slice B — 列表 / 工作台分裂 + 顶栏 CTA

5. [x] `DemandWorkbench` + HomeBoard surface 切换
6. [x] 顶栏 ViewModel 主 CTA
7. [x] 启动成功进工作台轨迹
8. [ ] 桌面窗手测启动路径（待用户 / 本机 `cargo tauri dev`）

### Slice C — HITL 注意力

9. [x] 侧栏角标 + 列表「待我处理」
10. [x] `human:request` → 可选自动确认切面
11. [x] 设置「HITL 自动跳转」+ localStorage
12. [ ] 桌面窗 HITL 手测

### Slice D — 文档 / 工作区切面

13. [x] 文档 Tab（`DemandProjectBrowser`）
14. [x] 工作区 Tab；Shell 去掉独立 workspace
15. [x] 空态

### Slice E — 壳 IA + 清理

16. [x] 侧栏仅 需求 / 仓库 / 设置
17. [x] 删除 `PipelineSidebar` / `useTauriEvents`
18. [x] `ARCHITECTURE.md` / `AGENTS.md` 侧栏描述
19. [x] `pnpm typecheck` + oxfmt；桌面验收 AC 待手测

## Validation

```bash
export NVM_DIR="$HOME/.nvm"
. "/opt/homebrew/opt/nvm/nvm.sh"
nvm use 24.20.0
pnpm typecheck
pnpm exec oxfmt .
# 桌面：cargo tauri dev — 非 :1420 浏览器
```

## Risky rollback points

- Slice B 合入前：仅 ViewModel 可单独留存。
- Slice E 合入前：旧 Tab 仍可用作逃生舱。

## Follow-up before `task.py start`

- [x] 切片策略：单任务 P0+P1（已写入 prd）
- [x] `implement.jsonl` / `check.jsonl` 已 validate
- [ ] 用户确认 `prd.md` / `design.md` / `implement.md` 后执行 `task.py start`
