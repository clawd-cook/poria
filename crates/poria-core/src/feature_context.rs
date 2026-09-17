use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub const ARTIFACT_PRD: &str = "PRD.md";
pub const ARTIFACT_PRD_REVIEW: &str = "PRD_REVIEW.md";
pub const ARTIFACT_TRD: &str = "TRD.md";
pub const ARTIFACT_BACKEND_TRD: &str = "BACKEND_TRD.md";
pub const ARTIFACT_TASK: &str = "TASK.md";
pub const ARTIFACT_CR: &str = "CR.md";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureContext {
    pub id: String,
    pub pipeline_id: String,
    pub demand_id: i64,
    pub root: PathBuf,
    pub created_at: DateTime<Utc>,
}

impl FeatureContext {
    pub fn create(
        app_data_dir: &Path,
        pipeline_id: &str,
        demand_id: i64,
    ) -> Result<Self, std::io::Error> {
        let root = app_data_dir.join("features").join(pipeline_id);
        fs::create_dir_all(&root)?;

        Ok(Self {
            id: pipeline_id.to_string(),
            pipeline_id: pipeline_id.to_string(),
            demand_id,
            root,
            created_at: Utc::now(),
        })
    }

    pub fn create_at(
        root: PathBuf,
        pipeline_id: &str,
        demand_id: i64,
    ) -> Result<Self, std::io::Error> {
        fs::create_dir_all(&root)?;
        Ok(Self {
            id: pipeline_id.to_string(),
            pipeline_id: pipeline_id.to_string(),
            demand_id,
            root,
            created_at: Utc::now(),
        })
    }

    pub fn from_existing(app_data_dir: &Path, pipeline_id: &str) -> Option<Self> {
        let root = app_data_dir.join("features").join(pipeline_id);
        if root.is_dir() {
            Some(Self {
                id: pipeline_id.to_string(),
                pipeline_id: pipeline_id.to_string(),
                demand_id: 0,
                root,
                created_at: Utc::now(),
            })
        } else {
            None
        }
    }

    pub fn from_root(root: &Path) -> Option<Self> {
        if root.is_dir() {
            let id = root
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            Some(Self {
                id: id.clone(),
                pipeline_id: id,
                demand_id: 0,
                root: root.to_path_buf(),
                created_at: Utc::now(),
            })
        } else {
            None
        }
    }

    pub fn artifact_path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    pub fn has_artifact(&self, name: &str) -> bool {
        self.artifact_path(name).is_file()
    }

    pub fn list_artifacts(&self) -> Vec<String> {
        let Ok(entries) = fs::read_dir(&self.root) else {
            return vec![];
        };
        entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
            .filter_map(|e| e.file_name().into_string().ok())
            .collect()
    }

    pub fn write_artifact(&self, name: &str, content: &str) -> Result<(), std::io::Error> {
        let path = self.artifact_path(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, content)
    }

    pub fn read_artifact(&self, name: &str) -> Result<Option<String>, std::io::Error> {
        let path = self.artifact_path(name);
        if path.is_file() {
            fs::read_to_string(&path).map(Some)
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("poria-fc-test-{}", nanoid::nanoid!(8)));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_create_at_uses_given_root() {
        let base = temp_dir();
        let root = base.join("R2026082156824");
        let ctx = FeatureContext::create_at(root.clone(), "pipe-012", 99).unwrap();
        assert_eq!(ctx.root, root);
        assert!(ctx.root.is_dir());
        assert_eq!(ctx.demand_id, 99);
        ctx.write_artifact(ARTIFACT_PRD, "# prd").unwrap();
        assert!(ctx.has_artifact(ARTIFACT_PRD));
        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn test_create_makes_directory() {
        let base = temp_dir();
        let ctx = FeatureContext::create(&base, "pipe-001", 42).unwrap();
        assert!(ctx.root.is_dir());
        assert_eq!(ctx.id, "pipe-001");
        assert_eq!(ctx.pipeline_id, "pipe-001");
        assert_eq!(ctx.demand_id, 42);
        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn test_from_existing_found() {
        let base = temp_dir();
        fs::create_dir_all(base.join("features/pipe-002")).unwrap();
        let ctx = FeatureContext::from_existing(&base, "pipe-002");
        assert!(ctx.is_some());
        let ctx = ctx.unwrap();
        assert_eq!(ctx.pipeline_id, "pipe-002");
        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn test_from_existing_not_found() {
        let base = temp_dir();
        assert!(FeatureContext::from_existing(&base, "nonexistent").is_none());
        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn test_write_and_read_artifact() {
        let base = temp_dir();
        let ctx = FeatureContext::create(&base, "pipe-003", 1).unwrap();
        ctx.write_artifact(ARTIFACT_PRD, "# My PRD\n\nDetails here.")
            .unwrap();
        assert!(ctx.has_artifact(ARTIFACT_PRD));
        let content = ctx.read_artifact(ARTIFACT_PRD).unwrap();
        assert_eq!(content.as_deref(), Some("# My PRD\n\nDetails here."));
        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn test_read_nonexistent_artifact() {
        let base = temp_dir();
        let ctx = FeatureContext::create(&base, "pipe-004", 1).unwrap();
        assert!(!ctx.has_artifact(ARTIFACT_TRD));
        let content = ctx.read_artifact(ARTIFACT_TRD).unwrap();
        assert!(content.is_none());
        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn test_list_artifacts_empty() {
        let base = temp_dir();
        let ctx = FeatureContext::create(&base, "pipe-005", 1).unwrap();
        assert!(ctx.list_artifacts().is_empty());
        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn test_list_artifacts_multiple() {
        let base = temp_dir();
        let ctx = FeatureContext::create(&base, "pipe-006", 1).unwrap();
        ctx.write_artifact(ARTIFACT_PRD, "prd").unwrap();
        ctx.write_artifact(ARTIFACT_PRD_REVIEW, "review").unwrap();
        ctx.write_artifact(ARTIFACT_TRD, "trd").unwrap();

        let mut artifacts = ctx.list_artifacts();
        artifacts.sort();
        assert_eq!(artifacts, vec!["PRD.md", "PRD_REVIEW.md", "TRD.md"]);
        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn test_write_artifact_nested_path() {
        let base = temp_dir();
        let ctx = FeatureContext::create(&base, "pipe-007", 1).unwrap();
        ctx.write_artifact("source/api-raw.md", "raw api content")
            .unwrap();
        assert!(ctx.has_artifact("source/api-raw.md"));
        let content = ctx.read_artifact("source/api-raw.md").unwrap();
        assert_eq!(content.as_deref(), Some("raw api content"));
        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn test_artifact_path() {
        let base = temp_dir();
        let ctx = FeatureContext::create(&base, "pipe-008", 1).unwrap();
        let expected = ctx.root.join("TRD.md");
        assert_eq!(ctx.artifact_path(ARTIFACT_TRD), expected);
        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn test_create_idempotent() {
        let base = temp_dir();
        let ctx1 = FeatureContext::create(&base, "pipe-009", 1).unwrap();
        ctx1.write_artifact(ARTIFACT_PRD, "first").unwrap();
        let ctx2 = FeatureContext::create(&base, "pipe-009", 1).unwrap();
        assert!(ctx2.has_artifact(ARTIFACT_PRD));
        let content = ctx2.read_artifact(ARTIFACT_PRD).unwrap();
        assert_eq!(content.as_deref(), Some("first"));
        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn test_overwrite_artifact() {
        let base = temp_dir();
        let ctx = FeatureContext::create(&base, "pipe-010", 1).unwrap();
        ctx.write_artifact(ARTIFACT_TRD, "v1").unwrap();
        ctx.write_artifact(ARTIFACT_TRD, "v2").unwrap();
        let content = ctx.read_artifact(ARTIFACT_TRD).unwrap();
        assert_eq!(content.as_deref(), Some("v2"));
        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn test_backend_trd_artifact_does_not_replace_frontend_trd() {
        let base = temp_dir();
        let ctx = FeatureContext::create(&base, "pipe-011", 1).unwrap();
        ctx.write_artifact(ARTIFACT_TRD, "frontend trd").unwrap();
        ctx.write_artifact(ARTIFACT_BACKEND_TRD, "backend trd")
            .unwrap();
        assert_eq!(ARTIFACT_BACKEND_TRD, "BACKEND_TRD.md");
        assert_ne!(ARTIFACT_BACKEND_TRD, ARTIFACT_TRD);
        assert_eq!(
            ctx.read_artifact(ARTIFACT_TRD).unwrap().as_deref(),
            Some("frontend trd")
        );
        assert_eq!(
            ctx.read_artifact(ARTIFACT_BACKEND_TRD).unwrap().as_deref(),
            Some("backend trd")
        );
        fs::remove_dir_all(&base).ok();
    }
}
