//! Research evaluation helpers shared by the statistical harnesses.
//!
//! The module is deterministic and offline. It reproduces the seeded CPython
//! random sequence and the normal quantiles the recorded evaluators used.

pub mod adapters;
pub mod cell;
pub mod eval;
pub mod merge;
pub mod program;
pub mod random;
pub mod registry;
pub mod stats;
