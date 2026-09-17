use std::process::Stdio;

use async_trait::async_trait;
use serde::Deserialize;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tracing::{debug, error, info, warn};

use super::agent_pool::{AgentQueryOptions, AgentSdk, SdkMessage};

/// Real implementation of `AgentSdk` that spawns `claude` CLI processes.
pub struct ClaudeCliSdk {
    claude_path: String,
}

impl ClaudeCliSdk {
    pub fn new(claude_path: Option<String>) -> Self {
        Self {
            claude_path: claude_path.unwrap_or_else(|| "claude".into()),
        }
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct StreamJsonEvent {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(default)]
    subtype: Option<String>,
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    message: Option<serde_json::Value>,
    #[serde(default)]
    result: Option<String>,
    #[serde(default)]
    total_cost_usd: Option<f64>,
    #[serde(default)]
    tool_name: Option<String>,
    #[serde(default)]
    content: Option<String>,
}

fn build_cli_args(prompt: &str, options: &AgentQueryOptions) -> Vec<String> {
    let mut args = vec![
        "-p".into(),
        prompt.into(),
        "--output-format".into(),
        "stream-json".into(),
        "--verbose".into(),
        "--dangerously-skip-permissions".into(),
    ];

    if let Some(ref sys_prompt) = options.append_system_prompt {
        args.push("--system-prompt".into());
        args.push(sys_prompt.clone());
    }

    if !options.allowed_tools.is_empty() {
        let tools = options.allowed_tools.join(",");
        args.push("--tools".into());
        args.push(tools.clone());
        args.push("--allowedTools".into());
        args.push(tools);
        let disallowed = disallowed_tools(&options.allowed_tools);
        if !disallowed.is_empty() {
            args.push("--disallowedTools".into());
            args.push(disallowed);
        }
    }

    if let Some(max_turns) = options.max_turns {
        args.push("--max-turns".into());
        args.push(max_turns.to_string());
    }

    args
}

const DEFAULT_DISALLOWED_TOOLS: &[&str] = &[
    "Agent",
    "Task",
    "Bash",
    "Glob",
    "WebFetch",
    "WebSearch",
];

fn disallowed_tools(allowed: &[String]) -> String {
    DEFAULT_DISALLOWED_TOOLS
        .iter()
        .copied()
        .filter(|name| {
            !allowed
                .iter()
                .any(|allowed_name| allowed_name.eq_ignore_ascii_case(name))
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn parse_stream_event(event: &StreamJsonEvent) -> SdkMessage {
    SdkMessage {
        msg_type: event.event_type.clone(),
        message: event.message.clone(),
        result: event.result.clone(),
        total_cost_usd: event.total_cost_usd,
    }
}

#[async_trait]
impl AgentSdk for ClaudeCliSdk {
    async fn query(
        &self,
        prompt: &str,
        options: AgentQueryOptions,
    ) -> Result<(Vec<SdkMessage>, Option<String>), Box<dyn std::error::Error + Send + Sync>> {
        let args = build_cli_args(prompt, &options);

        debug!(
            claude_path = %self.claude_path,
            args_count = args.len(),
            cwd = ?options.cwd,
            "spawning claude CLI"
        );

        let mut cmd = Command::new(&self.claude_path);
        cmd.args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        if let Some(ref cwd) = options.cwd {
            cmd.current_dir(cwd);
        }

        let mut child = cmd.spawn().map_err(|e| {
            error!(path = %self.claude_path, error = %e, "failed to spawn claude CLI");
            Box::new(e) as Box<dyn std::error::Error + Send + Sync>
        })?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "failed to capture stdout".to_string())?;

        let stderr_handle = child.stderr.take();

        let mut reader = BufReader::new(stdout).lines();
        let mut messages: Vec<SdkMessage> = Vec::new();
        let mut session_id: Option<String> = None;

        while let Some(line) = reader.next_line().await? {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            match serde_json::from_str::<StreamJsonEvent>(trimmed) {
                Ok(event) => {
                    if event.session_id.is_some() {
                        session_id = event.session_id.clone();
                    }

                    let msg = parse_stream_event(&event);
                    messages.push(msg);

                    debug!(
                        event_type = %event.event_type,
                        subtype = ?event.subtype,
                        "stream event"
                    );
                }
                Err(e) => {
                    warn!(line = %trimmed, error = %e, "failed to parse stream-json line");
                }
            }
        }

        let status = child.wait().await?;

        if !status.success() {
            let stderr_text = if let Some(stderr) = stderr_handle {
                let mut stderr_reader = BufReader::new(stderr).lines();
                let mut buf = String::new();
                while let Some(line) = stderr_reader.next_line().await? {
                    buf.push_str(&line);
                    buf.push('\n');
                }
                buf
            } else {
                String::new()
            };

            if messages.is_empty() {
                return Err(
                    format!("claude CLI exited with {}: {}", status, stderr_text.trim()).into(),
                );
            }
            warn!(
                status = %status,
                stderr = %stderr_text.trim(),
                "claude CLI exited with non-zero but produced output"
            );
        }

        info!(
            message_count = messages.len(),
            session_id = ?session_id,
            "claude CLI query completed"
        );

        Ok((messages, session_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_cli_args_minimal() {
        let options = AgentQueryOptions {
            cwd: None,
            max_budget_usd: None,
            max_turns: None,
            model: None,
            append_system_prompt: None,
            allowed_tools: vec![],
        };
        let args = build_cli_args("hello", &options);
        assert_eq!(
            args,
            vec![
                "-p",
                "hello",
                "--output-format",
                "stream-json",
                "--verbose",
                "--dangerously-skip-permissions",
            ]
        );
    }

    #[test]
    fn test_build_cli_args_full() {
        let options = AgentQueryOptions {
            cwd: Some("/tmp".into()),
            max_budget_usd: Some(5.0),
            max_turns: Some(20),
            model: Some("sonnet".into()),
            append_system_prompt: Some("You are a code reviewer.".into()),
            allowed_tools: vec!["Read".into(), "Grep".into()],
        };
        let args = build_cli_args("review this code", &options);
        assert!(args.contains(&"--system-prompt".to_string()));
        assert!(args.contains(&"You are a code reviewer.".to_string()));
        assert!(args.contains(&"--allowedTools".to_string()));
        assert!(args.contains(&"Read,Grep".to_string()));
        assert!(args.contains(&"--tools".to_string()));
        assert!(args.contains(&"--disallowedTools".to_string()));
        assert!(args.contains(&"Agent,Task,Bash,Glob,WebFetch,WebSearch".to_string()));
        assert!(args.contains(&"--max-turns".to_string()));
        assert!(args.contains(&"20".to_string()));
    }

    #[test]
    fn test_disallowed_tools_keeps_allowed_bash_and_glob() {
        let allowed = vec!["Read".into(), "Write".into(), "Bash".into(), "Glob".into()];
        assert_eq!(disallowed_tools(&allowed), "Agent,Task,WebFetch,WebSearch");
    }

    #[test]
    fn test_parse_stream_event_result() {
        let event = StreamJsonEvent {
            event_type: "result".into(),
            subtype: None,
            session_id: Some("sess-abc".into()),
            message: None,
            result: Some("Task completed successfully".into()),
            total_cost_usd: Some(0.42),
            tool_name: None,
            content: None,
        };
        let msg = parse_stream_event(&event);
        assert_eq!(msg.msg_type, "result");
        assert_eq!(msg.result.as_deref(), Some("Task completed successfully"));
        assert_eq!(msg.total_cost_usd, Some(0.42));
    }

    #[test]
    fn test_parse_stream_event_assistant() {
        let event = StreamJsonEvent {
            event_type: "assistant".into(),
            subtype: Some("text".into()),
            session_id: None,
            message: Some(serde_json::json!({"content": "thinking..."})),
            result: None,
            total_cost_usd: None,
            tool_name: None,
            content: Some("thinking...".into()),
        };
        let msg = parse_stream_event(&event);
        assert_eq!(msg.msg_type, "assistant");
        assert!(msg.message.is_some());
    }
}
