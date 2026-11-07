//! The strategy stage reads one model bundle and publishes decisions.
//!
//! The stage boundary is explicit: it consumes the published model bundle
//! (`readings.csv`, `events.csv`, and `provenance.json`) and a strategy
//! configuration. It does not read a candle, re-run the model, or touch
//! execution, portfolio, or account state.

pub(crate) mod artifacts;
mod bundle;
mod context;
mod run;

pub(crate) use artifacts::{DecisionCounts, StrategyArtifactMeta};
pub(crate) use context::{StrategyContextError, validate_context};
pub(crate) use run::{StrategyRunError, decision_record, explain, run};
