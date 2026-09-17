use std::collections::{HashMap, HashSet};

use serde_json::Value;

#[derive(Debug, Clone, Default)]
pub struct DiagramInfo {
    pub title: String,
    pub link_url: String,
    pub page_url: String,
    pub svg: String,
    pub mermaid: String,
}

#[derive(Debug, Clone)]
pub struct ConversionResult {
    pub title: String,
    pub markdown: String,
    pub warnings: Vec<String>,
}

struct ConvertState<'a> {
    warnings: &'a mut HashSet<String>,
    ordered_counters: HashMap<i64, i64>,
    diagrams: &'a HashMap<String, DiagramInfo>,
    page_url: String,
}

pub fn joyspace_content_to_markdown(
    title: &str,
    content: &[Value],
    diagrams: &HashMap<String, DiagramInfo>,
    page_url: &str,
) -> ConversionResult {
    let resolved_title = normalize_text(title);
    let resolved_title = if resolved_title.is_empty() {
        "Doc".to_string()
    } else {
        resolved_title
    };

    let mut warnings = HashSet::new();
    let mut blocks: Vec<Value> = content.to_vec();
    while let Some(first) = blocks.first() {
        if normalize_text(&block_plain_text(first)) == resolved_title {
            blocks.remove(0);
            continue;
        }
        break;
    }

    let mut state = ConvertState {
        warnings: &mut warnings,
        ordered_counters: HashMap::new(),
        diagrams,
        page_url: normalize_text(page_url),
    };
    let body = render_blocks(&blocks, &mut state);
    let mut markdown = format!("# {resolved_title}\n\n{body}");
    markdown = markdown
        .replace("\r", "")
        .split('\n')
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n");
    while markdown.contains("\n\n\n") {
        markdown = markdown.replace("\n\n\n", "\n\n");
    }
    markdown = format_markdown(markdown.trim());
    markdown.push('\n');

    ConversionResult {
        title: resolved_title,
        markdown,
        warnings: warnings.into_iter().collect(),
    }
}

fn node_type(node: &Value) -> &str {
    node.get("type").and_then(Value::as_str).unwrap_or("")
}

fn get_str(node: &Value, key: &str) -> String {
    node.get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn children(node: &Value) -> &[Value] {
    node.get("children")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn nested_str(node: &Value, keys: &[&str]) -> String {
    let mut current = node;
    for key in keys {
        match current.get(*key) {
            Some(next) => current = next,
            None => return String::new(),
        }
    }
    current.as_str().unwrap_or("").to_string()
}

fn normalize_whitespace(value: &str) -> String {
    let stripped = value.replace(['\u{200B}', '\u{FEFF}'], "").replace('\r', "");
    let mut out = String::with_capacity(stripped.len());
    let chars: Vec<char> = stripped.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == ' ' || chars[i] == '\t' {
            let mut j = i;
            while j < chars.len() && (chars[j] == ' ' || chars[j] == '\t') {
                j += 1;
            }
            if j < chars.len() && chars[j] == '\n' {
                out.push('\n');
                i = j + 1;
                continue;
            }
            out.extend(chars[i..j].iter());
            i = j;
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn normalize_text(value: &str) -> String {
    normalize_whitespace(value)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn escape_table_cell(value: &str) -> String {
    value.replace('|', "\\|")
}

fn decorate_marked_text(text: &str, decorator: impl Fn(&str) -> String) -> String {
    let leading: String = text.chars().take_while(|c| c.is_whitespace()).collect();
    let trailing: String = text
        .chars()
        .rev()
        .take_while(|c| c.is_whitespace())
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    let core_end = text.len().saturating_sub(trailing.len());
    let core = &text[leading.len()..core_end];
    if core.is_empty() {
        return text.to_string();
    }
    format!("{leading}{}{trailing}", decorator(core))
}

fn has_inline_highlight(node: &Value) -> bool {
    if node.get("highlight").and_then(Value::as_bool) == Some(true) {
        return true;
    }
    for key in ["bgColor", "backgroundColor"] {
        if let Some(color) = node.get(key).and_then(Value::as_str) {
            let trimmed = color.trim();
            if !trimmed.is_empty() && !trimmed.eq_ignore_ascii_case("transparent") {
                return true;
            }
        }
    }
    false
}

fn apply_marks(text: &str, node: &Value) -> String {
    if text.is_empty() {
        return String::new();
    }
    let mut output = text.to_string();
    if node.get("code").and_then(Value::as_bool) == Some(true) {
        output = decorate_marked_text(&output, |v| format!("`{v}`"));
    }
    if node.get("bold").and_then(Value::as_bool) == Some(true) {
        output = decorate_marked_text(&output, |v| format!("**{v}**"));
    }
    if node.get("italic").and_then(Value::as_bool) == Some(true) {
        output = decorate_marked_text(&output, |v| format!("*{v}*"));
    }
    if node.get("strikethrough").and_then(Value::as_bool) == Some(true)
        || node.get("strike").and_then(Value::as_bool) == Some(true)
    {
        output = decorate_marked_text(&output, |v| format!("~~{v}~~"));
    }
    if node.get("underline").and_then(Value::as_bool) == Some(true) {
        output = decorate_marked_text(&output, |v| format!("<u>{v}</u>"));
    }
    if has_inline_highlight(node) {
        output = decorate_marked_text(&output, |v| format!("<mark>{v}</mark>"));
    }
    output
}

fn needs_inline_space(left: &str, right: &str) -> bool {
    let left_trimmed = left.trim_end();
    let right_trimmed = right.trim_start();
    if left_trimmed.is_empty() || right_trimmed.is_empty() {
        return false;
    }
    let first = right_trimmed.chars().next().unwrap_or('\0');
    if first != '@' && first != '[' {
        return false;
    }
    let last = left_trimmed.chars().last().unwrap_or('\0');
    const NO_SPACE: &str = "：:，。、；！？,.!?([{（【「『";
    if NO_SPACE.contains(last) {
        return false;
    }
    last == '@'
        || last == ')'
        || last == ']'
        || last == '）'
        || last == '】'
        || last == '」'
        || last == '』'
        || last.is_ascii_alphanumeric()
        || last == '_'
        || ('\u{4e00}'..='\u{9fff}').contains(&last)
}

fn join_inline_fragments(parts: impl IntoIterator<Item = String>) -> String {
    let mut output = String::new();
    for part in parts {
        if part.is_empty() {
            continue;
        }
        if needs_inline_space(&output, &part) {
            output.push(' ');
        }
        output.push_str(&part);
    }
    output
}

fn inline_node_to_markdown(node: &Value) -> String {
    if node.is_null() {
        return String::new();
    }
    if let Some(items) = node.as_array() {
        return join_inline_fragments(items.iter().map(inline_node_to_markdown));
    }
    if let Some(text) = node.as_str() {
        return normalize_whitespace(text);
    }
    if let Some(text) = node.get("text").and_then(Value::as_str) {
        return apply_marks(&normalize_whitespace(text), node);
    }
    match node_type(node) {
        "mention" => {
            let name = nested_str(node, &["value", "name"]);
            if name.is_empty() {
                String::new()
            } else {
                format!("@{name}")
            }
        }
        "docfile" => {
            let title = nested_str(node, &["value", "title"]);
            let id = nested_str(node, &["value", "id"]);
            if !title.is_empty() && !id.is_empty() {
                format!("[{title}](https://joyspace.jd.com/pages/{id})")
            } else if !title.is_empty() {
                format!("[doc] {title}")
            } else {
                String::new()
            }
        }
        "icon-link" => {
            let title = normalize_text(&if !get_str(node, "title").is_empty() {
                get_str(node, "title")
            } else {
                nested_str(node, &["value", "title"])
            });
            let url = normalize_text(&if !get_str(node, "url").is_empty() {
                get_str(node, "url")
            } else {
                nested_str(node, &["value", "url"])
            });
            if !title.is_empty() && !url.is_empty() {
                format!("[{title}]({url})")
            } else if !title.is_empty() {
                title
            } else {
                url
            }
        }
        "link" => {
            let label = normalize_text(&inline_node_to_markdown(&Value::Array(children(node).to_vec())));
            let href = normalize_text(
                &[
                    get_str(node, "url"),
                    get_str(node, "href"),
                    get_str(node, "link"),
                    nested_str(node, &["value", "url"]),
                ]
                .into_iter()
                .find(|s| !s.is_empty())
                .unwrap_or_default()
                .as_str(),
            );
            let href = if href.is_empty() {
                label.clone()
            } else {
                href
            };
            if href.is_empty() {
                label
            } else if label.is_empty() {
                href
            } else {
                format!("[{label}]({href})")
            }
        }
        _ => {
            if !children(node).is_empty() {
                join_inline_fragments(children(node).iter().map(inline_node_to_markdown))
            } else {
                String::new()
            }
        }
    }
}

fn inline_children_to_markdown(node: &Value) -> String {
    normalize_whitespace(&inline_node_to_markdown(node))
        .trim()
        .to_string()
}

fn inline_children_slice(nodes: &[Value]) -> String {
    inline_children_to_markdown(&Value::Array(nodes.to_vec()))
}

fn block_plain_text(block: &Value) -> String {
    if let Some(items) = block.as_array() {
        return normalize_text(&join_inline_fragments(
            items.iter().map(|item| block_plain_text(item)),
        ));
    }
    if let Some(text) = block.get("text").and_then(Value::as_str) {
        return normalize_text(text);
    }
    match node_type(block) {
        "mention" => {
            let name = nested_str(block, &["value", "name"]);
            if name.is_empty() {
                String::new()
            } else {
                format!("@{name}")
            }
        }
        "docfile" => inline_node_to_markdown(block),
        "icon-link" => inline_node_to_markdown(block),
        _ => {
            if !children(block).is_empty() {
                normalize_text(&join_inline_fragments(
                    children(block).iter().map(block_plain_text),
                ))
            } else {
                String::new()
            }
        }
    }
}

fn prune_ordered_counters(counters: &mut HashMap<i64, i64>, indent: i64) {
    counters.retain(|&key, _| key <= indent);
}

fn render_blocks(blocks: &[Value], state: &mut ConvertState<'_>) -> String {
    let mut pieces = Vec::new();
    for block in blocks {
        let rendered = render_block(block, state);
        if !rendered.is_empty() {
            pieces.push(rendered);
        }
        if node_type(block) != "list" || block.get("header").is_some() {
            state.ordered_counters.clear();
        }
    }
    pieces.join("")
}

fn lookup_diagram<'a>(state: &'a ConvertState<'_>, diagram_id: &str) -> Option<&'a DiagramInfo> {
    if diagram_id.is_empty() {
        None
    } else {
        state.diagrams.get(diagram_id)
    }
}

fn render_diagram_caption(info: Option<&DiagramInfo>, diagram_id: &str, state: &ConvertState<'_>) -> String {
    let title_text = info
        .map(|d| normalize_text(&d.title))
        .unwrap_or_default();
    let headline = if title_text.is_empty() {
        "**JoySpace 绘图**".to_string()
    } else {
        format!("**JoySpace 绘图：{title_text}**")
    };
    let page_url = info
        .map(|d| d.page_url.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or(state.page_url.as_str());
    let mut meta = Vec::new();
    if !page_url.is_empty() {
        meta.push(format!("- 原页面：{page_url}"));
    }
    if let Some(info) = info {
        if !info.link_url.is_empty() {
            meta.push(format!(
                "- [drawio 源文件（签名链接，短期有效）]({})",
                info.link_url
            ));
        }
    }
    if !diagram_id.is_empty() {
        meta.push(format!("- diagramId: `{diagram_id}`"));
    }
    if meta.is_empty() {
        format!("{headline}\n\n")
    } else {
        format!("{headline}\n\n{}\n\n", meta.join("\n"))
    }
}

fn render_diagram_link_lines(
    info: Option<&DiagramInfo>,
    diagram_id: &str,
    state: &ConvertState<'_>,
) -> String {
    let title_text = info
        .map(|d| normalize_text(&d.title))
        .unwrap_or_default();
    let mut lines = Vec::new();
    if title_text.is_empty() {
        lines.push("> **JoySpace 绘图**".to_string());
    } else {
        lines.push(format!("> **JoySpace 绘图：{title_text}**"));
    }
    lines.push(">".to_string());
    let page_url = info
        .map(|d| d.page_url.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or(state.page_url.as_str());
    if !page_url.is_empty() {
        lines.push(format!("> - 原页面：{page_url}"));
    }
    if let Some(info) = info {
        if !info.link_url.is_empty() {
            lines.push(format!(
                "> - drawio 源文件（签名链接，短期有效）：{}",
                info.link_url
            ));
        }
    }
    if !diagram_id.is_empty() {
        lines.push(format!("> - diagramId: {diagram_id}"));
    }
    lines.join("\n")
}

fn render_diagram_block(block: &Value, state: &mut ConvertState<'_>) -> String {
    let diagram_id = normalize_text(&if !get_str(block, "diagramId").is_empty() {
        get_str(block, "diagramId")
    } else {
        get_str(block, "id")
    });
    let info = lookup_diagram(state, &diagram_id).cloned();
    if let Some(info) = info.as_ref() {
        if !info.svg.trim().is_empty() {
            state.warnings.insert("diagram-svg".into());
            return format!(
                "{}{}\n\n",
                render_diagram_caption(Some(info), &diagram_id, state),
                info.svg.trim()
            );
        }
        if !info.mermaid.trim().is_empty() {
            state.warnings.insert("diagram-mermaid".into());
            return format!(
                "{}```mermaid\n{}\n```\n\n",
                render_diagram_caption(Some(info), &diagram_id, state),
                info.mermaid.trim()
            );
        }
        state.warnings.insert("diagram-link".into());
        return format!(
            "{}\n\n",
            render_diagram_link_lines(Some(info), &diagram_id, state)
        );
    }
    state.warnings.insert("diagram".into());
    format!(
        "> JoySpace diagram not exported (diagramId: {})\n\n",
        if diagram_id.is_empty() {
            "unknown"
        } else {
            diagram_id.as_str()
        }
    )
}

fn render_diagram_inline_fallback(block: &Value, state: &mut ConvertState<'_>) -> String {
    let diagram_id = normalize_text(&if !get_str(block, "diagramId").is_empty() {
        get_str(block, "diagramId")
    } else {
        get_str(block, "id")
    });
    let info = lookup_diagram(state, &diagram_id).cloned();
    if info.is_none() {
        state.warnings.insert("diagram".into());
        return format!(
            "JoySpace diagram not exported (diagramId: {})",
            if diagram_id.is_empty() {
                "unknown"
            } else {
                diagram_id.as_str()
            }
        );
    }
    state.warnings.insert("diagram-link".into());
    let title_text = info
        .as_ref()
        .map(|d| normalize_text(&d.title))
        .unwrap_or_default();
    let label = if title_text.is_empty() {
        format!("JoySpace 绘图（diagramId: {diagram_id}）")
    } else {
        format!("JoySpace 绘图：{title_text}")
    };
    let href = info
        .as_ref()
        .map(|d| d.page_url.clone())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| state.page_url.clone());
    if href.is_empty() {
        label
    } else {
        format!("[{label}]({href})")
    }
}

fn render_attachment_block(block: &Value) -> String {
    let file_name = normalize_text(&nested_str(block, &["value", "fileName"]));
    let file_id = normalize_text(&nested_str(block, &["value", "fileId"]));
    if !file_id.is_empty() {
        return format!("- [{file_name}](https://joyspace.jd.com/api/files/{file_id})\n");
    }
    let relative_url = normalize_text(&nested_str(block, &["value", "url"]));
    if !relative_url.is_empty() {
        return format!("- [{file_name}](https://joyspace.jd.com{relative_url})\n");
    }
    if file_name.is_empty() {
        String::new()
    } else {
        format!("- {file_name}\n")
    }
}

fn render_list_block(block: &Value, state: &mut ConvertState<'_>) -> String {
    if let Some(header) = block.get("header") {
        let heading_text = block_plain_text(&Value::Array(children(block).to_vec()));
        state.ordered_counters.clear();
        if heading_text.is_empty() {
            return String::new();
        }
        let level = header
            .as_i64()
            .or_else(|| header.as_u64().map(|n| n as i64))
            .unwrap_or(1)
            + 1;
        let level = level.clamp(2, 6) as usize;
        return format!("{} {heading_text}\n\n", "#".repeat(level));
    }

    let indent = block
        .get("indent")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0);
    prune_ordered_counters(&mut state.ordered_counters, indent);

    let value = get_str(block, "value");
    let prefix = if value == "checkbox" {
        state.ordered_counters.remove(&indent);
        if block.get("checked").and_then(Value::as_bool) == Some(true) {
            "- [x] "
        } else {
            "- [ ] "
        }
        .to_string()
    } else if value == "ordered" {
        let next = state.ordered_counters.get(&indent).copied().unwrap_or(0) + 1;
        state.ordered_counters.insert(indent, next);
        format!("{next}. ")
    } else {
        state.ordered_counters.remove(&indent);
        "- ".to_string()
    };

    let content = inline_children_slice(children(block));
    if content.is_empty() {
        return String::new();
    }
    format!("{}{prefix}{content}\n", "  ".repeat(indent as usize))
}

fn render_table_cell_block(block: &Value, state: &mut ConvertState<'_>) -> String {
    match node_type(block) {
        "p" => inline_children_slice(children(block)),
        "img" => {
            let src = normalize_text(&if !get_str(block, "url").is_empty() {
                get_str(block, "url")
            } else {
                get_str(block, "src")
            });
            if src.is_empty() {
                String::new()
            } else {
                format!("![image]({src})")
            }
        }
        "list" => {
            let content = inline_children_slice(children(block));
            if content.is_empty() {
                return String::new();
            }
            if block.get("header").is_some() {
                state.ordered_counters.clear();
                return content;
            }
            let indent = block
                .get("indent")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                .max(0);
            let indent_prefix = "  ".repeat(indent as usize);
            let value = get_str(block, "value");
            if value == "checkbox" {
                prune_ordered_counters(&mut state.ordered_counters, indent);
                state.ordered_counters.remove(&indent);
                let mark = if block.get("checked").and_then(Value::as_bool) == Some(true) {
                    "[x]"
                } else {
                    "[ ]"
                };
                format!("{indent_prefix}{mark} {content}")
            } else if value == "ordered" {
                prune_ordered_counters(&mut state.ordered_counters, indent);
                let next = state.ordered_counters.get(&indent).copied().unwrap_or(0) + 1;
                state.ordered_counters.insert(indent, next);
                format!("{indent_prefix}{next}. {content}")
            } else {
                prune_ordered_counters(&mut state.ordered_counters, indent);
                state.ordered_counters.remove(&indent);
                format!("{indent_prefix}- {content}")
            }
        }
        "text-draw" => children(block)
            .iter()
            .map(|line| inline_children_slice(children(line)))
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("<br>"),
        "highlight-block" | "foldable-block" => children(block)
            .iter()
            .map(|child| render_table_cell_block(child, state))
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("<br>"),
        "divider" => "---".into(),
        "diagram" => render_diagram_inline_fallback(block, state),
        _ => {
            if block.get("text").and_then(Value::as_str).is_some() {
                inline_children_to_markdown(&Value::Array(vec![block.clone()]))
            } else if children(block).iter().any(|child| child.get("type").is_some()) {
                children(block)
                    .iter()
                    .map(|child| render_table_cell_block(child, state))
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
                    .join("<br>")
            } else {
                inline_children_slice(children(block))
            }
        }
    }
}

fn table_cell_to_markdown(cell: &Value, parent: &mut ConvertState<'_>) -> String {
    let mut nested_warnings = HashSet::new();
    let mut nested = ConvertState {
        warnings: &mut nested_warnings,
        ordered_counters: HashMap::new(),
        diagrams: parent.diagrams,
        page_url: parent.page_url.clone(),
    };
    let lines = children(cell)
        .iter()
        .map(|child| render_table_cell_block(child, &mut nested))
        .map(|line| line.replace('\r', "").replace('\n', "<br>"))
        .map(|line| line.trim_end().to_string())
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    for warning in nested_warnings {
        parent.warnings.insert(warning);
    }
    escape_table_cell(&lines.join("<br>"))
}

fn table_to_markdown(block: &Value, state: &mut ConvertState<'_>) -> String {
    let rows: Vec<&Value> = children(block)
        .iter()
        .filter(|item| node_type(item) == "table-row")
        .collect();
    if rows.is_empty() {
        return String::new();
    }
    let rendered_rows: Vec<Vec<String>> = rows
        .into_iter()
        .map(|row| {
            children(row)
                .iter()
                .filter(|cell| node_type(cell) == "table-cell")
                .map(|cell| table_cell_to_markdown(cell, state))
                .collect()
        })
        .filter(|row: &Vec<String>| !row.is_empty())
        .collect();
    if rendered_rows.is_empty() {
        return String::new();
    }
    let mut output = String::new();
    for (index, row) in rendered_rows.iter().enumerate() {
        output.push_str(&format!("| {} |\n", row.join(" | ")));
        if index == 0 {
            output.push_str(&format!(
                "| {} |\n",
                row.iter().map(|_| "---").collect::<Vec<_>>().join(" | ")
            ));
        }
    }
    output.push('\n');
    output
}

fn code_block_to_markdown(block: &Value) -> String {
    let language = normalize_text(&if !get_str(block, "lang").is_empty() {
        get_str(block, "lang")
    } else {
        get_str(block, "language")
    })
    .to_lowercase();
    let language = if language.is_empty() {
        "text".to_string()
    } else {
        language
    };
    let code = children(block)
        .iter()
        .map(|line| inline_children_slice(children(line)))
        .collect::<Vec<_>>()
        .join("\n")
        .trim_end_matches('\n')
        .to_string();
    format!("```{language}\n{code}\n```\n\n")
}

fn quote_markdown(markdown: &str) -> String {
    markdown
        .trim()
        .split('\n')
        .map(|line| {
            if line.is_empty() {
                ">".to_string()
            } else {
                format!("> {line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn fix_inline_spacing(line: &str) -> String {
    let mut output = String::new();
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        output.push(chars[i]);
        if chars[i] == '@' {
            let mut j = i + 1;
            while j < chars.len() {
                let ch = chars[j];
                if ch.is_whitespace() || ch == '@' || ch == '|' || ch == '<' {
                    break;
                }
                output.push(ch);
                j += 1;
            }
            if j < chars.len() && chars[j] == '@' {
                output.push(' ');
            }
            i = j;
            continue;
        }
        i += 1;
    }
    output
}

fn format_markdown(markdown: &str) -> String {
    let mut result = Vec::new();
    let mut in_code_fence = false;
    let mut in_svg_block = false;
    for raw_line in markdown.split('\n') {
        let mut line = if in_svg_block {
            raw_line.to_string()
        } else {
            fix_inline_spacing(raw_line.trim_end())
        };
        if !in_svg_block && line.trim().starts_with("```") {
            in_code_fence = !in_code_fence;
            result.push(line);
            continue;
        }
        if !in_code_fence && !in_svg_block && line.to_lowercase().contains("<svg") {
            in_svg_block = true;
        }
        if in_svg_block {
            let closed = line.to_lowercase().contains("</svg>");
            result.push(line);
            if closed {
                in_svg_block = false;
            }
            continue;
        }
        result.push(std::mem::take(&mut line));
    }
    let joined = result.join("\n");
    let mut collapsed = joined;
    while collapsed.contains("\n\n\n") {
        collapsed = collapsed.replace("\n\n\n", "\n\n");
    }
    collapsed.trim_end().to_string()
}

fn render_block(block: &Value, state: &mut ConvertState<'_>) -> String {
    if !block.is_object() {
        return String::new();
    }
    match node_type(block) {
        "p" => {
            let content = inline_children_slice(children(block));
            if content.is_empty() {
                String::new()
            } else {
                format!("{content}\n\n")
            }
        }
        "list" => render_list_block(block, state),
        "table" => table_to_markdown(block, state),
        "highlight-block" => {
            let mut nested = ConvertState {
                warnings: state.warnings,
                ordered_counters: HashMap::new(),
                diagrams: state.diagrams,
                page_url: state.page_url.clone(),
            };
            let nested_md = render_blocks(children(block), &mut nested);
            if nested_md.trim().is_empty() {
                String::new()
            } else {
                format!("{}\n\n", quote_markdown(&nested_md))
            }
        }
        "foldable-block" => {
            let summary = {
                let name = normalize_text(&get_str(block, "name"));
                if name.is_empty() {
                    "Foldable Block".to_string()
                } else {
                    name
                }
            };
            let mut nested = ConvertState {
                warnings: state.warnings,
                ordered_counters: HashMap::new(),
                diagrams: state.diagrams,
                page_url: state.page_url.clone(),
            };
            let nested_md = render_blocks(children(block), &mut nested)
                .trim()
                .to_string();
            if nested_md.is_empty() {
                format!("\n<details><summary>{summary}</summary>\n\n</details>\n\n")
            } else {
                format!("\n<details><summary>{summary}</summary>\n\n{nested_md}\n\n</details>\n\n")
            }
        }
        "text-draw" | "block-code" => code_block_to_markdown(block),
        "img" => {
            let src = normalize_text(&if !get_str(block, "url").is_empty() {
                get_str(block, "url")
            } else {
                get_str(block, "src")
            });
            if src.is_empty() {
                String::new()
            } else {
                format!("![image]({src})\n\n")
            }
        }
        "attachment" => render_attachment_block(block),
        "divider" => "---\n\n".into(),
        "diagram" => render_diagram_block(block, state),
        other => {
            if children(block)
                .iter()
                .any(|item| item.get("type").is_some())
            {
                state.warnings.insert(if other.is_empty() {
                    "unknown".into()
                } else {
                    other.into()
                });
                let mut nested = ConvertState {
                    warnings: state.warnings,
                    ordered_counters: HashMap::new(),
                    diagrams: state.diagrams,
                    page_url: state.page_url.clone(),
                };
                render_blocks(children(block), &mut nested)
            } else {
                let content = inline_children_slice(children(block));
                if content.is_empty() {
                    String::new()
                } else {
                    state.warnings.insert(if other.is_empty() {
                        "unknown".into()
                    } else {
                        other.into()
                    });
                    format!("{content}\n\n")
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn converts_main_block_types() {
        let content = json!([
            { "type": "p", "children": [{ "text": "Sample Doc" }] },
            { "type": "list", "header": 1, "children": [{ "text": "Overview" }] },
            {
                "type": "p",
                "children": [
                    { "text": "Hello " },
                    { "bold": true, "text": "world" },
                    { "text": " " },
                    { "strike": true, "text": "old" },
                    { "text": " " },
                    { "underline": true, "text": "new" },
                    { "text": " " },
                    { "type": "mention", "value": { "name": "Alice" }, "children": [{ "text": "" }] },
                    { "type": "mention", "value": { "name": "Carol" }, "children": [{ "text": "" }] }
                ]
            },
            { "type": "list", "value": "ordered", "children": [{ "text": "First item" }] },
            { "type": "list", "value": "ordered", "indent": 1, "children": [{ "text": "Nested item" }] },
            { "type": "list", "value": "checkbox", "children": [{ "text": "Todo item" }] },
            {
                "type": "table",
                "children": [
                    {
                        "type": "table-row",
                        "children": [
                            { "type": "table-cell", "children": [{ "type": "p", "children": [{ "text": "Name" }] }] },
                            { "type": "table-cell", "children": [{ "type": "p", "children": [{ "text": "Owner" }] }] }
                        ]
                    },
                    {
                        "type": "table-row",
                        "children": [
                            { "type": "table-cell", "children": [{ "type": "p", "children": [{ "text": "Task A" }] }] },
                            {
                                "type": "table-cell",
                                "children": [
                                    { "type": "p", "children": [{ "type": "mention", "value": { "name": "Bob" }, "children": [{ "text": "" }] }] },
                                    { "type": "img", "url": "https://example.com/t.png", "children": [{ "text": "" }] }
                                ]
                            }
                        ]
                    }
                ]
            },
            { "type": "highlight-block", "children": [{ "type": "p", "children": [{ "text": "Important note" }] }] },
            { "type": "foldable-block", "name": "Details", "children": [{ "type": "p", "children": [{ "text": "Hidden content" }] }] },
            {
                "type": "text-draw",
                "lang": "plantuml",
                "children": [
                    { "children": [{ "text": "@startuml" }] },
                    { "children": [{ "text": "Alice -> Bob" }] },
                    { "children": [{ "text": "@enduml" }] }
                ]
            },
            { "type": "img", "url": "https://example.com/diagram.png", "children": [{ "text": "" }] },
            { "type": "divider", "children": [{ "text": "" }] },
            { "type": "p", "children": [{ "type": "icon-link", "title": "需求链接", "url": "https://example.com/spec", "children": [{ "text": "" }] }] },
            { "type": "diagram", "diagramId": "diag-123", "children": [{ "text": "" }] }
        ]);
        let result = joyspace_content_to_markdown(
            "Sample Doc",
            content.as_array().unwrap(),
            &HashMap::new(),
            "",
        );
        assert!(result.markdown.starts_with("# Sample Doc"));
        assert!(!result.markdown.contains("# Sample Doc\n\nSample Doc"));
        assert!(result.markdown.contains("## Overview"));
        assert!(result
            .markdown
            .contains("Hello **world** ~~old~~ <u>new</u> @Alice @Carol"));
        assert!(result.markdown.contains("1. First item"));
        assert!(result.markdown.contains("  1. Nested item"));
        assert!(result.markdown.contains("- [ ] Todo item"));
        assert!(result.markdown.contains("| Name | Owner |"));
        assert!(result.markdown.contains("> Important note"));
        assert!(result.markdown.contains("<details><summary>Details</summary>"));
        assert!(result.markdown.contains("```plantuml"));
        assert!(result
            .markdown
            .contains("![image](https://example.com/diagram.png)"));
        assert!(result
            .markdown
            .contains("[需求链接](https://example.com/spec)"));
        assert!(result
            .markdown
            .contains("> JoySpace diagram not exported (diagramId: diag-123)"));
        assert!(result.warnings.contains(&"diagram".to_string()));
    }

    #[test]
    fn preserves_inline_highlight() {
        let content = json!([
            { "type": "p", "children": [
                { "text": "pre " },
                { "bgColor": "#FDF7AF", "text": "黄色高亮" },
                { "text": " post" }
            ]},
            { "type": "p", "children": [{ "highlight": true, "bold": true, "text": "bold highlight" }] },
            { "type": "p", "children": [{ "backgroundColor": "transparent", "text": "no mark" }] }
        ]);
        let result = joyspace_content_to_markdown("HL", content.as_array().unwrap(), &HashMap::new(), "");
        assert!(result.markdown.contains("pre <mark>黄色高亮</mark> post"));
        assert!(result.markdown.contains("<mark>**bold highlight**</mark>"));
        assert!(!result.markdown.contains("<mark>no mark</mark>"));
    }

    #[test]
    fn diagram_svg_mermaid_link_fallback() {
        let page_url = "https://joyspace.jd.com/pages/demo";
        let svg = "<svg xmlns=\"http://www.w3.org/2000/svg\"><rect/></svg>";
        let mut diagrams = HashMap::new();
        diagrams.insert(
            "d1".into(),
            DiagramInfo {
                title: "T".into(),
                svg: svg.into(),
                mermaid: "flowchart TD\n  n1-->n2".into(),
                ..Default::default()
            },
        );
        let content = json!([{ "type": "diagram", "diagramId": "d1", "children": [{ "text": "" }] }]);
        let svg_result =
            joyspace_content_to_markdown("D", content.as_array().unwrap(), &diagrams, page_url);
        assert!(svg_result.markdown.contains(svg));
        assert!(svg_result.warnings.contains(&"diagram-svg".to_string()));

        let mut diagrams = HashMap::new();
        diagrams.insert(
            "d2".into(),
            DiagramInfo {
                title: "流程".into(),
                mermaid: "flowchart TD\n  n1[\"A\"]\n  n2[\"B\"]\n  n1 --> n2".into(),
                link_url: "https://cdn.example.com/d2.xml?sig=abc".into(),
                ..Default::default()
            },
        );
        let content = json!([{ "type": "diagram", "diagramId": "d2", "children": [{ "text": "" }] }]);
        let mermaid_result =
            joyspace_content_to_markdown("D", content.as_array().unwrap(), &diagrams, page_url);
        assert!(mermaid_result.markdown.contains("```mermaid\nflowchart TD"));
        assert!(mermaid_result.warnings.contains(&"diagram-mermaid".to_string()));

        let mut diagrams = HashMap::new();
        diagrams.insert(
            "d3".into(),
            DiagramInfo {
                title: "流程图 A".into(),
                page_url: page_url.into(),
                ..Default::default()
            },
        );
        let content = json!([{ "type": "diagram", "diagramId": "d3", "children": [{ "text": "" }] }]);
        let link_result =
            joyspace_content_to_markdown("D", content.as_array().unwrap(), &diagrams, page_url);
        assert!(link_result.markdown.contains("> **JoySpace 绘图：流程图 A**"));
        assert!(link_result.warnings.contains(&"diagram-link".to_string()));
    }

    #[test]
    fn table_bare_text_leaves() {
        let content = json!([{
            "type": "table",
            "children": [
                { "type": "table-row", "children": [
                    { "type": "table-cell", "children": [{ "text": "序号" }] },
                    { "type": "table-cell", "children": [{ "text": "流程节点" }] }
                ]},
                { "type": "table-row", "children": [
                    { "type": "table-cell", "children": [{ "text": "1" }] },
                    { "type": "table-cell", "children": [{ "text": "简介页面" }] }
                ]}
            ]
        }]);
        let result =
            joyspace_content_to_markdown("Table", content.as_array().unwrap(), &HashMap::new(), "");
        assert!(result.markdown.contains("| 序号 | 流程节点 |"));
        assert!(result.markdown.contains("| 1 | 简介页面 |"));
    }
}
