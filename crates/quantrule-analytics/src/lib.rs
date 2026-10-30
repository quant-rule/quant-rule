//! The research and analytics engine.
//!
//! This crate turns the model's published numbers into the research dataset
//! defined by `docs/analytics/`. It is pure: it performs no I/O, reads no
//! clock, and uses no randomness. The shell owns all SQLite and CSV access.
//!
//! The crate currently provides the per-axis resolution in [`resolution`], the
//! as-of alignment in [`alignment`], the three research layers in [`layers`],
//! and the chart selector in [`chart`].

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod alignment;
pub mod chart;
pub mod layers;
pub mod research;
pub mod resolution;
pub mod summary;

use quantrule_model::aggregate::CrossTimeframeAggregates;
use quantrule_model::run::ModelRun;

/// Computes the descriptive cross-timeframe view over a model run.
///
/// The model run publishes per-timeframe snapshots only. Reporting and
/// analytics call this function when they need the descriptive aggregate; the
/// strategy decision path must not use it.
#[must_use]
pub fn cross_timeframe(run: &ModelRun) -> CrossTimeframeAggregates {
    CrossTimeframeAggregates::compute(&run.snapshots())
}
