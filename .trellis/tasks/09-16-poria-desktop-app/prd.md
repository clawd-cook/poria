# Poria Desktop App (Tauri v2) — PRD

> 替代原定的 cli-commands 子任务。参考 OpenMausBot 的 UI 模式，用 Tauri v2 构建桌面端。

## Goal

构建 Poria Desktop App，让用户通过桌面应用完成 Pipeline 全生命周期操作：提交行云链接 → 实时监控 7-stage 进度 → 处理人工回路请求 → 查看 MR 状态。取代 CLI 命令层，提供更好的可视化和交互体验。

---

## 核心需求

### R1: Pipeline 提交入口

- 主界面顶部输入框，粘贴行云卡片链接
- 提交后立即创建 Pipeline → 进入队列 → sidebar 显示新条目
- 输入校验：域名白名单 + 格式检查，失败即时反馈
- 提交前 SSO 校验，未登录 → 引导登录

### R2: Pipeline 列表 + 状态一览

- 左侧 Sidebar 展示所有 Pipeline，按状态分组（running / waiting_merge / blocked / completed / failed）
- 每条 Pipeline 显示：需求名称、当前阶段、状态 badge、创建时间
- blocked Pipeline 高亮（红色 badge + 通知指示）
- 支持按状态筛选

### R3: Pipeline 详情 + 实时进度

- 7-stage 进度条（横向步骤条，已完成/运行中/待执行/失败/跳过 五种状态）
- 当前运行中 stage 的实时事件流（类似 OpenMausBot 的 ChatView）
- 每个 stage 可展开查看：输入/输出摘要、耗时、重试次数
- 门禁结果展示（通过/未通过/warn）
- MR 链接（deploy 完成后可点击跳转）

### R4: 人工回路交互

- Pipeline blocked 时，桌面端弹出通知 + 详情面板中显示 HumanLoopCard
- HumanLoopCard 显示：问题分类、详情、三个操作按钮（修复/跳过/取消）
- 用户点击后立即执行对应操作，Pipeline 恢复/跳过/取消
- 替代京ME 回复解析，提供更可靠的操作路径

### R5: Auth 管理

- 设置面板中显示当前登录状态（用户名、cookie 有效性）
- 登录/登出操作
- cookie 过期时在 Pipeline 详情中提示重新登录

### R6: 设置面板

- 门禁阈值配置（CR 评分、测试覆盖率、变更行数限制）
- 超时配置（Agent 超时、Git 超时）
- 重试次数配置
- 数据目录路径

### R7: 桌面集成

- 系统托盘：状态图标（idle/running/blocked）+ 快捷菜单
- 原生通知：Pipeline 完成/失败/blocked 时推送 OS 通知
- 单实例：防止多开
- 窗口管理：记住位置和大小

---

## 约束

### C1: 技术栈
- Tauri v2 (Rust backend) + React + Vite + Tailwind CSS
- 前端状态：useReducer + Tauri event listener (参考 OpenMausBot 模式)
- Rust 侧：rusqlite 读 SQLite（UI 数据查询）
- Node sidecar：运行 PipelineWorker（Agent SDK + channel I/O）

### C2: 复用已有包
- `@poria/core` — 类型、状态机、门禁、事件
- `@poria/commands` — PipelineExecutor, Worker, Rollback
- `@poria/skills` — 7 skill + HumanLoopCoordinator
- `@poria/infrastructure` — SQLite store, queue, recovery, auth
- `@poria/channels` — xingyun, joyspace, coding, jme
- `@poria/resources` — agent pool, output guard, terminal, worktree

### C3: 平台
- macOS (primary, Apple Silicon)
- Node.js v24.20.0 (sidecar 内嵌)

### C4: 渐进式
- MVP: Pipeline 提交/列表/详情/人工回路 + 系统通知
- P2: 设置面板完善 + 自动更新 + deep-link
- P3: Dashboard 统计 + 多仓库可视化

---

## 非目标

- Web 版本（P3+ 计划）
- Windows / Linux 支持（MVP 只做 macOS）
- 自定义主题
- i18n（MVP 中文界面）

---

## Acceptance Criteria

- [ ] Tauri v2 app 可构建可运行（`cargo tauri dev`）
- [ ] 粘贴行云链接 → Pipeline 创建 → 列表更新
- [ ] 7-stage 进度条实时更新（fixture 模式）
- [ ] Pipeline blocked → OS 通知 + HumanLoopCard 可操作
- [ ] 系统托盘 + 状态图标
- [ ] Rust 侧可读 SQLite 查询 Pipeline 列表
- [ ] Node sidecar 可启动 PipelineWorker
- [ ] 前端 typecheck + lint 通过
- [ ] Rust `cargo check` 通过
