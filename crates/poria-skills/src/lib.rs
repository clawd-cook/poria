mod artifacts;
mod backend_aid;
mod claude_prompt;
mod cr_findings;
mod dev_verify;
mod error;
mod fixture;
mod human_loop;
mod post_cr_notes;
pub mod prompt_templates;
mod quality_gates;
mod stage_skill_map;
mod thin_docs;
mod workspace_layout;

mod code_review;
mod deploy;
mod gen_code;
mod gen_trd;
mod init;
mod review_prd;
mod workspace;

// --- Public re-exports ---

pub use crate::backend_aid::insert_backend_coding_aid_vars;
pub use crate::dev_verify::run_frontend_verify;
pub use crate::post_cr_notes::post_cr_blocking_notes;
pub use claude_prompt::{
    advance_note_from_input, build_claude_skill_prompt, build_claude_skill_prompt_with_note,
    bundled_skill_dir_name, BUNDLED_SKILL_DIRS, SKILL_CODE_REVIEW, SKILL_GEN_CODE, SKILL_GEN_TRD,
    SKILL_REVIEW_PRD,
};
pub use error::SkillError;
pub use fixture::is_fixture_mode;
pub use quality_gates::{
    collect_coverage_report, collect_security_scan, coverage_block_message, security_block_message,
};
pub use workspace_layout::{
    bundled_skills_complete, list_bundled_skill_docs, parse_skill_markdown,
    prepare_pipeline_workspace, read_bundled_skill_markdown, repo_bundled_skills_dir,
    workspace_claude_md, BundledSkillDoc, ARTIFACT_SYMLINK_NAMES,
};

pub use human_loop::{parse_human_reply, HumanAction, HumanLoop, HumanLoopCoordinator, HumanReply};

pub use stage_skill_map::{stage_to_skill_id, STAGE_SKILL_MAP};

pub use code_review::CodeReviewSkill;
pub use deploy::DeploySkill;
pub use gen_code::GenCodeSkill;
pub use gen_trd::GenTrdSkill;
pub use init::InitSkill;
pub use review_prd::ReviewPrdSkill;
pub use thin_docs::{
    ArchiveSkill, HandoffQaSkill, LintSkill, RunAutotestSkill, TestCasesSkill, TestPlanSkill,
};
pub use workspace::WorkspaceSkill;
