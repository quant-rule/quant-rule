//! The qualification stage: evaluate evidence against human-owned criteria.
//!
//! The stage consumes a backtest bundle and `qualification.json`. It publishes a
//! verdict with one record per criterion and one provenance. It never writes the
//! human's acceptance decision, never declares an edge, and never feeds a
//! measurement back into a strategy or model.

mod evaluate;
mod evidence;
mod render;
mod run;

pub(crate) use run::{QualificationRunError, run};
