//! The analytics stage: descriptive metrics and evaluation tables.
//!
//! The stage consumes a backtest bundle and publishes descriptive evaluation
//! artifacts. It writes no verdict, no acceptance, and no strategy decision.
//! No metric it publishes may enter a strategy or model input.

mod benchmarks;
mod evidence;
mod evidence_parse;
mod metrics;
mod render;
mod run;

pub use run::{AnalyticsOutcome, AnalyticsRequest, AnalyticsRunError, run};
