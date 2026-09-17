# Implement SSO login flow in Poria desktop app

## Goal

Poria 桌面应用目前没有登录功能，仅能读取 `~/.poria/auth.json` 显示登录状态。需要参考 h2o-plugin 的 SSO 登录流程，在 app 内实现完整的登录/登出功能。

## Background

- **当前状态**：只有 `get_auth_status` Tauri 命令，读 `~/.poria/auth.json`。无登录/登出命令，UI 只展示状态。
- **参考实现**：h2o-plugin 使用 loopback HTTP server + 外部浏览器 SSO 的方式完成登录。
- **SSO 页面**：`https://pre-ho.jd.com/plugin-auth`，接受 `state` 和 `redirect` 参数，回调 POST `{ state, erp, cookie }`。

## Requirements

### R1: Rust 后端 — login 命令
- 新增 `start_login` Tauri 命令
- 生成随机 CSRF state token
- 启动 tokio 本地 loopback HTTP server（`127.0.0.1` 随机端口，`/callback` 端点）
- 使用 `tauri-plugin-opener` 打开外部浏览器访问 SSO 页面
- 接收 SSO 回调 POST（JSON 或 form-urlencoded），验证 state，提取 `erp` + `cookie`
- 调用 `poria-infrastructure` 的 `save_credentials` 存储凭据
- 通过 Tauri event `auth:status-changed` 通知前端
- 超时自动关闭 loopback server（5 分钟）
- CORS 支持（同 h2o-plugin：`Access-Control-Allow-Private-Network`）

### R2: Rust 后端 — logout 命令
- 新增 `logout` Tauri 命令
- 调用 `poria-infrastructure` 的 `logout` 清除凭据
- 发送 `auth:status-changed` 事件通知前端

### R3: 前端 — 登录交互
- AuthStatus 组件未登录时显示「登录」按钮
- 点击调用 `start_login` Tauri 命令
- 已登录时显示用户名 + 登出选项
- 响应 `auth:status-changed` 事件更新 UI（已有监听）

### R4: 前端 — Tauri invoke 封装
- `src/lib/tauri.ts` 新增 `startLogin()` 和 `logout()` 函数

## Constraints

- Auth URL 使用 `https://pre-ho.jd.com/plugin-auth`（与 h2o-plugin 一致）
- 凭据存储复用现有 `~/.poria/auth.json` 机制
- loopback server 仅绑定 `127.0.0.1`（安全要求）
- 不引入新的 Cargo workspace 依赖，使用现有 `tokio`

## Acceptance Criteria

- [ ] 未登录状态下，AuthStatus 显示登录按钮
- [ ] 点击登录按钮，外部浏览器打开 SSO 页面
- [ ] SSO 登录完成后，app 自动切换为已登录状态，显示 ERP 用户名
- [ ] 已登录状态下可登出，状态切换为未登录
- [ ] `~/.poria/auth.json` 正确写入/删除
- [ ] loopback server 5 分钟超时自动关闭
- [ ] CSRF state 验证通过才接受凭据
- [ ] `cargo check --workspace` 通过
- [ ] `pnpm typecheck` 通过
