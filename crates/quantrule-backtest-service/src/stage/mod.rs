//! The diagnostic stage worker.
//!
//! The worker consumes the generated `stages.worker.<generation>` queue, claims
//! one stage row through evidence,
//! invokes a [`StageExecutor`] seam, publishes the content-addressed artifact to
//! the stage object namespace, and records the result through evidence. It never
//! runs a full backtest, writes a run row, or produces a canonical digest.
//!
//! The model, strategy, and portfolio implementations live behind the executor
//! trait (#852–#854). This module owns the control-plane mechanics and the
//! executor dispatch; a stage never writes a run row or a canonical digest.

pub mod executor;
pub mod model;
pub mod portfolio;
pub mod strategy;
pub mod worker;

pub use executor::{StageDispatch, StageExecutor, StageExecutorError, StageFile, StageOutput};
pub use model::ModelStageExecutor;
pub use portfolio::PortfolioStageExecutor;
pub use strategy::StrategyStageExecutor;
pub use worker::{StageProcessOutcome, StageWorkerError, StageWorkerServices, process_stage};
