# Design: SSO Login Flow

## Architecture

```
┌─────────────┐     invoke      ┌──────────────────┐     HTTP POST     ┌─────────────┐
│  React UI   │ ──────────────> │  Tauri command    │ <───────────────  │  SSO Page   │
│  AuthStatus │                 │  start_login      │    /callback      │  (browser)  │
│  component  │ <── event ───── │                   │                   │             │
│             │  auth:status-   │  loopback server  │ ── open URL ────> │             │
└─────────────┘   changed       └──────────────────┘                   └─────────────┘
                                         │
                                         ▼
                                ┌──────────────────┐
                                │  poria-infra      │
                                │  save_credentials │
                                │  ~/.poria/auth.json│
                                └──────────────────┘
```

## Backend Design (Rust)

### New module: `src-tauri/src/commands/auth.rs`

扩展现有 auth commands 模块，新增：

#### `start_login` command

```rust
#[tauri::command]
pub async fn start_login(app: tauri::AppHandle) -> Result<(), String>
```

流程：
1. 生成 32 字节随机 hex string 作为 CSRF state
2. 启动 `tokio::net::TcpListener` 绑定 `127.0.0.1:0`
3. 获取实际端口，构造 redirect URL：`http://127.0.0.1:{port}/callback`
4. 拼接 SSO URL：`https://pre-ho.jd.com/plugin-auth?state={state}&redirect={redirect_url}`
5. 调用 `app.opener().open_url(sso_url)` 打开浏览器
6. 异步等待 HTTP 回调（最多 5 分钟超时）
7. 收到 POST `/callback`：
   - 解析 body（支持 JSON 和 form-urlencoded）
   - 验证 `state` 匹配
   - 提取 `erp`（username）和 `cookie`
   - 响应 200 + 成功 HTML 页面
8. 调用 `save_credentials` 写入 `~/.poria/auth.json`
9. 发送 `auth:status-changed` Tauri event
10. 关闭 listener

#### `logout` command

```rust
#[tauri::command]
pub async fn logout(app: tauri::AppHandle) -> Result<(), String>
```

流程：
1. 调用 `auth::logout(None)` 删除 `auth.json`
2. 发送 `auth:status-changed` 事件（`{ logged_in: false, username: null, cookie_valid: false }`）

### HTTP callback 处理

直接用 `tokio::net::TcpListener` + 手动解析 HTTP，避免引入额外的 web 框架依赖。只需处理：
- `OPTIONS /callback` → 204 with CORS headers
- `POST /callback` → 解析 body、验证、返回 HTML
- 其他 → 404

CORS headers（与 h2o-plugin 一致）：
```
Access-Control-Allow-Origin: *
Access-Control-Allow-Methods: POST, OPTIONS
Access-Control-Allow-Headers: Content-Type
Access-Control-Allow-Private-Network: true
```

### 超时机制

使用 `tokio::time::timeout(Duration::from_secs(300), ...)` 包裹 accept 循环。超时后自动关闭 listener，不报错（用户可能关闭了浏览器）。

## Frontend Design

### AuthStatus 组件改造

未登录时渲染可点击的「登录」文字/链接，点击调用 `startLogin()`。已登录时渲染用户名，hover 或旁边显示登出入口。

### Tauri 封装新增

```typescript
export async function startLogin(): Promise<void> {
  return invoke<void>("start_login");
}

export async function logout(): Promise<void> {
  return invoke<void>("logout");
}
```

### 状态更新

前端已有 `auth:status-changed` 事件监听（`store.tsx:213`），无需改动。后端发事件即可。

## Compatibility

- `save_credentials` / `get_credentials` / `logout` 均已存在于 `poria-infrastructure`，直接复用
- `auth:status-changed` 事件前端已监听，后端只需 emit
- 不影响现有 CLI 登录路径（CLI 可以继续直接写 `auth.json`）
