mod error;
mod fixture;
mod human_loop;
pub mod prompt_templates;
mod stage_skill_map;

mod code_review;
mod deploy;
mod gen_code;
mod gen_trd;
mod init;
mod review_prd;
mod workspace;

// --- Public re-exports ---

pub use error::SkillError;
pub use fixture::is_fixture_mode;

pub use human_loop::{
    parse_human_reply, HumanAction, HumanLoop, HumanLoopCoordinator, HumanReply,
};

pub use stage_skill_map::{stage_to_skill_id, STAGE_SKILL_MAP};

pub use code_review::CodeReviewSkill;
pub use deploy::DeploySkill;
pub use gen_code::GenCodeSkill;
pub use gen_trd::GenTrdSkill;
pub use init::InitSkill;
pub use review_prd::ReviewPrdSkill;
pub use workspace::WorkspaceSkill;
