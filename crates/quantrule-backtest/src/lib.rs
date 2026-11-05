//! The backtest engine.
//!
//! The crate is the only place that decides a trade. It consumes the published
//! model values and simulates execution and portfolio accounting. The
//! [`strategy`] module holds the type boundary and the two generic event-driven
//! engines; the [`execution`] module holds the order, fill, and position
//! vocabulary, the causal fill engine, and the documented fill policy; the
//! [`portfolio`] module holds the capital configuration, sizing, state, records,
//! and the ledger.
//!
//! The axis ladder, its as-of context, and the runner left with the four-axis
//! cutover. S3.x rebuilds the multi-timeframe strategy context on the flat model.
//!
//! The crate is pure. It performs no I/O, reads no clock, uses no randomness,
//! and keeps no global mutable state. The shell owns source access and
//! rendering.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod benchmark;
pub mod execution;
pub mod intent;
pub mod portfolio;
pub mod side;
pub mod strategy;
