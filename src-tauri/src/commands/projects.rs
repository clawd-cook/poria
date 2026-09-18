use std::fs;
use std::path::Path;

use serde::Serialize;

use poria_infrastructure::auth::{
    assert_path_under_projects_root, demand_project_folder_name, get_demand_project_dir,
};

const MAX_MARKDOWN_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Debug, Serialize, Clone)]
pub struct DemandProjectFile {
    pub name: String,
    pub size: u64,
}

#[derive(Debug, Serialize, Clone)]
pub struct DemandProject {
    pub demand_code: String,
    pub exists: bool,
    pub files: Vec<DemandProjectFile>,
    pub project_dir: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct DemandProjectFileContent {
    pub content: String,
    pub name: String,
}

fn is_safe_markdown_name(name: &str) -> bool {
    let trimmed = name.trim();
    !trimmed.is_empty()
        && !trimmed.starts_with('.')
        && trimmed.ends_with(".md")
        && Path::new(trimmed).components().count() == 1
        && !trimmed.contains('\\')
}

fn resolve_project_dir(
    demand_code: &str,
    demand_id: Option<i64>,
) -> Result<std::path::PathBuf, String> {
    let folder = demand_project_folder_name(demand_code, demand_id.unwrap_or(0));
    get_demand_project_dir(None, &folder)
}

/// List markdown files under `~/.poria/projects/<demand_code>`.
#[tauri::command]
pub async fn list_demand_project(
    demand_code: String,
    demand_id: Option<i64>,
) -> Result<DemandProject, String> {
    let project_dir = resolve_project_dir(&demand_code, demand_id)?;
    assert_path_under_projects_root(None, &project_dir)?;
    if !project_dir.is_dir() {
        return Ok(DemandProject {
            demand_code,
            exists: false,
            files: vec![],
            project_dir: project_dir.to_string_lossy().to_string(),
        });
    }

    let mut files = Vec::new();
    for entry in fs::read_dir(&project_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if !is_safe_markdown_name(&name) {
            continue;
        }
        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
        files.push(DemandProjectFile { name, size });
    }
    files.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(DemandProject {
        demand_code,
        exists: true,
        files,
        project_dir: project_dir.to_string_lossy().to_string(),
    })
}

/// Read one markdown file from a demand project folder.
#[tauri::command]
pub async fn read_demand_project_file(
    demand_code: String,
    file_name: String,
    demand_id: Option<i64>,
) -> Result<DemandProjectFileContent, String> {
    if !is_safe_markdown_name(&file_name) {
        return Err("不支持的文档文件".into());
    }
    let project_dir = resolve_project_dir(&demand_code, demand_id)?;
    let path = project_dir.join(file_name.trim());
    assert_path_under_projects_root(None, &path)?;
    if !path.is_file() {
        return Err("文档不存在".into());
    }
    let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    if size > MAX_MARKDOWN_BYTES {
        return Err("文档过大".into());
    }
    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    Ok(DemandProjectFileContent {
        content,
        name: file_name,
    })
}
