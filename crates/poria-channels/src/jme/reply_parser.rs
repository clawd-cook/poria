use regex::Regex;

/// Parsed action from a human reply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplyAction {
    Resume,
    Skip,
    Cancel,
    Unknown,
}

impl std::fmt::Display for ReplyAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReplyAction::Resume => write!(f, "resume"),
            ReplyAction::Skip => write!(f, "skip"),
            ReplyAction::Cancel => write!(f, "cancel"),
            ReplyAction::Unknown => write!(f, "unknown"),
        }
    }
}

/// Parse a human reply text into an action.
/// Returns `Unknown` if no pattern matches.
pub fn parse_reply(text: &str) -> ReplyAction {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return ReplyAction::Unknown;
    }

    // Cancel patterns (checked first -- highest priority)
    let cancel_patterns: &[Regex] = &[
        Regex::new(r"\x{53D6}\x{6D88}").unwrap(), // 取消
        Regex::new(r"(?i)cancel").unwrap(),
        Regex::new(r"\x{7EC8}\x{6B62}").unwrap(), // 终止
        Regex::new(r"(?i)abort").unwrap(),
        Regex::new(r"(?i)stop").unwrap(),
    ];
    for pat in cancel_patterns {
        if pat.is_match(trimmed) {
            return ReplyAction::Cancel;
        }
    }

    // Skip patterns
    let skip_patterns: &[Regex] = &[
        Regex::new(r"\x{8DF3}\x{8FC7}").unwrap(), // 跳过
        Regex::new(r"(?i)skip").unwrap(),
        Regex::new(r"\x{5FFD}\x{7565}").unwrap(), // 忽略
        Regex::new(r"(?i)ignore").unwrap(),
    ];
    for pat in skip_patterns {
        if pat.is_match(trimmed) {
            return ReplyAction::Skip;
        }
    }

    // Resume patterns
    let resume_patterns: &[Regex] = &[
        Regex::new(r"\x{4FEE}\x{590D}").unwrap(), // 修复
        Regex::new(r"(?i)fix").unwrap(),
        Regex::new(r"(?i)resume").unwrap(),
        Regex::new(r"\x{5DF2}\x{4FEE}\x{590D}").unwrap(), // 已修复
        Regex::new(r"\x{5DF2}\x{89E3}\x{51B3}").unwrap(), // 已解决
        Regex::new(r"\x{91CD}\x{8BD5}").unwrap(),         // 重试
        Regex::new(r"(?i)retry").unwrap(),
    ];
    for pat in resume_patterns {
        if pat.is_match(trimmed) {
            return ReplyAction::Resume;
        }
    }

    ReplyAction::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resume_chinese_fix() {
        assert_eq!(parse_reply("\u{4FEE}\u{590D}"), ReplyAction::Resume);
    }

    #[test]
    fn resume_english_fix() {
        assert_eq!(parse_reply("fix"), ReplyAction::Resume);
    }

    #[test]
    fn resume_fix_case_insensitive() {
        assert_eq!(parse_reply("Fix"), ReplyAction::Resume);
    }

    #[test]
    fn resume_already_fixed() {
        assert_eq!(
            parse_reply("\u{5DF2}\u{4FEE}\u{590D}\u{4E86}"),
            ReplyAction::Resume
        );
    }

    #[test]
    fn resume_retry_chinese() {
        assert_eq!(parse_reply("\u{8BF7}\u{91CD}\u{8BD5}"), ReplyAction::Resume);
    }

    #[test]
    fn resume_retry_english() {
        assert_eq!(parse_reply("please retry"), ReplyAction::Resume);
    }

    #[test]
    fn skip_chinese() {
        assert_eq!(parse_reply("\u{8DF3}\u{8FC7}"), ReplyAction::Skip);
    }

    #[test]
    fn skip_english() {
        assert_eq!(parse_reply("skip this"), ReplyAction::Skip);
    }

    #[test]
    fn skip_ignore_chinese() {
        assert_eq!(parse_reply("\u{5FFD}\u{7565}"), ReplyAction::Skip);
    }

    #[test]
    fn cancel_chinese() {
        assert_eq!(parse_reply("\u{53D6}\u{6D88}"), ReplyAction::Cancel);
    }

    #[test]
    fn cancel_english() {
        assert_eq!(parse_reply("cancel"), ReplyAction::Cancel);
    }

    #[test]
    fn cancel_terminate_chinese() {
        assert_eq!(
            parse_reply("\u{7EC8}\u{6B62}\u{6D41}\u{6C34}\u{7EBF}"),
            ReplyAction::Cancel
        );
    }

    #[test]
    fn unknown_for_unrecognized() {
        assert_eq!(parse_reply("I need more time"), ReplyAction::Unknown);
    }

    #[test]
    fn unknown_for_empty() {
        assert_eq!(parse_reply(""), ReplyAction::Unknown);
    }

    #[test]
    fn unknown_for_whitespace() {
        assert_eq!(parse_reply("   "), ReplyAction::Unknown);
    }

    #[test]
    fn cancel_wins_over_resume() {
        // cancel is checked first
        assert_eq!(
            parse_reply("\u{53D6}\u{6D88}\u{4FEE}\u{590D}"),
            ReplyAction::Cancel
        );
    }
}
