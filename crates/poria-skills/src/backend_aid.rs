use std::collections::HashMap;

use poria_core::feature_context::{FeatureContext, ARTIFACT_BACKEND_TRD};
use poria_core::types::PipelineConfig;

/// Inject read-only backend TRD / repo vars for frontend design and coding prompts.
/// Missing URL, markdown export, or backend context become empty strings.
pub fn insert_backend_coding_aid_vars(
    vars: &mut HashMap<String, String>,
    config: &PipelineConfig,
    feature_ctx: &FeatureContext,
) {
    vars.insert(
        "backend_trd_url".into(),
        config.backend_trd_url.clone().unwrap_or_default(),
    );
    vars.insert(
        "backend_trd_content".into(),
        feature_ctx
            .read_artifact(ARTIFACT_BACKEND_TRD)
            .ok()
            .flatten()
            .unwrap_or_default(),
    );
    match config.backend_context.as_ref() {
        Some(backend) => {
            vars.insert("backend_repo_path".into(), backend.local_path.clone());
            vars.insert("backend_branch".into(), backend.branch.clone());
        }
        None => {
            vars.insert("backend_repo_path".into(), String::new());
            vars.insert("backend_branch".into(), String::new());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace_layout::repo_bundled_skills_dir;
    use poria_core::feature_context::ARTIFACT_TRD;
    use poria_core::types::BackendContext;
    use std::fs;
    use std::path::PathBuf;

    fn temp_dir(suffix: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("poria-backend-aid-{suffix}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn insert_backend_coding_aid_vars_reads_config_and_optional_artifact() {
        let base = temp_dir("present");
        let ctx = FeatureContext::create(&base, "pipe-aid", 1).unwrap();
        ctx.write_artifact(ARTIFACT_TRD, "frontend trd must not leak")
            .unwrap();
        ctx.write_artifact(ARTIFACT_BACKEND_TRD, "# backend trd")
            .unwrap();

        let config = PipelineConfig {
            backend_trd_url: Some("https://joyspace.jd.com/pages/be".into()),
            backend_context: Some(BackendContext {
                git_url: "git@coding.jd.com:ls/ls-api.git".into(),
                local_path: "/tmp/.poria/repos/ls/ls-api".into(),
                branch: "release".into(),
                scope: "ls".into(),
                name: "ls-api".into(),
            }),
            ..Default::default()
        };

        let mut vars = HashMap::new();
        insert_backend_coding_aid_vars(&mut vars, &config, &ctx);
        assert_eq!(
            vars.get("backend_trd_url").map(String::as_str),
            Some("https://joyspace.jd.com/pages/be")
        );
        assert_eq!(
            vars.get("backend_trd_content").map(String::as_str),
            Some("# backend trd")
        );
        assert_eq!(
            vars.get("backend_repo_path").map(String::as_str),
            Some("/tmp/.poria/repos/ls/ls-api")
        );
        assert_eq!(
            vars.get("backend_branch").map(String::as_str),
            Some("release")
        );
        assert_ne!(
            vars.get("backend_trd_content").map(String::as_str),
            Some("frontend trd must not leak")
        );

        let bundled = repo_bundled_skills_dir();
        for name in ["gen-trd", "gen-code"] {
            let body = fs::read_to_string(bundled.join(name).join("SKILL.md")).unwrap();
            assert!(body.contains("禁止修改后端仓"));
            assert!(body.contains("TRD.md"));
            assert!(!body.contains("{{backend_trd_url}}"));
        }
        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn insert_backend_coding_aid_vars_empty_when_absent() {
        let base = temp_dir("empty");
        let ctx = FeatureContext::create(&base, "pipe-aid-empty", 1).unwrap();
        let mut vars = HashMap::new();
        insert_backend_coding_aid_vars(&mut vars, &PipelineConfig::default(), &ctx);
        assert_eq!(vars.get("backend_trd_url").map(String::as_str), Some(""));
        assert_eq!(
            vars.get("backend_trd_content").map(String::as_str),
            Some("")
        );
        assert_eq!(vars.get("backend_repo_path").map(String::as_str), Some(""));
        assert_eq!(vars.get("backend_branch").map(String::as_str), Some(""));
        fs::remove_dir_all(&base).ok();
    }
}
