use poria_core::types::StageEnum;

/// Maps each [`StageEnum`] to its skill identifier string.
pub const STAGE_SKILL_MAP: &[(StageEnum, &str)] = &[
    (StageEnum::Init, "skill:init"),
    (StageEnum::ReviewPrd, "skill:review-prd"),
    (StageEnum::Design, "skill:gen-trd"),
    (StageEnum::TestPlan, "skill:test-plan"),
    (StageEnum::Dev, "skill:gen-code"),
    (StageEnum::Lint, "skill:lint"),
    (StageEnum::Cr, "skill:code-review"),
    (StageEnum::TestCases, "skill:test-cases"),
    (StageEnum::RunAutotest, "skill:run-autotest"),
    (StageEnum::HandoffQa, "skill:handoff-qa"),
    (StageEnum::Deploy, "skill:deploy"),
    (StageEnum::Archive, "skill:archive"),
];

/// Look up the skill identifier for a given [`StageEnum`].
pub fn stage_to_skill_id(stage: StageEnum) -> &'static str {
    match stage {
        StageEnum::Init => "skill:init",
        StageEnum::ReviewPrd => "skill:review-prd",
        StageEnum::Design => "skill:gen-trd",
        StageEnum::TestPlan => "skill:test-plan",
        StageEnum::Dev => "skill:gen-code",
        StageEnum::Lint => "skill:lint",
        StageEnum::Cr => "skill:code-review",
        StageEnum::TestCases => "skill:test-cases",
        StageEnum::RunAutotest => "skill:run-autotest",
        StageEnum::HandoffQa => "skill:handoff-qa",
        StageEnum::Deploy => "skill:deploy",
        StageEnum::Archive => "skill:archive",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use poria_core::types::STAGE_ORDER;

    #[test]
    fn map_covers_every_stage_in_order() {
        let stages: Vec<_> = STAGE_SKILL_MAP.iter().map(|(s, _)| *s).collect();
        assert_eq!(
            stages,
            vec![
                StageEnum::Init,
                StageEnum::ReviewPrd,
                StageEnum::Design,
                StageEnum::TestPlan,
                StageEnum::Dev,
                StageEnum::Lint,
                StageEnum::Cr,
                StageEnum::TestCases,
                StageEnum::RunAutotest,
                StageEnum::HandoffQa,
                StageEnum::Deploy,
                StageEnum::Archive,
            ]
        );
        assert_eq!(stages.len(), STAGE_ORDER.len());
    }

    #[test]
    fn stage_to_skill_id_matches_map() {
        for (stage, skill_id) in STAGE_SKILL_MAP {
            assert_eq!(stage_to_skill_id(*stage), *skill_id);
        }
    }
}
