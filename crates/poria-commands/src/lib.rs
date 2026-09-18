pub mod exception_classifier;
pub mod executor;
pub mod handle_error;
pub mod rollback;
pub mod traits;
pub mod worker;

// Re-export primary public items for ergonomic use.
pub use exception_classifier::{classify, is_auth_expired};
pub use executor::PipelineExecutor;
pub use handle_error::{
    handle_stage_error, stage_error_outcome, ErrorAction, HandleErrorResult, StageErrorOutcome,
};
pub use rollback::{PipelineRollback, RollbackDeps};
pub use traits::*;
pub use worker::{PipelineWorker, WorkerDeps};
