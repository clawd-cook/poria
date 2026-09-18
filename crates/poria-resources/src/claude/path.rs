use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::process::Command;
use tracing::{debug, warn};

const VERSION_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClaudePathSource {
    Which,
    Config,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedClaudePath {
    pub path: String,
    pub source: ClaudePathSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeProbeResult {
    pub ok: bool,
    pub resolved_path: Option<String>,
    pub version: Option<String>,
    pub error: Option<String>,
    pub source: Option<ClaudePathSource>,
}

/// Resolve `claude` for spawn / probe.
///
/// 1. Non-empty `path_override` must be an existing absolute executable.
/// 2. Otherwise run `$SHELL -lc 'which claude'` (SHELL empty → `/bin/zsh`) and
///    take the first stdout line that starts with `/`.
pub fn resolve_claude_path(path_override: Option<&str>) -> Result<ResolvedClaudePath, String> {
    if let Some(override_path) = normalize_override(path_override) {
        let path = PathBuf::from(&override_path);
        if !path.is_absolute() {
            return Err(format!("claude 路径必须是绝对路径，收到: {override_path}"));
        }
        validate_claude_binary(&path)?;
        return Ok(ResolvedClaudePath {
            path: override_path,
            source: ClaudePathSource::Config,
        });
    }

    let (stdout, stderr, status) = run_which_claude()?;
    match parse_which_stdout(&stdout) {
        Some(path) => {
            validate_claude_binary(Path::new(&path)).map_err(|e| {
                format!("{e}（which 输出: stdout={}, stderr={}, status={status}）", stdout.trim(), stderr.trim())
            })?;
            Ok(ResolvedClaudePath {
                path,
                source: ClaudePathSource::Which,
            })
        }
        None => Err(format!(
            "找不到 claude。请安装 Claude CLI 或在设置中填写绝对路径。which 结果: status={status}, stdout={}, stderr={}",
            stdout.trim(),
            stderr.trim()
        )),
    }
}

pub fn parse_which_stdout(stdout: &str) -> Option<String> {
    stdout
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with('/'))
        .map(str::to_string)
}

pub async fn probe_claude_cli(path_override: Option<&str>) -> ClaudeProbeResult {
    let source_hint = normalize_override(path_override)
        .map(|_| ClaudePathSource::Config)
        .or(Some(ClaudePathSource::Which));

    match resolve_claude_path(path_override) {
        Ok(resolved) => match claude_version(&resolved.path).await {
            Ok(version) => ClaudeProbeResult {
                ok: true,
                resolved_path: Some(resolved.path),
                version: Some(version),
                error: None,
                source: Some(resolved.source),
            },
            Err(error) => ClaudeProbeResult {
                ok: false,
                resolved_path: Some(resolved.path),
                version: None,
                error: Some(error),
                source: Some(resolved.source),
            },
        },
        Err(error) => ClaudeProbeResult {
            ok: false,
            resolved_path: None,
            version: None,
            error: Some(error),
            source: source_hint,
        },
    }
}

fn normalize_override(path_override: Option<&str>) -> Option<String> {
    path_override
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

const WHICH_CLAUDE_LOGIN_ARGS: [&str; 2] = ["-lc", "which claude"];

fn login_shell() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into())
}

fn run_which_claude() -> Result<(String, String, String), String> {
    let shell = login_shell();
    let output = std::process::Command::new(&shell)
        .args(WHICH_CLAUDE_LOGIN_ARGS)
        .output()
        .map_err(|e| format!("无法执行 `{shell} -lc 'which claude'` 解析 claude 路径: {e}"))?;

    Ok((
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
        output.status.to_string(),
    ))
}

fn validate_claude_binary(path: &Path) -> Result<(), String> {
    let meta = std::fs::metadata(path)
        .map_err(|e| format!("找不到 claude 可执行文件 {}: {e}", path.display()))?;
    if !meta.is_file() {
        return Err(format!("claude 路径不是可执行文件: {}", path.display()));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if meta.permissions().mode() & 0o111 == 0 {
            return Err(format!("claude 路径不可执行: {}", path.display()));
        }
    }
    Ok(())
}

async fn claude_version(path: &str) -> Result<String, String> {
    debug!(path, "probing claude --version");
    let mut cmd = Command::new(path);
    cmd.arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    match tokio::time::timeout(VERSION_TIMEOUT, cmd.output()).await {
        Err(_) => Err(format!(
            "claude --version 超时（{}s），路径: {path}",
            VERSION_TIMEOUT.as_secs()
        )),
        Ok(Err(e)) => Err(format!("无法执行 `{path} --version`: {e}")),
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let text = stdout.trim();
            if output.status.success() {
                if text.is_empty() {
                    Ok(stderr.trim().to_string())
                } else {
                    Ok(text.to_string())
                }
            } else {
                warn!(path, status = %output.status, "claude --version failed");
                Err(format!(
                    "`{path} --version` 失败: status={}, stdout={}, stderr={}",
                    output.status,
                    text,
                    stderr.trim()
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    fn make_executable(dir: &tempfile::TempDir, name: &str) -> PathBuf {
        let path = dir.path().join(name);
        fs::write(&path, "#!/bin/sh\necho ok\n").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    #[test]
    fn test_parse_which_stdout_first_absolute_line() {
        assert_eq!(
            parse_which_stdout("/opt/homebrew/bin/claude\n"),
            Some("/opt/homebrew/bin/claude".into())
        );
        assert_eq!(parse_which_stdout("claude not found\n"), None);
        assert_eq!(
            parse_which_stdout("alias hint\n/usr/local/bin/claude\n/opt/bin/claude\n"),
            Some("/usr/local/bin/claude".into())
        );
        assert_eq!(
            parse_which_stdout("  /bin/claude  \n"),
            Some("/bin/claude".into())
        );
    }

    #[test]
    fn test_resolve_claude_path_absolute_override() {
        let dir = tempfile::tempdir().unwrap();
        let bin = make_executable(&dir, "claude");
        let resolved = resolve_claude_path(Some(bin.to_str().unwrap())).unwrap();
        assert_eq!(resolved.path, bin.to_str().unwrap());
        assert_eq!(resolved.source, ClaudePathSource::Config);
    }

    #[test]
    fn test_resolve_claude_path_rejects_relative() {
        let err = resolve_claude_path(Some("claude")).unwrap_err();
        assert!(err.contains("绝对路径"), "{err}");
        let err = resolve_claude_path(Some("./claude")).unwrap_err();
        assert!(err.contains("绝对路径"), "{err}");
    }

    #[test]
    fn test_resolve_claude_path_missing_absolute() {
        let err = resolve_claude_path(Some("/tmp/poria-missing-claude-bin")).unwrap_err();
        assert!(err.contains("找不到 claude"), "{err}");
    }

    #[test]
    fn test_resolve_claude_path_blank_override_uses_which_parser_contract() {
        assert_eq!(normalize_override(Some("")), None);
        assert_eq!(normalize_override(Some("  ")), None);
        assert_eq!(normalize_override(None), None);
    }

    #[test]
    fn test_which_uses_login_shell_without_interactive_flag() {
        assert_eq!(WHICH_CLAUDE_LOGIN_ARGS, ["-lc", "which claude"]);
        assert!(
            !WHICH_CLAUDE_LOGIN_ARGS.contains(&"-i"),
            "must not use interactive -i (oh-my-zsh can hang)"
        );
    }

    #[test]
    fn test_probe_result_uses_camel_case_and_lowercase_source() {
        let json = serde_json::to_value(ClaudeProbeResult {
            ok: true,
            resolved_path: Some("/opt/homebrew/bin/claude".into()),
            version: Some("2.0.0".into()),
            error: None,
            source: Some(ClaudePathSource::Which),
        })
        .unwrap();
        assert_eq!(json["resolvedPath"], "/opt/homebrew/bin/claude");
        assert_eq!(json["source"], "which");
        assert!(json.get("resolved_path").is_none());
    }

    #[tokio::test]
    async fn test_probe_claude_cli_reports_version_for_override() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("claude");
        fs::write(&path, "#!/bin/sh\necho 'claude 2.0.0 (test)'\n").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        let bin = path.to_str().unwrap();

        let result = probe_claude_cli(Some(bin)).await;
        assert!(result.ok, "{result:?}");
        assert_eq!(result.resolved_path.as_deref(), Some(bin));
        assert_eq!(result.source, Some(ClaudePathSource::Config));
        assert!(
            result
                .version
                .as_deref()
                .is_some_and(|v| v.contains("2.0.0")),
            "{result:?}"
        );
        assert_eq!(result.error, None);
    }
}
