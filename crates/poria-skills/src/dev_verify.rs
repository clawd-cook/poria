use std::path::Path;

use serde_json::Value;
use tracing::info;

use poria_resources::{terminal, TerminalExecInput};

const VERIFY_TIMEOUT_MS: u64 = 600_000;
const OUTPUT_TAIL_CHARS: usize = 4_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VerifyKind {
    Typecheck,
    Test,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VerifyCommand {
    pub command: String,
    pub kind: VerifyKind,
}

/// Run convention or Settings commands in the frontend worktree after OutputGuard.
///
/// Empty / missing override → discover from `package.json` + lockfile.
/// Fixture mode never reaches this (GenCode returns earlier).
pub async fn run_frontend_verify(
    worktree: &Path,
    override_commands: Option<&str>,
) -> Result<(), String> {
    let commands = match override_commands.map(str::trim).filter(|s| !s.is_empty()) {
        Some(raw) => parse_override_commands(raw),
        None => discover_verify_commands(worktree)?,
    };
    if commands.is_empty() {
        info!(
            worktree = %worktree.display(),
            "no local verify commands; skipping"
        );
        return Ok(());
    }

    let cwd = worktree
        .to_str()
        .ok_or_else(|| "LocalVerifyError: tsc worktree path is not valid UTF-8".to_string())?;

    for item in commands {
        run_one(cwd, &item).await?;
    }
    Ok(())
}

pub(crate) fn parse_override_commands(raw: &str) -> Vec<VerifyCommand> {
    raw.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| VerifyCommand {
            command: line.to_string(),
            kind: classify_command(line),
        })
        .collect()
}

pub(crate) fn discover_verify_commands(worktree: &Path) -> Result<Vec<VerifyCommand>, String> {
    let package_path = worktree.join("package.json");
    if !package_path.is_file() {
        return Ok(Vec::new());
    }
    let content = std::fs::read_to_string(&package_path).map_err(|e| {
        format!("LocalVerifyError: tsc failed to read package.json: {e}。不得进入 CR。")
    })?;
    let pkg: Value = serde_json::from_str(&content).map_err(|e| {
        format!("LocalVerifyError: tsc package.json is invalid JSON: {e}。不得进入 CR。")
    })?;
    Ok(commands_from_package(
        &pkg,
        detect_package_manager(worktree),
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PackageManager {
    Bun,
    Npm,
    Pnpm,
    Yarn,
}

pub(crate) fn detect_package_manager(worktree: &Path) -> PackageManager {
    if worktree.join("pnpm-lock.yaml").is_file() {
        PackageManager::Pnpm
    } else if worktree.join("yarn.lock").is_file() {
        PackageManager::Yarn
    } else if worktree.join("bun.lockb").is_file() || worktree.join("bun.lock").is_file() {
        PackageManager::Bun
    } else if worktree.join("package-lock.json").is_file() {
        PackageManager::Npm
    } else {
        PackageManager::Pnpm
    }
}

pub(crate) fn commands_from_package(pkg: &Value, pm: PackageManager) -> Vec<VerifyCommand> {
    let scripts = pkg.get("scripts").and_then(Value::as_object);
    let mut commands = Vec::new();

    if script_value(scripts, "typecheck").is_some() {
        commands.push(VerifyCommand {
            command: pm_run(pm, "typecheck"),
            kind: VerifyKind::Typecheck,
        });
    } else if has_dep(pkg, "typescript") {
        commands.push(VerifyCommand {
            command: pm_exec(pm, "tsc --noEmit"),
            kind: VerifyKind::Typecheck,
        });
    }

    if let Some(script) = script_value(scripts, "test:run") {
        if !is_watch_script(script) {
            commands.push(VerifyCommand {
                command: pm_run(pm, "test:run"),
                kind: VerifyKind::Test,
            });
        }
    } else if let Some(script) = script_value(scripts, "test:ci") {
        if !is_watch_script(script) {
            commands.push(VerifyCommand {
                command: pm_run(pm, "test:ci"),
                kind: VerifyKind::Test,
            });
        }
    } else if let Some(script) = script_value(scripts, "test") {
        if !is_watch_script(script) {
            commands.push(VerifyCommand {
                command: pm_run(pm, "test"),
                kind: VerifyKind::Test,
            });
        } else if has_dep(pkg, "vitest") {
            commands.push(VerifyCommand {
                command: pm_exec(pm, "vitest run"),
                kind: VerifyKind::Test,
            });
        }
    } else if has_dep(pkg, "vitest") {
        commands.push(VerifyCommand {
            command: pm_exec(pm, "vitest run"),
            kind: VerifyKind::Test,
        });
    }

    commands
}

fn script_value<'a>(
    scripts: Option<&'a serde_json::Map<String, Value>>,
    name: &str,
) -> Option<&'a str> {
    scripts
        .and_then(|map| map.get(name))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn has_dep(pkg: &Value, name: &str) -> bool {
    ["dependencies", "devDependencies", "peerDependencies"]
        .iter()
        .any(|key| pkg.get(key).and_then(|deps| deps.get(name)).is_some())
}

pub(crate) fn is_watch_script(script: &str) -> bool {
    let lower = script.to_ascii_lowercase();
    if lower.contains("--watch=false")
        || lower.contains("--watch false")
        || lower.contains("--no-watch")
    {
        return false;
    }
    if lower.contains("--watch") {
        return true;
    }
    let trimmed = lower.trim();
    trimmed == "vitest"
        || (trimmed.starts_with("vitest ")
            && !trimmed.contains(" run")
            && !trimmed.contains("--run"))
}

fn classify_command(command: &str) -> VerifyKind {
    let lower = command.to_ascii_lowercase();
    if lower.contains("typecheck") || lower.contains("tsc") {
        VerifyKind::Typecheck
    } else if lower.contains("test") || lower.contains("vitest") || lower.contains("jest") {
        VerifyKind::Test
    } else {
        VerifyKind::Typecheck
    }
}

fn pm_run(pm: PackageManager, script: &str) -> String {
    match pm {
        PackageManager::Pnpm => format!("pnpm {script}"),
        PackageManager::Yarn => format!("yarn {script}"),
        PackageManager::Npm => format!("npm run {script}"),
        PackageManager::Bun => format!("bun run {script}"),
    }
}

fn pm_exec(pm: PackageManager, rest: &str) -> String {
    match pm {
        PackageManager::Pnpm => format!("pnpm exec {rest}"),
        PackageManager::Yarn => format!("yarn {rest}"),
        PackageManager::Npm => format!("npx {rest}"),
        PackageManager::Bun => format!("bunx {rest}"),
    }
}

fn wrap_desktop_path(command: &str) -> String {
    format!(
        "export NVM_DIR=\"${{NVM_DIR:-$HOME/.nvm}}\"; \
         [ -s \"$NVM_DIR/nvm.sh\" ] && . \"$NVM_DIR/nvm.sh\"; \
         [ -s /opt/homebrew/opt/nvm/nvm.sh ] && . /opt/homebrew/opt/nvm/nvm.sh; \
         export PATH=\"/opt/homebrew/bin:$HOME/.local/share/pnpm:$PATH\"; \
         {command}"
    )
}

async fn run_one(cwd: &str, item: &VerifyCommand) -> Result<(), String> {
    info!(command = %item.command, cwd, "running Dev local verify");
    let result = terminal::exec(TerminalExecInput {
        command: wrap_desktop_path(&item.command),
        cwd: Some(cwd.to_string()),
        env: None,
        timeout_ms: Some(VERIFY_TIMEOUT_MS),
    })
    .await
    .map_err(|err| format_resource_error(item, &err.to_string()))?;

    if result.code == 0 {
        return Ok(());
    }
    Err(format_verify_error(
        item.kind,
        &item.command,
        result.code,
        &result.stdout,
        &result.stderr,
    ))
}

fn format_resource_error(item: &VerifyCommand, detail: &str) -> String {
    match item.kind {
        VerifyKind::Typecheck => format!(
            "LocalVerifyError: tsc failed to run `{}`: {detail}。不得进入 CR。",
            item.command
        ),
        VerifyKind::Test => format!(
            "LocalVerifyError: test fail running `{}`: {detail}。不得进入 CR。",
            item.command
        ),
    }
}

fn format_verify_error(
    kind: VerifyKind,
    command: &str,
    code: i32,
    stdout: &str,
    stderr: &str,
) -> String {
    let tail = output_tail(stdout, stderr);
    match kind {
        VerifyKind::Typecheck => format!(
            "LocalVerifyError: tsc/typecheck failed (`{command}`, exit {code})。不得进入 CR。\n{tail}"
        ),
        VerifyKind::Test => {
            format!("LocalVerifyError: test fail (`{command}`, exit {code})。不得进入 CR。\n{tail}")
        }
    }
}

fn output_tail(stdout: &str, stderr: &str) -> String {
    let combined = format!("{}\n{}", stdout, stderr);
    let trimmed = combined.trim();
    if trimmed.chars().count() <= OUTPUT_TAIL_CHARS {
        return trimmed.to_string();
    }
    let tail: String = trimmed
        .chars()
        .rev()
        .take(OUTPUT_TAIL_CHARS)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    format!("…{tail}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn discover_typecheck_script_and_non_watch_test() {
        let pkg = json!({
            "scripts": {
                "test": "node --test",
                "typecheck": "tsc --noEmit"
            }
        });
        let commands = commands_from_package(&pkg, PackageManager::Pnpm);
        assert_eq!(
            commands,
            vec![
                VerifyCommand {
                    command: "pnpm typecheck".into(),
                    kind: VerifyKind::Typecheck,
                },
                VerifyCommand {
                    command: "pnpm test".into(),
                    kind: VerifyKind::Test,
                },
            ]
        );
    }

    #[test]
    fn discover_falls_back_to_tsc_and_vitest_run() {
        let pkg = json!({
            "devDependencies": {
                "typescript": "5.0.0",
                "vitest": "3.0.0"
            },
            "scripts": {
                "test": "vitest"
            }
        });
        let commands = commands_from_package(&pkg, PackageManager::Pnpm);
        assert_eq!(
            commands,
            vec![
                VerifyCommand {
                    command: "pnpm exec tsc --noEmit".into(),
                    kind: VerifyKind::Typecheck,
                },
                VerifyCommand {
                    command: "pnpm exec vitest run".into(),
                    kind: VerifyKind::Test,
                },
            ]
        );
    }

    #[test]
    fn discover_prefers_test_run_over_watch_test() {
        let pkg = json!({
            "scripts": {
                "test": "vitest",
                "test:run": "vitest run",
                "typecheck": "tsc -b"
            }
        });
        let commands = commands_from_package(&pkg, PackageManager::Npm);
        assert_eq!(
            commands
                .iter()
                .map(|c| c.command.as_str())
                .collect::<Vec<_>>(),
            vec!["npm run typecheck", "npm run test:run"]
        );
    }

    #[test]
    fn lockfile_prefers_pnpm() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("pnpm-lock.yaml"), "lockfileVersion: 9\n").unwrap();
        std::fs::write(dir.path().join("package-lock.json"), "{}\n").unwrap();
        assert_eq!(detect_package_manager(dir.path()), PackageManager::Pnpm);
    }

    #[test]
    fn missing_package_json_skips() {
        let dir = tempfile::tempdir().unwrap();
        assert!(discover_verify_commands(dir.path()).unwrap().is_empty());
    }

    #[test]
    fn override_lines_classify_test_vs_typecheck() {
        let commands = parse_override_commands("pnpm typecheck\npnpm exec vitest run\n# ignored\n");
        assert_eq!(commands[0].kind, VerifyKind::Typecheck);
        assert_eq!(commands[1].kind, VerifyKind::Test);
        assert_eq!(commands.len(), 2);
    }

    #[test]
    fn watch_script_detection() {
        assert!(is_watch_script("vitest"));
        assert!(is_watch_script("vitest --watch"));
        assert!(!is_watch_script("vitest run"));
        assert!(!is_watch_script("vitest --run"));
        assert!(!is_watch_script("node --test"));
        assert!(is_watch_script("jest --watch"));
        assert!(!is_watch_script("jest"));
    }

    #[tokio::test]
    async fn override_true_passes() {
        let dir = tempfile::tempdir().unwrap();
        run_frontend_verify(dir.path(), Some("true")).await.unwrap();
    }

    #[tokio::test]
    async fn override_false_blocks_cr_as_tsc() {
        let dir = tempfile::tempdir().unwrap();
        let err = run_frontend_verify(dir.path(), Some("false"))
            .await
            .unwrap_err();
        assert!(err.contains("LocalVerifyError"));
        assert!(err.contains("tsc"));
        assert!(err.contains("不得进入 CR"));
    }

    #[tokio::test]
    async fn override_test_failure_mentions_test_fail() {
        let dir = tempfile::tempdir().unwrap();
        let err = run_frontend_verify(dir.path(), Some("false # vitest"))
            .await
            .unwrap_err();
        assert!(err.contains("test fail"));
        assert!(err.contains("LocalVerifyError"));
    }
}
