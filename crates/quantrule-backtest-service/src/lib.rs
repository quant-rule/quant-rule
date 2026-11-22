//! The QuantRule backtest service.
//!
//! The service owns the worker that consumes `run.requested`, reads the config
//! and dataset through `qr-data`, obtains the model bundle from `qr-model`,
//! runs strategy, execution, and portfolio from that bundle, writes the
//! artifact set to CAS, and records the computed digest through `qr-evidence`.
//! It owns no database schema and reads no catalog, run, or research table.
//!
//! The worker is the only place the strategy, execution, and portfolio engines
//! run for a queued run. It never computes a model, renders a label, or accepts
//! a client path. A separate diagnostic stage worker runs one engine behind an
//! executor seam; it never writes a run or a canonical digest.

#![forbid(unsafe_code)]

pub mod backtest;
pub mod drain;
pub mod stage;
