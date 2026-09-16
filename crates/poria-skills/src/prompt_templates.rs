use std::collections::HashMap;

pub const PRD_REVIEW_PROMPT: &str = include_str!("prompts/prd_review.md");
pub const TRD_GEN_PROMPT: &str = include_str!("prompts/trd_gen.md");
pub const CODE_IMPL_PROMPT: &str = include_str!("prompts/code_impl.md");
pub const CR_WEB_PROMPT: &str = include_str!("prompts/cr_web.md");

pub fn render_prompt(template: &str, vars: &HashMap<String, String>) -> String {
    let mut result = template.to_string();
    for (key, value) in vars {
        result = result.replace(&format!("{{{{{}}}}}", key), value);
    }
    result
}

pub fn skill_prompt_for_stage(stage: &str) -> Option<&'static str> {
    match stage {
        "review-prd" | "ReviewPrd" => Some(PRD_REVIEW_PROMPT),
        "gen-trd" | "Design" => Some(TRD_GEN_PROMPT),
        "gen-code" | "Dev" => Some(CODE_IMPL_PROMPT),
        "code-review" | "Cr" => Some(CR_WEB_PROMPT),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_prompt_basic() {
        let template = "Review the PRD at {{feature_dir}}/PRD.md for project {{project_root}}";
        let mut vars = HashMap::new();
        vars.insert("feature_dir".into(), "/tmp/feat-001".into());
        vars.insert("project_root".into(), "/home/user/project".into());

        let rendered = render_prompt(template, &vars);
        assert_eq!(
            rendered,
            "Review the PRD at /tmp/feat-001/PRD.md for project /home/user/project"
        );
    }

    #[test]
    fn test_render_prompt_no_vars() {
        let template = "Static prompt with no variables";
        let vars = HashMap::new();
        assert_eq!(render_prompt(template, &vars), template);
    }

    #[test]
    fn test_render_prompt_unmatched_vars() {
        let template = "Value: {{known}} and {{unknown}}";
        let mut vars = HashMap::new();
        vars.insert("known".into(), "resolved".into());

        let rendered = render_prompt(template, &vars);
        assert_eq!(rendered, "Value: resolved and {{unknown}}");
    }

    #[test]
    fn test_skill_prompt_for_stage() {
        assert!(skill_prompt_for_stage("review-prd").is_some());
        assert!(skill_prompt_for_stage("ReviewPrd").is_some());
        assert!(skill_prompt_for_stage("gen-trd").is_some());
        assert!(skill_prompt_for_stage("gen-code").is_some());
        assert!(skill_prompt_for_stage("code-review").is_some());
        assert!(skill_prompt_for_stage("unknown").is_none());
    }
}
