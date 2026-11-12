//! The strategy stage reads one model bundle and publishes decisions.
//!
//! The stage boundary is explicit: it consumes the published model bundle
//! (`readings.csv`, `events.csv`, and `provenance.json`) and a strategy
//! configuration. It does not read a candle, re-run the model, or touch
//! execution, portfolio, or account state.

pub mod artifacts;
mod bundle;
pub mod context;
mod run;

pub use artifacts::{DecisionCounts, StrategyArtifactMeta};
pub use context::{StrategyContextError, validate_context};
pub use run::{
    StrategyExplainRequest, StrategyOutcome, StrategyRunError, StrategyRunRequest, decision_record,
    explain, run,
};
