# Implementation Plan: SSO Login Flow

## Step 1: Extend `src-tauri/src/commands/auth.rs` — login + logout commands

### 1.1 Add `start_login` command
- Generate CSRF state (32-byte random hex via `rand`)
- Start `tokio::net::TcpListener` on `127.0.0.1:0`
- Build SSO URL with state + redirect params
- Open browser via `app.opener().open_url()`
- Accept one connection with 5-min timeout
- Parse HTTP request (OPTIONS → 204 CORS; POST `/callback` → parse body)
- Validate state, extract erp + cookie
- Respond with success HTML page
- Call `poria_infrastructure::auth::save_credentials()`
- Emit `auth:status-changed` via `app.emit()`
- Close listener

### 1.2 Add `logout` command
- Call `poria_infrastructure::auth::logout(None)`
- Emit `auth:status-changed` event with `{ logged_in: false, username: null, cookie_valid: false }`

### 1.3 Register commands in `src-tauri/src/lib.rs`
- Add `commands::auth::start_login` and `commands::auth::logout` to `invoke_handler`

### Validation
```bash
cargo check --workspace
```

## Step 2: Add `rand` dependency (if needed)

- Check if `rand` is in workspace deps; if not, add to `src-tauri/Cargo.toml`
- Used for CSRF state token generation

### Validation
```bash
cargo check -p poria
```

## Step 3: Frontend — Tauri invoke wrappers

### 3.1 `src/lib/tauri.ts`
- Add `startLogin()` → `invoke<void>("start_login")`
- Add `logout()` → `invoke<void>("logout")`

### Validation
```bash
pnpm typecheck
```

## Step 4: Frontend — AuthStatus component

### 4.1 `src/components/AuthStatus.tsx`
- Not logged in: show clickable 「登录」 text, onClick calls `startLogin()`
- Logged in: show username + 「登出」 link, onClick calls `logout()`
- Handle loading state during login (optional: "登录中..." indicator)

### Validation
```bash
pnpm typecheck
pnpm dev  # manual test
```

## Step 5: End-to-end verification

```bash
cargo check --workspace
cargo clippy --workspace
pnpm typecheck
pnpm tauri dev  # test login flow manually
```

## Review gates

- [ ] CSRF state validated before accepting credentials
- [ ] Loopback server binds only `127.0.0.1`
- [ ] 5-min timeout on loopback server
- [ ] CORS headers match h2o-plugin
- [ ] No new workspace-level Cargo deps beyond `rand`
- [ ] `cargo check --workspace` passes
- [ ] `pnpm typecheck` passes
