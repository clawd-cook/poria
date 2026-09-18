use poria_core::types::StageEnum;

/// Static mapping from each pipeline stage to its corresponding skill identifier.
pub const STAGE_SKILL_MAP: &[(StageEnum, &str)] = &[
    (StageEnum::Init, "skill:init"),
    (StageEnum::ReviewPrd, "skill:review-prd"),
    (StageEnum::Design, "skill:gen-trd"),
    (StageEnum::Dev, "skill:gen-code"),
    (StageEnum::Cr, "skill:code-review"),
    (StageEnum::Deploy, "skill:deploy"),
];

/// Look up the skill identifier for a given [`StageEnum`].
pub fn stage_to_skill_id(stage: StageEnum) -> &'static str {
    match stage {
        StageEnum::Init => "skill:init",
        StageEnum::ReviewPrd => "skill:review-prd",
        StageEnum::Design => "skill:gen-trd",
        StageEnum::Dev => "skill:gen-code",
        StageEnum::Cr => "skill:code-review",
        StageEnum::Deploy => "skill:deploy",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_covers_all_stages() {
        let stages = [
            StageEnum::Init,
            StageEnum::ReviewPrd,
            StageEnum::Design,
            StageEnum::Dev,
            StageEnum::Cr,
            StageEnum::Deploy,
        ];
        for stage in stages {
            let id = stage_to_skill_id(stage);
            assert!(id.starts_with("skill:"), "stage {stage:?} has bad id: {id}");
        }
    }

    #[test]
    fn test_map_array_matches_function() {
        for (stage, expected_id) in STAGE_SKILL_MAP {
            assert_eq!(stage_to_skill_id(*stage), *expected_id);
        }
    }

    #[test]
    fn test_specific_mappings() {
        assert_eq!(stage_to_skill_id(StageEnum::Init), "skill:init");
        assert_eq!(stage_to_skill_id(StageEnum::ReviewPrd), "skill:review-prd");
        assert_eq!(stage_to_skill_id(StageEnum::Design), "skill:gen-trd");
        assert_eq!(stage_to_skill_id(StageEnum::Dev), "skill:gen-code");
        assert_eq!(stage_to_skill_id(StageEnum::Cr), "skill:code-review");
        assert_eq!(stage_to_skill_id(StageEnum::Deploy), "skill:deploy");
    }
}
