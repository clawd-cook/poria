/// Extract OutputGuard allowed-path globs from frontend `TRD.md`.
///
/// Prefers the dedicated `## 允许修改范围` section written by `skill:gen-trd`.
/// Falls back to path-like lines under `## 代码位置概览` when that section is
/// missing so an older TRD still yields a scope.
pub fn parse_trd_scope(markdown: &str) -> Vec<String> {
    let from_allowed = collect_globs_in_sections(
        markdown,
        &["允许修改范围", "allowed paths", "allowed scope", "trdscope"],
    );
    if !from_allowed.is_empty() {
        return dedupe_preserve_order(from_allowed);
    }
    dedupe_preserve_order(collect_globs_in_sections(
        markdown,
        &["代码位置概览", "directory tree", "代码落点"],
    ))
}

fn collect_globs_in_sections(markdown: &str, heading_needles: &[&str]) -> Vec<String> {
    let mut globs = Vec::new();
    let mut in_section = false;
    let mut in_fence = false;
    for line in markdown.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if !in_fence && is_heading(trimmed) {
            in_section = heading_matches(heading_text(trimmed), heading_needles);
            continue;
        }
        if !in_section {
            continue;
        }
        if let Some(glob) = extract_glob(trimmed) {
            globs.push(glob);
        }
    }
    globs
}

fn is_heading(line: &str) -> bool {
    line.starts_with('#')
}

fn heading_text(line: &str) -> String {
    line.trim_start_matches('#').trim().to_ascii_lowercase()
}

fn heading_matches(heading: String, needles: &[&str]) -> bool {
    needles
        .iter()
        .any(|needle| heading.contains(&needle.to_ascii_lowercase()))
}

fn extract_glob(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty()
        || trimmed.starts_with('|')
        || trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
        || trimmed.starts_with("<!--")
    {
        return None;
    }
    let without_marker = trimmed
        .trim_start_matches(['-', '*', '+'])
        .trim()
        .trim_start_matches(|c: char| c.is_ascii_digit())
        .trim_start_matches(['.', '、', ')', '）'])
        .trim();
    let unquoted = without_marker
        .trim_matches('`')
        .trim_matches('"')
        .trim_matches('\'')
        .trim();
    let token = unquoted
        .split_whitespace()
        .next()?
        .trim_end_matches(['，', ',', '。', ';', '；', '）', ')']);
    if looks_like_glob(token) {
        Some(token.to_string())
    } else {
        None
    }
}

fn looks_like_glob(value: &str) -> bool {
    if value.is_empty() || value.contains("://") || value.len() > 240 {
        return false;
    }
    if value.contains('*') || value.contains('/') {
        return true;
    }
    matches!(
        value.rsplit_once('.').map(|(_, ext)| ext),
        Some(
            "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "css" | "scss" | "less" | "vue" | "json"
        )
    )
}

fn dedupe_preserve_order(items: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for item in items {
        if seen.insert(item.clone()) {
            out.push(item);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_allowed_section_list_items() {
        let md = r#"
# TRD

## 允许修改范围

- `src/pages/list/**`
- src/api/demand.ts
- package.json

## 页面详情
"#;
        assert_eq!(
            parse_trd_scope(md),
            vec![
                "src/pages/list/**".to_string(),
                "src/api/demand.ts".to_string(),
                "package.json".to_string()
            ]
        );
    }

    #[test]
    fn parse_fenced_globs() {
        let md = r#"
## 允许修改范围

```
src/components/Foo.tsx
src/hooks/**
```
"#;
        assert_eq!(
            parse_trd_scope(md),
            vec![
                "src/components/Foo.tsx".to_string(),
                "src/hooks/**".to_string()
            ]
        );
    }

    #[test]
    fn fallback_to_code_layout_when_allowed_section_missing() {
        let md = r#"
## 代码位置概览

- src/pages/foo/index.tsx
- src/api/foo.ts

本期只改上述目录。
"#;
        assert_eq!(
            parse_trd_scope(md),
            vec![
                "src/pages/foo/index.tsx".to_string(),
                "src/api/foo.ts".to_string()
            ]
        );
    }

    #[test]
    fn empty_when_no_paths() {
        let md = "# TRD\n\n## 概要\n\n本期做列表页。\n";
        assert!(parse_trd_scope(md).is_empty());
    }

    #[test]
    fn ignores_urls_and_tables() {
        let md = r#"
## 允许修改范围

| 路径 | 说明 |
| --- | --- |
https://example.com/src/foo.ts
- src/real.ts
"#;
        assert_eq!(parse_trd_scope(md), vec!["src/real.ts".to_string()]);
    }
}
