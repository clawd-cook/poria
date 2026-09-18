use std::path::{Path, PathBuf};

use poria_skills::{
    bundled_skill_dir_name, bundled_skills_complete, list_bundled_skill_docs, parse_skill_markdown,
    read_bundled_skill_markdown, BundledSkillDoc,
};
use serde::Serialize;
use tauri::{AppHandle, Manager};

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SkillInfo {
    pub description: String,
    pub id: String,
    pub name: String,
    pub version: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SkillDetail {
    pub description: String,
    pub id: String,
    pub markdown: Option<String>,
    pub name: String,
    pub version: String,
}

pub(crate) fn bundled_skills_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../skills");
    if bundled_skills_complete(&dev) {
        return Ok(dev.canonicalize().unwrap_or(dev));
    }
    let resource = app
        .path()
        .resource_dir()
        .map_err(|e| format!("无法解析资源目录: {e}"))?;
    let bundled = resource.join("skills");
    if bundled_skills_complete(&bundled) {
        return Ok(bundled);
    }
    Err(format!(
        "找不到随包 skill 目录（已试 {} 与 {}）",
        dev.display(),
        bundled.display()
    ))
}

fn skill_from_doc(dir: &str, doc: BundledSkillDoc) -> SkillDetail {
    SkillDetail {
        description: doc.description,
        id: dir.to_string(),
        markdown: Some(doc.body),
        name: doc.name,
        version: String::new(),
    }
}

fn load_bundled_skill(root: &Path, dir: &str) -> Result<SkillDetail, String> {
    let raw = read_bundled_skill_markdown(root, dir)?;
    Ok(skill_from_doc(dir, parse_skill_markdown(&raw)?))
}

/// Return metadata for bundled Claude skills (`skills/*/SKILL.md`). Init / Deploy
/// are pipeline stages, not skills. `id` is the bundled directory name.
#[tauri::command]
pub async fn list_skills(app: AppHandle) -> Result<Vec<SkillInfo>, String> {
    let root = bundled_skills_dir(&app)?;
    let listed = list_bundled_skill_docs(&root)?;
    Ok(listed
        .into_iter()
        .map(|(dir, doc)| {
            let detail = skill_from_doc(dir, doc);
            SkillInfo {
                description: detail.description,
                id: detail.id,
                name: detail.name,
                version: detail.version,
            }
        })
        .collect())
}

/// Return one bundled Claude skill. `markdown` is the SKILL.md body without YAML.
#[tauri::command]
pub async fn get_skill(app: AppHandle, skill_id: String) -> Result<SkillDetail, String> {
    let id = skill_id.trim();
    if id.is_empty() {
        return Err("缺少技能 ID".into());
    }
    let dir = bundled_skill_dir_name(id).ok_or_else(|| format!("未知技能: {id}"))?;
    let root = bundled_skills_dir(&app)?;
    load_bundled_skill(&root, dir)
}
