//! Research evaluation helpers shared by the statistical harnesses.
//!
//! The module is deterministic and offline. It reproduces the seeded CPython
//! random sequence and the normal quantiles the recorded evaluators used.

pub mod random;
pub mod stats;
