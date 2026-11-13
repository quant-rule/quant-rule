//! Hypothesis evaluation.
//!
//! This crate answers the kata's core question: is an apparent advantage larger
//! than what randomness would produce? It takes the trade returns of one rule,
//! applies the cost model that produced them, and reports the measures a
//! researcher needs to accept, revise, or stop.
//!
//! It is pure: no I/O, no clock. The resampling seed is a parameter, so the same
//! trades and the same seed give the same numbers on every machine.

#![forbid(unsafe_code)]

pub mod costs;
pub mod rng;
pub mod stats;

pub use costs::{CostError, CostModel};
pub use rng::Pcg32;
pub use stats::{Statistics, StatsError, evaluate};
