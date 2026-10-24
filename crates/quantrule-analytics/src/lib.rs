//! The research and analytics engine.
//!
//! This crate turns the model's published numbers into the research dataset
//! defined by `docs/analytics/`. It is pure: it performs no I/O, reads no
//! clock, and uses no randomness. The shell owns all SQLite and CSV access.
//!
//! The crate currently provides the per-axis resolution in [`resolution`]. The
//! as-of alignment and the three research layers are added by their own work
//! items against the same contract.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod resolution;
