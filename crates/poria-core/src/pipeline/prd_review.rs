use serde::{Deserialize, Serialize};

/// Markers that mean a P0/P1/P2 answer has not been filled in yet.
const UNANSWERED_PLACEHOLDERS: &[&str] = &["todo", "tbd", "待填写", "待确认", "待补充"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrdReviewPriority {
    P0,
    P1,
    P2,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PrdReviewStatus {
    pub p0_done: bool,
    pub p1_done: bool,
    pub p2_done: bool,
    pub p0_unanswered: Vec<String>,
    pub p1_unanswered: Vec<String>,
    pub p2_unanswered: Vec<String>,
}

impl PrdReviewStatus {
    pub fn from_markdown(markdown: &str) -> Self {
        parse_prd_review(markdown)
    }

    pub fn p0_block_message(&self) -> String {
        if self.p0_unanswered.is_empty() {
            "P0 unanswered: 请在 PRD_REVIEW.md 填写 P0 答案后再进入设计".into()
        } else {
            format!(
                "P0 unanswered: {} 请在 PRD_REVIEW.md 填写后再进入设计",
                self.p0_unanswered.join("、")
            )
        }
    }

    pub fn warn_message(&self) -> Option<String> {
        let mut parts = Vec::new();
        if !self.p1_unanswered.is_empty() {
            parts.push(format!(
                "P1 未答 {}（不阻塞）",
                self.p1_unanswered.join("、")
            ));
        }
        if !self.p2_unanswered.is_empty() {
            parts.push(format!(
                "P2 未答 {}（不阻塞）",
                self.p2_unanswered.join("、")
            ));
        }
        if parts.is_empty() {
            None
        } else {
            Some(format!("提醒：{}", parts.join("；")))
        }
    }

    pub fn gate_results_json(&self) -> serde_json::Value {
        serde_json::json!([{
            "gate": "PRD 澄清 P0 完成",
            "passed": self.p0_done,
            "actual": self.p0_done.to_string(),
            "threshold": "true",
        }])
    }
}

/// True when the answer text is blank or a known placeholder.
pub fn is_unanswered_answer(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return true;
    }
    let stripped = trimmed
        .trim_end_matches(['。', '.', '！', '!', '？', '?'])
        .trim();
    let lower = stripped.to_lowercase();
    UNANSWERED_PLACEHOLDERS
        .iter()
        .any(|marker| lower == *marker)
}

pub fn parse_prd_review(markdown: &str) -> PrdReviewStatus {
    if markdown.trim().is_empty() {
        return PrdReviewStatus {
            p0_done: false,
            p1_done: true,
            p2_done: true,
            p0_unanswered: vec!["PRD_REVIEW.md".into()],
            p1_unanswered: vec![],
            p2_unanswered: vec![],
        };
    }

    #[derive(Clone)]
    struct Item {
        priority: PrdReviewPriority,
        answer: String,
        has_q: bool,
        has_a: bool,
    }

    let mut current = None::<PrdReviewPriority>;
    let mut items: Vec<(u32, Item)> = Vec::new();
    let mut pending_answer: Option<u32> = None;

    fn find_item(items: &mut Vec<(u32, Item)>, num: u32) -> Option<&mut Item> {
        items
            .iter_mut()
            .find(|(n, _)| *n == num)
            .map(|(_, item)| item)
    }

    fn upsert(items: &mut Vec<(u32, Item)>, num: u32, priority: PrdReviewPriority) -> &mut Item {
        if let Some(pos) = items.iter().position(|(n, _)| *n == num) {
            &mut items[pos].1
        } else {
            items.push((
                num,
                Item {
                    priority,
                    answer: String::new(),
                    has_q: false,
                    has_a: false,
                },
            ));
            &mut items.last_mut().unwrap().1
        }
    }

    fn heading_priority(line: &str) -> Option<PrdReviewPriority> {
        let trimmed = line.trim();
        if !trimmed.starts_with('#') {
            return None;
        }
        let rest = trimmed.trim_start_matches('#').trim();
        if rest.starts_with("P0") {
            Some(PrdReviewPriority::P0)
        } else if rest.starts_with("P1") {
            Some(PrdReviewPriority::P1)
        } else if rest.starts_with("P2") {
            Some(PrdReviewPriority::P2)
        } else {
            None
        }
    }

    fn q_number(line: &str) -> Option<u32> {
        parse_marker_number(line, 'Q')
    }

    fn a_number_and_rest(line: &str) -> Option<(u32, String)> {
        let num = parse_marker_number(line, 'A')?;
        let rest = line
            .split("**：")
            .nth(1)
            .or_else(|| line.split("**:").nth(1))
            .unwrap_or("")
            .trim()
            .to_string();
        Some((num, rest))
    }

    fn infer_priority_from_q(line: &str, current: Option<PrdReviewPriority>) -> PrdReviewPriority {
        if line.contains("P0") {
            PrdReviewPriority::P0
        } else if line.contains("P1") {
            PrdReviewPriority::P1
        } else if line.contains("P2") {
            PrdReviewPriority::P2
        } else {
            current.unwrap_or(PrdReviewPriority::P0)
        }
    }

    for line in markdown.lines() {
        if let Some(priority) = heading_priority(line) {
            current = Some(priority);
            pending_answer = None;
            continue;
        }

        if let Some(num) = q_number(line) {
            pending_answer = None;
            let priority = infer_priority_from_q(line, current);
            let item = upsert(&mut items, num, priority);
            item.has_q = true;
            item.priority = priority;
            continue;
        }

        if let Some((num, rest)) = a_number_and_rest(line) {
            let priority = current.unwrap_or(PrdReviewPriority::P0);
            let item = upsert(&mut items, num, priority);
            item.has_a = true;
            item.answer = rest;
            if current.is_some() {
                item.priority = priority;
            }
            pending_answer = Some(num);
            continue;
        }

        if let Some(num) = pending_answer {
            let trimmed = line.trim();
            if trimmed.starts_with('#') || trimmed.starts_with("**Q") || trimmed.starts_with("**A")
            {
                pending_answer = None;
            } else if let Some(item) = find_item(&mut items, num) {
                if !item.answer.is_empty() {
                    item.answer.push('\n');
                }
                item.answer.push_str(line);
            }
        }
    }

    let mut status = PrdReviewStatus::default();
    let mut p0_count = 0usize;
    let mut p1_count = 0usize;
    let mut p2_count = 0usize;

    for (num, item) in items {
        let label = format!("Q{num}");
        let unanswered = !item.has_a || is_unanswered_answer(&item.answer);
        match item.priority {
            PrdReviewPriority::P0 => {
                p0_count += 1;
                if unanswered {
                    status.p0_unanswered.push(label);
                }
            }
            PrdReviewPriority::P1 => {
                p1_count += 1;
                if unanswered {
                    status.p1_unanswered.push(label);
                }
            }
            PrdReviewPriority::P2 => {
                p2_count += 1;
                if unanswered {
                    status.p2_unanswered.push(label);
                }
            }
        }
    }

    status.p0_done = p0_count == 0 || status.p0_unanswered.is_empty();
    status.p1_done = p1_count == 0 || status.p1_unanswered.is_empty();
    status.p2_done = p2_count == 0 || status.p2_unanswered.is_empty();
    status
}

fn parse_marker_number(line: &str, marker: char) -> Option<u32> {
    let trimmed = line.trim();
    let prefix = format!("**{marker}");
    if !trimmed.starts_with(&prefix) {
        return None;
    }
    let rest = &trimmed[prefix.len()..];
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    let after = &rest[digits.len()..];
    if !after.starts_with("**") {
        return None;
    }
    digits.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SKILL_SAMPLE: &str = r#"# PRD 前端落地澄清：行程卡片

## 2. 前端落地澄清清单

### P0（必填 · 不答将阻塞开发）

**Q1**（P0 · GAP · 行程卡片）：空态怎么展示？<br>
**A1**：

**Q2**（P0 · GAP · 行程卡片）：失败如何提示？<br>
**A2**：TODO

### P1（建议填写 · 不答有较大返工风险）

**Q3**（P1 · FIX · 历史归档）：文案谁提供？<br>
**A3**：

### P2（选填 · 不答易在验收/体验上产生争议）

**Q4**（P2 · GAP · 历史归档）：无数据插画用哪张？<br>
**A4**：待确认
"#;

    #[test]
    fn unanswered_placeholders() {
        assert!(is_unanswered_answer(""));
        assert!(is_unanswered_answer("   "));
        assert!(is_unanswered_answer("TODO"));
        assert!(is_unanswered_answer("todo"));
        assert!(is_unanswered_answer("待填写"));
        assert!(is_unanswered_answer("待确认"));
        assert!(is_unanswered_answer("待确认。"));
        assert!(!is_unanswered_answer("走列表页空态"));
    }

    #[test]
    fn empty_markdown_blocks_p0() {
        let status = parse_prd_review("  \n");
        assert!(!status.p0_done);
        assert_eq!(status.p0_unanswered, vec!["PRD_REVIEW.md"]);
    }

    #[test]
    fn skill_format_detects_unanswered_p0() {
        let status = parse_prd_review(SKILL_SAMPLE);
        assert!(!status.p0_done);
        assert_eq!(status.p0_unanswered, vec!["Q1", "Q2"]);
        assert!(!status.p1_done);
        assert_eq!(status.p1_unanswered, vec!["Q3"]);
        assert!(!status.p2_done);
        assert_eq!(status.p2_unanswered, vec!["Q4"]);
        assert!(status.p0_block_message().contains("Q1"));
        assert!(status.warn_message().unwrap().contains("P1"));
    }

    #[test]
    fn answered_p0_does_not_block() {
        let markdown = r#"
### P0（必填）

**Q1**（P0 · GAP · 卡片）：空态？<br>
**A1**：展示「暂无行程」文案

**Q2**（P0 · GAP · 卡片）：失败？<br>
**A2**：Toast 提示稍后重试
"#;
        let status = parse_prd_review(markdown);
        assert!(status.p0_done);
        assert!(status.p0_unanswered.is_empty());
    }

    #[test]
    fn p0_none_is_done() {
        let markdown = r#"
### P0（必填 · 不答将阻塞开发）

无

### P1（建议填写）

**Q1**（P1 · GAP · 卡片）：文案？<br>
**A1**：
"#;
        let status = parse_prd_review(markdown);
        assert!(status.p0_done);
        assert!(status.p0_unanswered.is_empty());
        assert_eq!(status.p1_unanswered, vec!["Q1"]);
    }

    #[test]
    fn multiline_answer_counts_as_filled() {
        let markdown = r#"
### P0

**Q1**（P0 · GAP · 卡片）：规则？<br>
**A1**：第一行
继续补充的第二行
"#;
        let status = parse_prd_review(markdown);
        assert!(status.p0_done);
    }

    #[test]
    fn ascii_colon_answer_is_parsed() {
        let markdown = r#"
### P0

**Q1**（P0 · GAP · 卡片）：口径？<br>
**A1**: 已确认走审核流
"#;
        let status = parse_prd_review(markdown);
        assert!(status.p0_done);
    }

    #[test]
    fn missing_answer_line_is_unanswered() {
        let markdown = r#"
### P0

**Q1**（P0 · GAP · 卡片）：口径？<br>
"#;
        let status = parse_prd_review(markdown);
        assert!(!status.p0_done);
        assert_eq!(status.p0_unanswered, vec!["Q1"]);
    }
}
