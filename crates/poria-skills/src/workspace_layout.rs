use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use poria_core::feature_context::{
    ARTIFACT_BACKEND_TRD, ARTIFACT_CR, ARTIFACT_PRD, ARTIFACT_PRD_REVIEW, ARTIFACT_TASK,
    ARTIFACT_TRD,
};

use crate::claude_prompt::BUNDLED_SKILL_DIRS;

pub const ARTIFACT_SYMLINK_NAMES: &[&str] = &[
    ARTIFACT_PRD,
    ARTIFACT_BACKEND_TRD,
    ARTIFACT_PRD_REVIEW,
    ARTIFACT_TRD,
    ARTIFACT_TASK,
    ARTIFACT_CR,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundledSkillDoc {
    pub body: String,
    pub description: String,
    pub name: String,
}

pub fn parse_skill_markdown(raw: &str) -> Result<BundledSkillDoc, String> {
    let raw = raw.trim_start_matches('\u{feff}');
    let Some(after_open) = raw.strip_prefix("---") else {
        return Err("SKILL.md 缺少 YAML frontmatter".into());
    };
    let after_open = after_open
        .strip_prefix('\n')
        .or_else(|| after_open.strip_prefix("\r\n"))
        .ok_or("SKILL.md frontmatter 格式无效")?;
    let close = after_open
        .find("\n---")
        .ok_or("SKILL.md frontmatter 未结束")?;
    let front = &after_open[..close];
    let after_close = &after_open[close + 1..];
    let body = after_close
        .strip_prefix("---")
        .unwrap_or(after_close)
        .trim_start_matches(['\r', '\n'])
        .to_string();
    Ok(BundledSkillDoc {
        body,
        description: frontmatter_field(front, "description").unwrap_or_default(),
        name: frontmatter_field(front, "name").ok_or("SKILL.md 缺少 name")?,
    })
}

fn frontmatter_field(front: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}:");
    for line in front.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix(&prefix) else {
            continue;
        };
        let value = rest.trim().trim_matches('"').trim_matches('\'').trim();
        if !value.is_empty() {
            return Some(value.to_string());
        }
    }
    None
}

pub fn read_bundled_skill_markdown(bundled_root: &Path, dir_name: &str) -> Result<String, String> {
    if !BUNDLED_SKILL_DIRS.contains(&dir_name) {
        return Err(format!("不是随包 Claude skill: {dir_name}"));
    }
    let path = bundled_root.join(dir_name).join("SKILL.md");
    fs::read_to_string(&path).map_err(|e| format!("无法读取 {}: {e}", path.display()))
}

/// Bundled Claude skills only (`review-prd` / `gen-trd` / `gen-code` / `code-review`).
/// The first tuple field is the directory id, not the YAML `name`.
pub fn list_bundled_skill_docs(
    bundled_root: &Path,
) -> Result<Vec<(&'static str, BundledSkillDoc)>, String> {
    BUNDLED_SKILL_DIRS
        .iter()
        .copied()
        .map(|dir| {
            let raw = read_bundled_skill_markdown(bundled_root, dir)?;
            Ok((dir, parse_skill_markdown(&raw)?))
        })
        .collect()
}

pub fn bundled_skills_complete(dir: &Path) -> bool {
    BUNDLED_SKILL_DIRS
        .iter()
        .all(|name| dir.join(name).join("SKILL.md").is_file())
}

/// Create the pipeline workspace: artifact + skill symlinks and `CLAUDE.md`.
/// Worktrees are created afterwards under the same directory.
pub fn prepare_pipeline_workspace(
    workspace_dir: &Path,
    project_dir: &Path,
    bundled_skills_dir: &Path,
) -> Result<(), String> {
    if !bundled_skills_complete(bundled_skills_dir) {
        return Err(format!(
            "随包 skill 不完整: {}",
            bundled_skills_dir.display()
        ));
    }
    fs::create_dir_all(workspace_dir).map_err(|e| format!("无法创建工作区: {e}"))?;
    fs::create_dir_all(project_dir).map_err(|e| format!("无法创建项目文档目录: {e}"))?;

    for name in ARTIFACT_SYMLINK_NAMES {
        let link = workspace_dir.join(name);
        let target = project_dir.join(name);
        force_symlink(&target, &link)?;
    }

    let skills_parent = workspace_dir.join(".claude").join("skills");
    fs::create_dir_all(&skills_parent).map_err(|e| format!("无法创建 .claude/skills: {e}"))?;
    for name in BUNDLED_SKILL_DIRS {
        let link = skills_parent.join(name);
        let target = bundled_skills_dir.join(name);
        force_symlink(&target, &link)?;
    }

    fs::write(workspace_dir.join("CLAUDE.md"), workspace_claude_md())
        .map_err(|e| format!("无法写入 CLAUDE.md: {e}"))?;
    Ok(())
}

pub fn workspace_claude_md() -> &'static str {
    r#"# Poria 流水线工作区

这是 Claude 的 cwd。本目录不是 git 仓根，不要在这里 `git init`。

## 布局

- `PRD.md` / `BACKEND_TRD.md` / `PRD_REVIEW.md` / `TRD.md` / `TASK.md` / `CR.md` 是指向 `~/.poria/projects/<demand_code>/` 的软链。写入这些文件名即落到文档真源，不要把它们复制进前后端 git 仓。
- 前端仓子目录是 feature worktree：业务代码只改这里。
- 后端仓子目录是只读 detached worktree：可以读，禁止改文件、禁止建后端 MR。
- `.claude/skills/` 下的 `review-prd` / `gen-trd` / `gen-code` / `code-review` 是随包 skill 软链，不要修改。

## 执行

桌面 AutoRun 按阶段 `claude -p` 点名 skill。禁止提问、禁止 HITL、禁止把后面阶段在一场长会话里跑完。
"#
}

fn force_symlink(target: &Path, link: &Path) -> Result<(), String> {
    match fs::symlink_metadata(link) {
        Ok(meta) => {
            if meta.file_type().is_symlink() {
                if let Ok(existing) = fs::read_link(link) {
                    if existing == target {
                        return Ok(());
                    }
                }
            }
            if meta.is_dir() && !meta.file_type().is_symlink() {
                fs::remove_dir_all(link)
                    .map_err(|e| format!("无法替换目录 {}: {e}", link.display()))?;
            } else {
                fs::remove_file(link).map_err(|e| format!("无法替换 {}: {e}", link.display()))?;
            }
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => {}
        Err(err) => return Err(format!("无法读取 {}: {err}", link.display())),
    }
    symlink_path(target, link)
}

fn symlink_path(target: &Path, link: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link).map_err(|e| {
            format!(
                "无法创建软链 {} -> {}: {e}",
                link.display(),
                target.display()
            )
        })
    }
    #[cfg(not(unix))]
    {
        let _ = (target, link);
        Err("当前平台不支持工作区软链".into())
    }
}

/// Resolve repo-root `skills/` in tests and `cargo test`.
pub fn repo_bundled_skills_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../skills")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claude_prompt::{
        BUNDLED_SKILL_DIRS, SKILL_CODE_REVIEW, SKILL_GEN_CODE, SKILL_GEN_TRD, SKILL_REVIEW_PRD,
    };

    fn temp_dir(suffix: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "poria-ws-layout-{suffix}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn prepare_workspace_symlinks_docs_and_skills() {
        let base = temp_dir("ok");
        let workspace = base.join("workspaces").join("p1");
        let project = base.join("projects").join("R1");
        fs::create_dir_all(&project).unwrap();
        fs::write(project.join(ARTIFACT_PRD), "# prd").unwrap();
        let bundled = repo_bundled_skills_dir();
        assert!(
            bundled_skills_complete(&bundled),
            "repo skills/ missing: {}",
            bundled.display()
        );

        prepare_pipeline_workspace(&workspace, &project, &bundled).unwrap();

        let prd_link = workspace.join(ARTIFACT_PRD);
        assert!(prd_link
            .symlink_metadata()
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(
            fs::read_link(&prd_link).unwrap(),
            project.join(ARTIFACT_PRD)
        );
        assert_eq!(fs::read_to_string(&prd_link).unwrap(), "# prd");

        let review_link = workspace.join(ARTIFACT_PRD_REVIEW);
        assert!(review_link
            .symlink_metadata()
            .unwrap()
            .file_type()
            .is_symlink());
        assert!(!review_link.exists());
        fs::write(project.join(ARTIFACT_PRD_REVIEW), "review").unwrap();
        assert_eq!(fs::read_to_string(&review_link).unwrap(), "review");

        for name in [
            SKILL_REVIEW_PRD,
            SKILL_GEN_TRD,
            SKILL_GEN_CODE,
            SKILL_CODE_REVIEW,
        ] {
            let link = workspace.join(".claude").join("skills").join(name);
            assert!(
                link.symlink_metadata().unwrap().file_type().is_symlink(),
                "{name} is not a symlink"
            );
            assert_eq!(fs::read_link(&link).unwrap(), bundled.join(name));
            assert!(link.join("SKILL.md").is_file());
        }
        let claude = fs::read_to_string(workspace.join("CLAUDE.md")).unwrap();
        assert!(claude.contains("软链"));
        assert!(claude.contains("不要把它们复制进前后端 git 仓"));
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn prepare_workspace_rejects_incomplete_bundle() {
        let base = temp_dir("bad-bundle");
        let err = prepare_pipeline_workspace(
            &base.join("ws"),
            &base.join("proj"),
            &base.join("empty-skills"),
        )
        .unwrap_err();
        assert!(err.contains("随包 skill 不完整"));
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn read_bundled_skill_markdown_returns_skill_md() {
        let bundled = repo_bundled_skills_dir();
        let body = read_bundled_skill_markdown(&bundled, "review-prd").unwrap();
        assert!(body.contains("name: review-prd"));
        assert!(read_bundled_skill_markdown(&bundled, "init")
            .unwrap_err()
            .contains("不是随包"));
        let parsed = parse_skill_markdown(&body).unwrap();
        assert_eq!(parsed.name, "review-prd");
        assert!(!parsed.description.is_empty());
        assert!(parsed.body.starts_with('#'));
        assert!(!parsed.body.starts_with("---"));
        assert!(!parsed.body.contains("name: review-prd"));
    }

    #[test]
    fn parse_skill_markdown_rejects_missing_frontmatter() {
        let err = parse_skill_markdown("# just a heading\n").unwrap_err();
        assert!(err.contains("frontmatter"));
    }

    #[test]
    fn list_bundled_skill_docs_uses_directory_id_and_excludes_pipeline_stages() {
        let bundled = repo_bundled_skills_dir();
        let listed = list_bundled_skill_docs(&bundled).unwrap();
        let ids: Vec<&str> = listed.iter().map(|(id, _)| *id).collect();
        assert_eq!(ids, BUNDLED_SKILL_DIRS);
        assert!(!ids.iter().any(|id| *id == "init" || *id == "deploy"));
        for (id, doc) in &listed {
            assert!(
                doc.body.starts_with('#'),
                "{id} body should start with markdown, not YAML"
            );
            assert!(!doc.body.starts_with("---"));
            assert!(!doc.description.is_empty());
        }

        let base = temp_dir("list-ids");
        for name in BUNDLED_SKILL_DIRS {
            let dir = base.join(name);
            fs::create_dir_all(&dir).unwrap();
            fs::write(
                dir.join("SKILL.md"),
                format!("---\nname: Display {name}\ndescription: demo\n---\n# Body\n"),
            )
            .unwrap();
        }
        let renamed = list_bundled_skill_docs(&base).unwrap();
        let renamed_ids: Vec<&str> = renamed.iter().map(|(id, _)| *id).collect();
        assert_eq!(renamed_ids, BUNDLED_SKILL_DIRS);
        for (id, doc) in &renamed {
            assert_eq!(doc.name, format!("Display {id}"));
            assert_eq!(doc.body, "# Body\n");
        }
        let _ = fs::remove_dir_all(&base);
    }
}
