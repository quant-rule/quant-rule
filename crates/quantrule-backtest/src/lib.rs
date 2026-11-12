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
//! The multi-timeframe strategy context and the runner are implemented in the
//! strategy and shell composition layers.
//!
//! The crate is pure. It performs no I/O, reads no clock, uses no randomness,
//! and keeps no global mutable state. The application service and the shell own
//! source access; the shell and the desktop own rendering.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod benchmark;
pub mod execution;
pub mod intent;
pub mod portfolio;
pub mod side;
pub mod strategy;
