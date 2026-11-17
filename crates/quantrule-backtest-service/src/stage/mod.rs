//! The diagnostic stage worker.
//!
//! The worker consumes `stages.worker`, claims one stage row through evidence,
//! invokes a [`StageExecutor`] seam, publishes the content-addressed artifact to
//! the stage object namespace, and records the result through evidence. It never
//! runs a full backtest, writes a run row, or produces a canonical digest.
//!
//! The engine implementations live behind the executor trait (#852–#854). This
//! module owns the control-plane mechanics and the model executor; it never
//! runs a strategy or portfolio stage.

pub mod executor;
pub mod model;
pub mod worker;

pub use executor::{StageDispatch, StageExecutor, StageExecutorError, StageFile, StageOutput};
pub use model::ModelStageExecutor;
pub use worker::{StageProcessOutcome, StageWorkerError, StageWorkerServices, process_stage};
