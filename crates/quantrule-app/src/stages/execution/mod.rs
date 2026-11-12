//! The execution stage: turn strategy decisions into orders, fills, and
//! rejections.
//!
//! The stage consumes a strategy decisions bundle and the run's data,
//! execution, and sizing configuration. It does not decide direction or
//! strategy, and it does not account for cash, equity, or P&L.

mod decisions;
mod run;

pub use run::{
    ExecutionArtifactMeta, ExecutionOutcome, ExecutionRequest, ExecutionRunError, run, write_bundle,
};
