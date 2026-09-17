use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::client::{
    extract_page_id_from_url, fetch_diagram_detail, fetch_page_basic, fetch_page_content,
    JoySpaceAuth,
};
use super::markdown::{joyspace_content_to_markdown, ConversionResult, DiagramInfo};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedDocument {
    pub title: String,
    pub markdown: String,
    pub page_id: String,
    pub effective_page_id: String,
    pub warnings: Vec<String>,
}

fn json_i64(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_u64().map(|n| n as i64))
        .or_else(|| value.as_str().and_then(|s| s.parse().ok()))
}

fn json_str(value: &Value, keys: &[&str]) -> String {
    for key in keys {
        if let Some(text) = value.get(*key).and_then(Value::as_str) {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }
    String::new()
}

fn collect_diagram_ids(nodes: &[Value]) -> Vec<String> {
    let mut ids = Vec::new();
    let mut seen = HashSet::new();
    fn visit(nodes: &[Value], ids: &mut Vec<String>, seen: &mut HashSet<String>) {
        for node in nodes {
            if node.get("type").and_then(Value::as_str) == Some("diagram") {
                let id = node
                    .get("diagramId")
                    .or_else(|| node.get("id"))
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if !id.is_empty() && seen.insert(id.clone()) {
                    ids.push(id);
                }
            }
            if let Some(children) = node.get("children").and_then(Value::as_array) {
                visit(children, ids, seen);
            }
        }
    }
    visit(nodes, &mut ids, &mut seen);
    ids
}

async fn fetch_diagrams(
    auth: &JoySpaceAuth,
    content: &[Value],
    page_id: &str,
    page_url: &str,
) -> HashMap<String, DiagramInfo> {
    let mut diagrams = HashMap::new();
    for diagram_id in collect_diagram_ids(content) {
        match fetch_diagram_detail(auth, &diagram_id, page_id).await {
            Ok(detail) => {
                let title = json_str(&detail, &["title", "name", "fileName"]);
                let link_url = json_str(&detail, &["linkUrl", "link_url", "url"]);
                diagrams.insert(
                    diagram_id,
                    DiagramInfo {
                        title,
                        link_url,
                        page_url: page_url.to_string(),
                        svg: String::new(),
                        mermaid: String::new(),
                    },
                );
            }
            Err(_) => {
                diagrams.insert(
                    diagram_id,
                    DiagramInfo {
                        page_url: page_url.to_string(),
                        ..Default::default()
                    },
                );
            }
        }
    }
    diagrams
}

fn conversion_to_export(
    conversion: ConversionResult,
    page_id: String,
    effective_page_id: String,
) -> Result<ExportedDocument, Box<dyn std::error::Error + Send + Sync>> {
    if conversion.markdown.trim().is_empty() {
        return Err("Markdown 转换结果为空。".into());
    }
    let markdown = if conversion.markdown.ends_with('\n') {
        conversion.markdown
    } else {
        format!("{}\n", conversion.markdown)
    };
    Ok(ExportedDocument {
        title: conversion.title,
        markdown,
        page_id,
        effective_page_id,
        warnings: conversion.warnings,
    })
}

/// Fetch a JoySpace page via SSO cookie + POST `/v1/pages/content` and convert to Markdown.
pub async fn export_page_markdown(
    auth: &JoySpaceAuth,
    url: &str,
) -> Result<ExportedDocument, Box<dyn std::error::Error + Send + Sync>> {
    let page_id = extract_page_id_from_url(url)?;
    let basic = fetch_page_basic(auth, &page_id).await?;

    let mut effective_page_id = page_id.clone();
    let mut title_source = basic.clone();
    let page_type = basic
        .get("page_type")
        .or_else(|| basic.get("type"))
        .and_then(json_i64);
    if page_type == Some(5) {
        if let Some(origin_id) = basic.get("origin_id").and_then(Value::as_str) {
            let origin_id = origin_id.trim();
            if !origin_id.is_empty() {
                effective_page_id = origin_id.to_string();
                if let Ok(origin_basic) = fetch_page_basic(auth, &effective_page_id).await {
                    title_source = origin_basic;
                }
            }
        }
    }

    let content_payload = fetch_page_content(auth, &effective_page_id).await?;
    let content = content_payload
        .get("content")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if content.is_empty() {
        return Err(format!(
            "JoySpace API 未返回有效的 data.content。pageId={page_id}"
        )
        .into());
    }

    let diagrams = fetch_diagrams(auth, &content, &effective_page_id, url).await;
    let title = json_str(&title_source, &["title", "full_name"]);
    let title = if title.is_empty() {
        json_str(&basic, &["title", "full_name"])
    } else {
        title
    };
    let conversion = joyspace_content_to_markdown(&title, &content, &diagrams, url);
    conversion_to_export(conversion, page_id, effective_page_id)
}

/// Convert an already-fetched content array (used by tests / fixture).
pub fn markdown_from_content(
    title: &str,
    content: &[Value],
    page_url: &str,
) -> Result<ExportedDocument, Box<dyn std::error::Error + Send + Sync>> {
    let conversion = joyspace_content_to_markdown(title, content, &HashMap::new(), page_url);
    conversion_to_export(conversion, String::new(), String::new())
}
