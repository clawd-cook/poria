/// Blocking CR findings (P0/P1) to post as Coding MR notes.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrFinding {
    pub severity: String,
    pub id: String,
    pub title: String,
    pub body: String,
}

pub fn parse_blocking_findings(cr_md: &str) -> Vec<CrFinding> {
    let mut findings = Vec::new();
    let lines: Vec<&str> = cr_md.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let Some((id, title)) = parse_heading(lines[i]) else {
            i += 1;
            continue;
        };
        let severity = id.split('-').next().unwrap_or("P0").to_string();
        i += 1;
        let mut body_lines = Vec::new();
        while i < lines.len() {
            if parse_heading(lines[i]).is_some() || lines[i].starts_with("## ") {
                break;
            }
            body_lines.push(lines[i]);
            i += 1;
        }
        let body = body_lines
            .into_iter()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        if title == "无" || title.eq_ignore_ascii_case("none") {
            continue;
        }
        findings.push(CrFinding {
            severity,
            id,
            title,
            body,
        });
    }
    findings
}

fn parse_heading(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    let rest = trimmed.strip_prefix("#### ")?;
    let mut parts = rest.splitn(2, char::is_whitespace);
    let id = parts.next()?.trim();
    if !(id.starts_with("P0-") || id.starts_with("P1-")) {
        return None;
    }
    let title = parts.next().unwrap_or("").trim().to_string();
    Some((id.to_string(), title))
}

pub fn format_mr_note(finding: &CrFinding) -> String {
    let mut note = format!("【Poria 独立评审】{} {}", finding.id, finding.title);
    if !finding.body.is_empty() {
        note.push('\n');
        note.push('\n');
        note.push_str(&finding.body);
    }
    note
}

pub fn mr_notes_from_cr(cr_md: &str) -> Vec<String> {
    parse_blocking_findings(cr_md)
        .iter()
        .map(format_mr_note)
        .collect()
}

pub fn cr_stage_mr_notes(output: Option<&serde_json::Value>) -> Vec<String> {
    output
        .and_then(|value| value.get("mrNotes"))
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .filter(|note| !note.trim().is_empty())
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_p0_and_p1_skips_p2_and_empty() {
        let cr = r#"
# Code Review 报告

### P0

#### P0-01 空指针
- 风险说明：未判空
- 证据：src/a.ts:12
- 修复建议：加守卫

### P1

#### P1-01 错误吞掉
- 风险说明：catch 后忽略
- 证据：src/b.ts:4

### P2

#### P2-01 边界
- 风险说明：列表为空

### 疑似风险

#### S-01 猜测
"#;
        let findings = parse_blocking_findings(cr);
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].id, "P0-01");
        assert_eq!(findings[0].title, "空指针");
        assert_eq!(findings[1].id, "P1-01");
        assert!(format_mr_note(&findings[0]).contains("【Poria 独立评审】P0-01 空指针"));
        assert!(format_mr_note(&findings[0]).contains("src/a.ts:12"));
    }

    #[test]
    fn skip_placeholder_none() {
        let cr = "#### P0-01 无\n";
        assert!(parse_blocking_findings(cr).is_empty());
    }

    #[test]
    fn cr_stage_mr_notes_reads_output() {
        let output = serde_json::json!({
            "mrNotes": ["note-a", "", "note-b"]
        });
        assert_eq!(
            cr_stage_mr_notes(Some(&output)),
            vec!["note-a".to_string(), "note-b".to_string()]
        );
        assert!(cr_stage_mr_notes(None).is_empty());
    }
}
