use std::fs;
use std::path::{Path, PathBuf};

use poria_core::workflow::{parse_and_validate, WorkflowDocument, DEFAULT_WORKFLOW_ID};

use crate::auth::get_workflows_root;

pub fn load_workflow(
    workflow_id: &str,
    bundled_dir: &Path,
    home: Option<&Path>,
) -> Result<WorkflowDocument, String> {
    let yaml = load_workflow_yaml(workflow_id, bundled_dir, home)?;
    parse_and_validate(&yaml).map_err(|err| err.to_string())
}

pub fn load_workflow_yaml(
    workflow_id: &str,
    bundled_dir: &Path,
    home: Option<&Path>,
) -> Result<String, String> {
    let id = sanitize_workflow_id(workflow_id)?;
    let override_path = get_workflows_root(home).join(format!("{id}.yml"));
    if override_path.is_file() {
        return fs::read_to_string(&override_path)
            .map_err(|err| format!("读取工作流覆盖失败 {}: {err}", override_path.display()));
    }
    let bundled_yml = bundled_dir.join(format!("{id}.yml"));
    if bundled_yml.is_file() {
        return fs::read_to_string(&bundled_yml)
            .map_err(|err| format!("读取随包工作流失败 {}: {err}", bundled_yml.display()));
    }
    let bundled_yaml = bundled_dir.join(format!("{id}.yaml"));
    if bundled_yaml.is_file() {
        return fs::read_to_string(&bundled_yaml)
            .map_err(|err| format!("读取随包工作流失败 {}: {err}", bundled_yaml.display()));
    }
    Err(format!(
        "找不到工作流 {id}（已试 {} 与 {}）",
        override_path.display(),
        bundled_yml.display()
    ))
}

pub fn default_bundled_workflows_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../workflows")
}

pub fn load_default_workflow(
    bundled_dir: Option<&Path>,
    home: Option<&Path>,
) -> Result<WorkflowDocument, String> {
    let default_dir = default_bundled_workflows_dir();
    let dir = bundled_dir.unwrap_or(default_dir.as_path());
    load_workflow(DEFAULT_WORKFLOW_ID, dir, home)
}

fn sanitize_workflow_id(workflow_id: &str) -> Result<&str, String> {
    let id = workflow_id.trim();
    if id.is_empty() {
        return Err("workflow id 不能为空".into());
    }
    if !id
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        return Err(format!("非法 workflow id: {id}"));
    }
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use poria_core::types::STAGE_ORDER;

    #[test]
    fn bundled_demand_to_mr_loads_from_repo_workflows() {
        let doc = load_default_workflow(None, None).unwrap();
        assert_eq!(doc.name, "demand-to-mr");
        assert_eq!(doc.jobs.len(), STAGE_ORDER.len());
    }

    #[test]
    fn override_yml_wins_over_bundle() {
        let home = tempfile::tempdir().unwrap();
        let workflows = home.path().join(".poria").join("workflows");
        fs::create_dir_all(&workflows).unwrap();
        fs::write(
            workflows.join("demand-to-mr.yml"),
            r#"
name: override
jobs:
  init:
    steps:
      - uses: skill:init
"#,
        )
        .unwrap();
        let doc = load_workflow(
            "demand-to-mr",
            &default_bundled_workflows_dir(),
            Some(home.path()),
        )
        .unwrap();
        assert_eq!(doc.name, "override");
        assert_eq!(doc.jobs.len(), 1);
    }

    #[test]
    fn unknown_action_in_override_is_rejected() {
        let home = tempfile::tempdir().unwrap();
        let workflows = home.path().join(".poria").join("workflows");
        fs::create_dir_all(&workflows).unwrap();
        fs::write(
            workflows.join("demand-to-mr.yml"),
            r#"
name: override
jobs:
  init:
    steps:
      - uses: skill:nope
"#,
        )
        .unwrap();
        let err = load_workflow(
            "demand-to-mr",
            &default_bundled_workflows_dir(),
            Some(home.path()),
        )
        .unwrap_err();
        assert!(err.contains("非法 uses"), "{err}");
    }

    #[test]
    fn path_traversal_id_is_rejected() {
        let err = sanitize_workflow_id("../etc/passwd").unwrap_err();
        assert!(err.contains("非法 workflow id"));
    }
}
