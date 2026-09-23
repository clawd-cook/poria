use poria_core::types::SkillInput;

pub const SKILL_REVIEW_PRD: &str = "review-prd";
pub const SKILL_GEN_TRD: &str = "gen-trd";
pub const SKILL_GEN_CODE: &str = "gen-code";
pub const SKILL_CODE_REVIEW: &str = "code-review";

pub const BUNDLED_SKILL_DIRS: &[&str] = &[
    SKILL_REVIEW_PRD,
    SKILL_GEN_TRD,
    SKILL_GEN_CODE,
    SKILL_CODE_REVIEW,
];

/// Directory under bundled `skills/` for a Claude skill id (`review-prd` or `skill:review-prd`).
pub fn bundled_skill_dir_name(skill_id: &str) -> Option<&'static str> {
    match skill_id.trim() {
        "skill:review-prd" | "review-prd" => Some(SKILL_REVIEW_PRD),
        "skill:gen-trd" | "gen-trd" => Some(SKILL_GEN_TRD),
        "skill:gen-code" | "gen-code" => Some(SKILL_GEN_CODE),
        "skill:code-review" | "code-review" => Some(SKILL_CODE_REVIEW),
        _ => None,
    }
}

/// Short `claude -p` prompt that names a bundled skill. Keep this short; the
/// procedure lives in `SKILL.md`.
pub fn build_claude_skill_prompt(
    skill_dir: &str,
    demand_code: &str,
    workspace_path: &str,
    frontend_dir: &str,
    backend_dir: &str,
    backend_trd_url: &str,
    base_branch: &str,
) -> String {
    build_claude_skill_prompt_with_note(
        skill_dir,
        demand_code,
        workspace_path,
        frontend_dir,
        backend_dir,
        backend_trd_url,
        base_branch,
        None,
    )
}

/// Same as [`build_claude_skill_prompt`], plus optional stage-boundary annotate note (R8).
pub fn build_claude_skill_prompt_with_note(
    skill_dir: &str,
    demand_code: &str,
    workspace_path: &str,
    frontend_dir: &str,
    backend_dir: &str,
    backend_trd_url: &str,
    base_branch: &str,
    advance_note: Option<&str>,
) -> String {
    let mut prompt = format!(
        "使用 {skill_dir} skill。需求号 {demand_code}。工作区根 `{workspace_path}`。前端目录 `{frontend_dir}`。后端只读目录 `{backend_dir}`。按 SKILL.md 非交互完成，不要提问。"
    );
    if skill_dir == SKILL_CODE_REVIEW {
        prompt.push_str(" 你是独立评审者，不要沿用 gen-code 会话，不要改业务代码。");
    }
    let url = backend_trd_url.trim();
    if !url.is_empty() {
        prompt.push_str(&format!(" 后端 TRD `{url}`。"));
    }
    let base = base_branch.trim();
    if !base.is_empty() {
        prompt.push_str(&format!(" 前端基准分支 `{base}`。"));
    }
    if let Some(note) = advance_note.map(str::trim).filter(|s| !s.is_empty()) {
        prompt.push_str(&format!(" 补充上下文：{note}。"));
    }
    prompt
}

/// Read pipeline `advance_note` for injection into the next stage prompt.
pub fn advance_note_from_input(input: &SkillInput) -> Option<&str> {
    input
        .pipeline
        .config
        .advance_note
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

pub fn extra_nonempty<'a>(input: &'a SkillInput, key: &str) -> Option<&'a str> {
    input
        .extra
        .get(key)
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub fn resolve_workspace_cwd(
    input: &SkillInput,
    ctx_workdir: &str,
) -> Result<String, &'static str> {
    extra_nonempty(input, "workspace_path")
        .map(str::to_string)
        .or_else(|| {
            let workdir = ctx_workdir.trim();
            if workdir.is_empty() {
                None
            } else {
                Some(workdir.to_string())
            }
        })
        .ok_or("missing workspace_path in skill input")
}

pub fn resolve_feature_dir(input: &SkillInput) -> Result<String, &'static str> {
    extra_nonempty(input, "feature_dir")
        .map(str::to_string)
        .or_else(|| input.pipeline.config.project_dir.clone())
        .ok_or("missing feature_dir in skill input")
}

pub fn backend_dir(input: &SkillInput) -> String {
    input
        .pipeline
        .config
        .backend_context
        .as_ref()
        .map(|ctx| ctx.local_path.clone())
        .unwrap_or_default()
}

pub fn backend_trd_url(input: &SkillInput) -> &str {
    input
        .pipeline
        .config
        .backend_trd_url
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("")
}

pub fn frontend_base_branch(input: &SkillInput) -> String {
    extra_nonempty(input, "base_branch")
        .map(str::to_string)
        .or_else(|| {
            input
                .pipeline
                .config
                .repos
                .first()
                .map(|repo| repo.base_branch.clone())
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn short_prompt_names_skill_and_paths() {
        let prompt = build_claude_skill_prompt(
            SKILL_REVIEW_PRD,
            "R1",
            "/tmp/.poria/workspaces/p1",
            "/tmp/.poria/workspaces/p1/fe",
            "/tmp/.poria/workspaces/p1/be",
            "https://joyspace.jd.com/pages/be",
            "master",
        );
        assert!(prompt.contains("review-prd"));
        assert!(prompt.contains("R1"));
        assert!(prompt.contains("/tmp/.poria/workspaces/p1"));
        assert!(prompt.contains("/tmp/.poria/workspaces/p1/fe"));
        assert!(prompt.contains("/tmp/.poria/workspaces/p1/be"));
        assert!(prompt.contains("https://joyspace.jd.com/pages/be"));
        assert!(prompt.contains("master"));
        assert!(prompt.len() < 500);
        assert!(!prompt.contains("system-prompt"));
        assert!(!prompt.contains("P0（必填"));
    }

    #[test]
    fn short_prompt_omits_empty_url_and_base_branch() {
        let prompt = build_claude_skill_prompt(
            SKILL_GEN_TRD,
            "R1",
            "/tmp/.poria/workspaces/p1",
            "/tmp/.poria/workspaces/p1/fe",
            "/tmp/.poria/workspaces/p1/be",
            "  ",
            "",
        );
        assert!(prompt.contains("gen-trd"));
        assert!(!prompt.contains("后端 TRD"));
        assert!(!prompt.contains("前端基准分支"));
    }

    #[test]
    fn code_review_prompt_marks_independent_reviewer() {
        let prompt = build_claude_skill_prompt(
            SKILL_CODE_REVIEW,
            "R1",
            "/tmp/.poria/workspaces/p1",
            "/tmp/.poria/workspaces/p1/fe",
            "/tmp/.poria/workspaces/p1/be",
            "",
            "master",
        );
        assert!(prompt.contains("独立评审者"));
        assert!(prompt.contains("gen-code"));
        let gen_code = build_claude_skill_prompt(
            SKILL_GEN_CODE,
            "R1",
            "/tmp/.poria/workspaces/p1",
            "/tmp/.poria/workspaces/p1/fe",
            "/tmp/.poria/workspaces/p1/be",
            "",
            "master",
        );
        assert!(!gen_code.contains("独立评审者"));
    }

    #[test]
    fn bundled_skill_dir_name_maps_claude_stages_only() {
        assert_eq!(
            bundled_skill_dir_name("skill:review-prd"),
            Some(SKILL_REVIEW_PRD)
        );
        assert_eq!(bundled_skill_dir_name("review-prd"), Some(SKILL_REVIEW_PRD));
        assert_eq!(bundled_skill_dir_name("skill:gen-trd"), Some(SKILL_GEN_TRD));
        assert_eq!(bundled_skill_dir_name("gen-code"), Some(SKILL_GEN_CODE));
        assert_eq!(
            bundled_skill_dir_name("skill:gen-code"),
            Some(SKILL_GEN_CODE)
        );
        assert_eq!(
            bundled_skill_dir_name("skill:code-review"),
            Some(SKILL_CODE_REVIEW)
        );
        assert_eq!(bundled_skill_dir_name("skill:init"), None);
        assert_eq!(bundled_skill_dir_name("skill:deploy"), None);
        assert_eq!(bundled_skill_dir_name("init"), None);
        assert_eq!(bundled_skill_dir_name("deploy"), None);
        assert_eq!(bundled_skill_dir_name("unknown"), None);
        assert_eq!(BUNDLED_SKILL_DIRS.len(), 4);
        assert!(!BUNDLED_SKILL_DIRS.contains(&"init"));
        assert!(!BUNDLED_SKILL_DIRS.contains(&"deploy"));
    }

    #[test]
    fn bundled_skill_markdown_is_non_interactive() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../skills");
        for name in [
            SKILL_REVIEW_PRD,
            SKILL_GEN_TRD,
            SKILL_GEN_CODE,
            SKILL_CODE_REVIEW,
        ] {
            let body = std::fs::read_to_string(root.join(name).join("SKILL.md")).unwrap();
            assert!(body.contains("非交互"));
            assert!(!body.contains("请 review TASK.md，确认后"));
            assert!(!body.contains("停下，说"));
            if name == SKILL_CODE_REVIEW {
                assert!(body.contains("独立评审者"));
            }
        }
    }

    #[test]
    fn prompt_includes_advance_note_when_present() {
        let prompt = build_claude_skill_prompt_with_note(
            SKILL_GEN_CODE,
            "R1",
            "/tmp/ws",
            "/tmp/ws/fe",
            "/tmp/ws/be",
            "",
            "master",
            Some("优先改 Button 组件"),
        );
        assert!(prompt.contains("补充上下文：优先改 Button 组件"));
        let plain = build_claude_skill_prompt(
            SKILL_GEN_CODE,
            "R1",
            "/tmp/ws",
            "/tmp/ws/fe",
            "/tmp/ws/be",
            "",
            "master",
        );
        assert!(!plain.contains("补充上下文"));
    }
}
