# Poria Desktop App — Implementation Plan

## 实施顺序

### Step 1: Tauri v2 脚手架

- [ ] 1.1 `apps/desktop/` 目录，`package.json` (react, vite, tailwind, @tauri-apps/api, @tauri-apps/plugin-*)
- [ ] 1.2 `vite.config.ts` + `tailwind.config.ts` + `tsconfig.json` + `index.html`
- [ ] 1.3 `src-tauri/Cargo.toml` + `tauri.conf.json` + `capabilities/default.json`
- [ ] 1.4 `src-tauri/src/main.rs` + `lib.rs` 最小可运行
- [ ] 1.5 `src/main.tsx` + `src/App.tsx` + `src/styles.css` 最小 React 入口
- [ ] 1.6 验证 `cargo tauri dev` 可启动窗口

**验证**: 空白 Tauri 窗口可弹出

### Step 2: Rust 后端 — SQLite 读取 + Commands

- [ ] 2.1 `src-tauri/src/db.rs` — rusqlite 连接 + 只读查询函数
- [ ] 2.2 `src-tauri/src/commands/pipeline.rs` — list_pipelines, get_pipeline
- [ ] 2.3 `src-tauri/src/commands/auth.rs` — get_auth_status
- [ ] 2.4 `src-tauri/src/commands/config.rs` — get_config, update_config
- [ ] 2.5 AppState 结构 + command 注册
- [ ] 2.6 `src-tauri/src/commands/mod.rs` 统一导出

**验证**: `cargo check` 通过

### Step 3: 前端 — 状态管理 + 核心组件

- [ ] 3.1 `src/state/store.tsx` — useReducer + StoreProvider + dispatch wrapper
- [ ] 3.2 `src/state/actions.ts` — Action union 类型定义
- [ ] 3.3 `src/lib/tauri.ts` — invoke/listen 封装
- [ ] 3.4 `src/lib/types.ts` — PipelineSummary, PipelineDetail, AuthStatus, AppConfig 类型
- [ ] 3.5 `src/hooks/useTauriEvents.ts` — Tauri event listener hook
- [ ] 3.6 `src/hooks/usePipeline.ts` — Pipeline 数据 hook

**验证**: TypeScript 编译通过

### Step 4: 前端 — UI 组件

- [ ] 4.1 `src/components/Shell.tsx` — 主布局 (sidebar + main area)
- [ ] 4.2 `src/components/PipelineSidebar.tsx` — Pipeline 列表 + 状态分组 + 筛选
- [ ] 4.3 `src/components/SubmitBar.tsx` — 行云链接输入 + 提交
- [ ] 4.4 `src/components/PipelineDetail.tsx` — Pipeline 详情容器
- [ ] 4.5 `src/components/StageProgress.tsx` — 7-stage 横向步骤条
- [ ] 4.6 `src/components/EventStream.tsx` — 实时事件流展示
- [ ] 4.7 `src/components/HumanLoopCard.tsx` — 人工回路操作卡片
- [ ] 4.8 `src/components/GateResults.tsx` — 门禁结果展示
- [ ] 4.9 `src/components/StatusBadge.tsx` — 状态 badge 组件
- [ ] 4.10 `src/components/AuthStatus.tsx` — 登录状态
- [ ] 4.11 `src/components/SettingsPanel.tsx` — 设置面板
- [ ] 4.12 `src/App.tsx` 组装所有组件

**验证**: `pnpm typecheck` 通过，UI 可渲染

### Step 5: Rust 后端 — Sidecar + 事件桥接 + 系统托盘

- [ ] 5.1 `src-tauri/src/sidecar.rs` — Node sidecar 生命周期管理
- [ ] 5.2 `src-tauri/src/events.rs` — sidecar stdout → Tauri event 桥接
- [ ] 5.3 `src-tauri/src/tray.rs` — 系统托盘 + 状态图标 + 菜单
- [ ] 5.4 `src-tauri/src/commands/pipeline.rs` — submit_pipeline, human_loop_respond, cancel, rollback (委托 sidecar)
- [ ] 5.5 通知集成 (tauri-plugin-notification)

**验证**: `cargo check` 通过，系统托盘显示

### Step 6: Sidecar 构建 + 集成验证

- [ ] 6.1 Node sidecar 入口脚本 (packages/commands 的 daemon 模式)
- [ ] 6.2 tauri.conf.json `externalBin` 配置
- [ ] 6.3 `cargo tauri dev` 启动完整应用
- [ ] 6.4 fixture 模式端到端验证

**验证**: 完整应用可运行，fixture 模式下可提交/查看/操作 Pipeline

## 验收标准

- [ ] `cargo tauri dev` 可启动完整应用
- [ ] 粘贴行云链接 → Pipeline 创建 → 列表更新 (fixture 模式)
- [ ] 7-stage 进度条展示正确
- [ ] HumanLoopCard 可操作（fixture 模式）
- [ ] 系统托盘 + 通知工作
- [ ] Rust `cargo check` + 前端 `pnpm typecheck` 均通过
