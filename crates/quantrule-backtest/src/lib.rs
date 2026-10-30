//! The backtest engine.
//!
//! The crate is the only place that decides a trade. It consumes the published
//! model values and simulates execution and, in later work items, portfolio
//! accounting. The [`strategy`] module holds the type boundary and the first
//! axis-agreement strategy; the [`execution`] module holds the order, fill, and
//! position vocabulary, the causal fill engine, and the documented fill policy;
//! the [`portfolio`] module holds the capital configuration, sizing, state,
//! records, and the ledger; the [`runner`] module composes all three in one
//! deterministic pass.
//!
//! The crate is pure. It performs no I/O, reads no clock, uses no randomness,
//! and keeps no global mutable state. The shell owns source access and
//! rendering.
//!
//! The boundary is recorded in
//! `docs/adr/2026-10-27/analysis-and-backtest-boundaries.md`.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod execution;
pub mod portfolio;
pub mod runner;
pub mod side;
pub mod strategy;
