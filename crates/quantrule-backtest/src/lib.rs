//! The backtest engine.
//!
//! The crate is the only place that decides a trade. It consumes the published
//! model values and, in later work items, simulates execution and portfolio
//! accounting. The [`strategy`] module holds the type boundary and the first
//! axis-agreement strategy.
//!
//! The crate is pure. It performs no I/O, reads no clock, uses no randomness,
//! and keeps no global mutable state. The shell owns source access and
//! rendering.
//!
//! The boundary is recorded in
//! `docs/adr/2026-10-27/analysis-and-backtest-boundaries.md`.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod strategy;
