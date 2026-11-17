//! The diagnostic stage worker.
//!
//! The worker consumes `stages.worker`, claims one stage row through evidence,
//! invokes a [`StageExecutor`] seam, publishes the content-addressed artifact to
//! the stage object namespace, and records the result through evidence. It never
//! runs a full backtest, writes a run row, or produces a canonical digest.
//!
//! The engine implementations live behind the executor trait (#852–#854). This
//! module owns only the control-plane mechanics: cancellation boundaries,
//! idempotent claims, retry policy, and the no-partial-publish rule.

pub mod executor;
pub mod worker;

pub use executor::{StageDispatch, StageExecutor, StageExecutorError, StageFile, StageOutput};
pub use worker::{StageProcessOutcome, StageWorkerError, StageWorkerServices, process_stage};
