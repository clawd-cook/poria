use poria_infrastructure::auth;
use serde::Serialize;
use std::time::Duration;
use tauri::Emitter;
use tauri_plugin_opener::OpenerExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[derive(Debug, Serialize, Clone)]
pub struct AuthStatusView {
    pub logged_in: bool,
    pub username: Option<String>,
    pub cookie_valid: bool,
}

const AUTH_URL: &str = "https://pre-ho.jd.com/plugin-auth";
const LOGIN_TIMEOUT_SECS: u64 = 300;

const SUCCESS_HTML: &str = r#"<!DOCTYPE html>
<html lang="zh-CN">
<head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Poria 授权</title></head>
<body style="margin:0;min-height:100vh;display:flex;align-items:center;justify-content:center;background:#111;color:#f4f4f4;font-family:-apple-system,BlinkMacSystemFont,sans-serif;">
<div style="width:min(360px,calc(100% - 48px));padding:32px 24px;text-align:center;">
<h1 style="margin:0 0 8px;font-size:22px;">Poria</h1>
<p style="margin:0;font-size:14px;line-height:1.6;opacity:.82;">授权已回传到应用，可以关闭此页。</p>
</div>
</body>
</html>"#;

fn generate_state() -> String {
    hex::encode(rand::random::<[u8; 16]>())
}

fn build_cors_headers() -> String {
    "Access-Control-Allow-Origin: *\r\n\
     Access-Control-Allow-Methods: POST, OPTIONS\r\n\
     Access-Control-Allow-Headers: Content-Type\r\n\
     Access-Control-Allow-Private-Network: true\r\n"
        .to_string()
}

struct CallbackPayload {
    state: String,
    erp: String,
    cookie: String,
}

fn parse_json_payload(body: &str) -> Option<CallbackPayload> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    Some(CallbackPayload {
        state: v["state"].as_str().unwrap_or("").to_string(),
        erp: v["erp"].as_str().unwrap_or("").to_string(),
        cookie: v["cookie"].as_str().unwrap_or("").to_string(),
    })
}

fn parse_form_payload(body: &str) -> CallbackPayload {
    let params: Vec<(String, String)> = url::form_urlencoded::parse(body.as_bytes())
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    let get = |key: &str| -> String {
        params
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
            .unwrap_or_default()
    };
    CallbackPayload {
        state: get("state"),
        erp: get("erp"),
        cookie: get("cookie"),
    }
}

fn parse_payload(content_type: &str, body: &str) -> CallbackPayload {
    let ct = content_type.split(';').next().unwrap_or("").trim();
    if ct == "application/json" {
        parse_json_payload(body).unwrap_or_else(|| parse_form_payload(body))
    } else {
        parse_form_payload(body)
    }
}

fn http_response(
    status: u16,
    status_text: &str,
    cors: &str,
    content_type: &str,
    body: &str,
) -> Vec<u8> {
    format!(
        "HTTP/1.1 {} {}\r\n{}\
         Content-Type: {}\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\r\n{}",
        status,
        status_text,
        cors,
        content_type,
        body.len(),
        body,
    )
    .into_bytes()
}

fn http_empty(status: u16, status_text: &str, cors: &str) -> Vec<u8> {
    format!(
        "HTTP/1.1 {} {}\r\n{}\
         Content-Length: 0\r\n\
         Connection: close\r\n\r\n",
        status, status_text, cors,
    )
    .into_bytes()
}

#[tauri::command]
pub async fn get_auth_status() -> Result<AuthStatusView, String> {
    let status = auth::get_status(None);
    Ok(AuthStatusView {
        logged_in: status.logged_in,
        username: status.username,
        cookie_valid: status.logged_in,
    })
}

#[tauri::command]
pub async fn start_login(app: tauri::AppHandle) -> Result<(), String> {
    let state = generate_state();
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| format!("Failed to bind loopback server: {e}"))?;
    let port = listener
        .local_addr()
        .map_err(|e| format!("Failed to get local address: {e}"))?
        .port();

    let redirect_url = format!("http://127.0.0.1:{port}/callback");
    let sso_url = format!(
        "{AUTH_URL}?state={state}&redirect={}",
        urlencoding::encode(&redirect_url)
    );

    app.opener()
        .open_url(&sso_url, None::<&str>)
        .map_err(|e| format!("Failed to open browser: {e}"))?;

    let expected_state = state;
    let app_handle = app.clone();

    tokio::spawn(async move {
        let result = tokio::time::timeout(Duration::from_secs(LOGIN_TIMEOUT_SECS), async {
            loop {
                let (mut stream, _) = match listener.accept().await {
                    Ok(conn) => conn,
                    Err(_) => continue,
                };

                let mut buf = vec![0u8; 8192];
                let n = match stream.read(&mut buf).await {
                    Ok(n) if n > 0 => n,
                    _ => continue,
                };
                let raw = String::from_utf8_lossy(&buf[..n]);

                let first_line = raw.lines().next().unwrap_or("");
                let cors = build_cors_headers();

                if first_line.starts_with("OPTIONS") {
                    let resp = http_empty(204, "No Content", &cors);
                    let _ = stream.write_all(&resp).await;
                    continue;
                }

                if !first_line.contains("/callback") || !first_line.starts_with("POST") {
                    let resp = http_empty(404, "Not Found", &cors);
                    let _ = stream.write_all(&resp).await;
                    continue;
                }

                let content_type = raw
                    .lines()
                    .find(|l| l.to_lowercase().starts_with("content-type:"))
                    .map(|l| l.split_once(':').map(|(_, v)| v.trim()).unwrap_or(""))
                    .unwrap_or("");

                let body = raw.split_once("\r\n\r\n").map(|(_, b)| b).unwrap_or("");
                let payload = parse_payload(content_type, body);

                if payload.state != expected_state {
                    let resp = http_response(
                        400,
                        "Bad Request",
                        &cors,
                        "text/html; charset=utf-8",
                        "<!DOCTYPE html><html><body><p>授权状态不匹配</p></body></html>",
                    );
                    let _ = stream.write_all(&resp).await;
                    continue;
                }

                if payload.erp.is_empty() || payload.cookie.is_empty() {
                    let resp = http_response(
                        400,
                        "Bad Request",
                        &cors,
                        "text/html; charset=utf-8",
                        "<!DOCTYPE html><html><body><p>回调缺少凭据</p></body></html>",
                    );
                    let _ = stream.write_all(&resp).await;
                    continue;
                }

                let resp =
                    http_response(200, "OK", &cors, "text/html; charset=utf-8", SUCCESS_HTML);
                let _ = stream.write_all(&resp).await;

                let creds = auth::JacpCredentials {
                    username: payload.erp,
                    cookie: payload.cookie,
                };

                if let Err(e) = auth::save_credentials(&creds, None) {
                    tracing::error!("Failed to save credentials: {e}");
                    return;
                }

                let status_view = AuthStatusView {
                    logged_in: true,
                    username: Some(creds.username),
                    cookie_valid: true,
                };
                let _ = app_handle.emit("auth:status-changed", status_view);
                return;
            }
        })
        .await;

        if result.is_err() {
            tracing::warn!("Login loopback server timed out after {LOGIN_TIMEOUT_SECS}s");
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn logout(app: tauri::AppHandle) -> Result<(), String> {
    auth::logout(None)?;
    let status_view = AuthStatusView {
        logged_in: false,
        username: None,
        cookie_valid: false,
    };
    app.emit("auth:status-changed", status_view)
        .map_err(|e| format!("Failed to emit event: {e}"))?;
    Ok(())
}
