pub mod cr_regress;
pub mod exception_classifier;
pub mod executor;
pub mod handle_error;
pub mod rollback;
pub mod traits;
pub mod worker;

// Re-export primary public items for ergonomic use.
pub use cr_regress::try_regress_cr_to_dev;
pub use exception_classifier::{
    classify, is_auth_expired, is_out_of_scope, is_quality_gate_block, is_requirement_ambiguous,
    is_security_violation, is_trd_unconfirmed,
};
pub use executor::PipelineExecutor;
pub use handle_error::{
    handle_stage_error, parse_retry_delay, retry_delay_for_message, stage_error_outcome,
    ErrorAction, HandleErrorResult, StageErrorOutcome,
};
pub use rollback::{PipelineRollback, RollbackDeps};
pub use traits::*;
pub use worker::{PipelineWorker, WorkerDeps};
