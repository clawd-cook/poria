use std::path::Path;

use poria_core::feature_context::{
    FeatureContext, ARTIFACT_BACKEND_TRD, ARTIFACT_CR, ARTIFACT_PRD, ARTIFACT_PRD_REVIEW,
    ARTIFACT_TASK, ARTIFACT_TRD,
};

const PROJECT_DOC_ARTIFACTS: &[&str] = &[
    ARTIFACT_PRD,
    ARTIFACT_BACKEND_TRD,
    ARTIFACT_PRD_REVIEW,
    ARTIFACT_TRD,
    ARTIFACT_TASK,
    ARTIFACT_CR,
];

/// If the agent wrote a project doc into the git worktree, copy it to the
/// demand project dir and delete the worktree copy so Deploy cannot commit it.
pub fn adopt_and_remove_from_worktree(feature_ctx: &FeatureContext, worktree: &str, name: &str) {
    let candidate = Path::new(worktree).join(name);
    if candidate
        .symlink_metadata()
        .map(|meta| meta.file_type().is_symlink())
        .unwrap_or(false)
    {
        return;
    }
    if !candidate.is_file() {
        return;
    }
    if !feature_ctx.has_artifact(name) {
        if let Ok(content) = std::fs::read_to_string(&candidate) {
            let _ = feature_ctx.write_artifact(name, &content);
        }
    }
    let _ = std::fs::remove_file(&candidate);
}

/// Drop leaked project markdown from a frontend worktree before `git add`.
pub fn strip_project_docs_from_worktree(worktree: &Path) {
    for name in PROJECT_DOC_ARTIFACTS {
        let path = worktree.join(name);
        if path
            .symlink_metadata()
            .map(|meta| meta.file_type().is_symlink())
            .unwrap_or(false)
        {
            continue;
        }
        if path.is_file() {
            let _ = std::fs::remove_file(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use poria_core::feature_context::FeatureContext;
    use std::fs;

    fn temp_dir(suffix: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("poria-artifacts-{suffix}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn adopt_and_remove_skips_symlink_into_projects() {
        let base = temp_dir("adopt-symlink");
        let project = base.join("project");
        let workspace = base.join("workspace");
        fs::create_dir_all(&workspace).unwrap();
        let ctx = FeatureContext::create_at(project.clone(), "pipe", 1).unwrap();
        ctx.write_artifact(ARTIFACT_PRD_REVIEW, "from-projects")
            .unwrap();
        std::os::unix::fs::symlink(
            ctx.artifact_path(ARTIFACT_PRD_REVIEW),
            workspace.join(ARTIFACT_PRD_REVIEW),
        )
        .unwrap();
        adopt_and_remove_from_worktree(&ctx, &workspace.to_string_lossy(), ARTIFACT_PRD_REVIEW);
        assert!(workspace
            .join(ARTIFACT_PRD_REVIEW)
            .symlink_metadata()
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(
            ctx.read_artifact(ARTIFACT_PRD_REVIEW).unwrap().unwrap(),
            "from-projects"
        );
        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn adopt_and_remove_moves_leaked_review_out_of_worktree() {
        let base = temp_dir("adopt");
        let project = base.join("project");
        let worktree = base.join("worktree");
        fs::create_dir_all(&worktree).unwrap();
        fs::write(worktree.join(ARTIFACT_PRD_REVIEW), "review").unwrap();
        let ctx = FeatureContext::create_at(project, "pipe", 1).unwrap();
        adopt_and_remove_from_worktree(&ctx, &worktree.to_string_lossy(), ARTIFACT_PRD_REVIEW);
        assert!(ctx.has_artifact(ARTIFACT_PRD_REVIEW));
        assert!(!worktree.join(ARTIFACT_PRD_REVIEW).exists());
        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn strip_project_docs_leaves_source_files() {
        let base = temp_dir("strip");
        fs::write(base.join(ARTIFACT_TRD), "trd").unwrap();
        fs::write(base.join("src.rs"), "code").unwrap();
        strip_project_docs_from_worktree(&base);
        assert!(!base.join(ARTIFACT_TRD).exists());
        assert!(base.join("src.rs").exists());
        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn strip_project_docs_skips_symlink_into_projects() {
        let base = temp_dir("strip-symlink");
        let project = base.join("project");
        let worktree = base.join("worktree");
        fs::create_dir_all(&worktree).unwrap();
        let ctx = FeatureContext::create_at(project, "pipe", 1).unwrap();
        ctx.write_artifact(ARTIFACT_TRD, "keep").unwrap();
        std::os::unix::fs::symlink(ctx.artifact_path(ARTIFACT_TRD), worktree.join(ARTIFACT_TRD))
            .unwrap();
        strip_project_docs_from_worktree(&worktree);
        assert!(worktree
            .join(ARTIFACT_TRD)
            .symlink_metadata()
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(ctx.read_artifact(ARTIFACT_TRD).unwrap().unwrap(), "keep");
        fs::remove_dir_all(&base).ok();
    }
}
