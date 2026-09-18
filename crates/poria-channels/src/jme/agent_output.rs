/// Extract text payloads from JoyClaw `--json` stdout.
/// The agent often prints logs then a trailing `{ "payloads": [{ "text": "..." }] }`.
pub fn parse_agent_output(stdout: &str) -> Vec<String> {
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    if let Some(texts) = payloads_from_json(trimmed) {
        return texts;
    }
    for line in trimmed.lines().rev() {
        let line = line.trim();
        if line.starts_with('{') {
            if let Some(texts) = payloads_from_json(line) {
                return texts;
            }
        }
    }
    if let Some(idx) = trimmed.rfind('{') {
        if let Some(texts) = payloads_from_json(&trimmed[idx..]) {
            return texts;
        }
    }
    Vec::new()
}

fn payloads_from_json(raw: &str) -> Option<Vec<String>> {
    let value: serde_json::Value = serde_json::from_str(raw).ok()?;
    let items = value.get("payloads")?.as_array()?;
    let texts: Vec<String> = items
        .iter()
        .filter_map(|item| {
            item.get("text")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToOwned::to_owned)
        })
        .collect();
    if texts.is_empty() {
        None
    } else {
        Some(texts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_payloads_object() {
        assert_eq!(
            parse_agent_output(r#"{"payloads":[{"text":"sent ok"}]}"#),
            vec!["sent ok"]
        );
    }

    #[test]
    fn reads_trailing_json_after_logs() {
        let stdout = "info starting\n{\"payloads\":[{\"text\":\"msg 1\"},{\"text\":\"msg 2\"}]}\n";
        assert_eq!(parse_agent_output(stdout), vec!["msg 1", "msg 2"]);
    }

    #[test]
    fn empty_when_no_json() {
        assert!(parse_agent_output("gateway starting").is_empty());
    }
}
