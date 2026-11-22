//! The strategy stage reads one model bundle and publishes decisions.
//!
//! The stage boundary is explicit: it consumes the published model bundle
//! (`readings.csv`, `events.csv`, and `provenance.json`) and a strategy
//! configuration. It does not read a candle, re-run the model, or touch
//! execution, portfolio, or account state. The same model-bundle reader is the
//! signal artifact's input; the signal computation adds its own reference
//! position and artifact writer in `quantrule_app::signal`.

pub mod artifacts;
mod bundle;
pub mod context;
mod evidence;
mod run;

pub use artifacts::{DecisionCounts, StrategyArtifactMeta};
pub use bundle::{BarRow, ModelBundle, StrategyBundleError};
pub use context::{StrategyContextError, validate_context};
pub(crate) use evidence::decision_evidence;
pub use quantrule_backtest::intent::StrategyPosition;
pub use quantrule_backtest::side::Side;
pub use run::{
    StrategyExplainRequest, StrategyOutcome, StrategyRunError, StrategyRunRequest, decision_record,
    explain, run,
};
