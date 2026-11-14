//! The QuantRule gateway and backtest worker.
//!
//! P5 moved the run, research, and evidence modules to
//! `quantrule-evidence-service`. The gateway owns profiles, static files, health,
//! and the proxy table; the worker owns backtest execution. P8 retires this
//! crate in favour of `quantrule-gateway` and `quantrule-backtest-service`.

#![forbid(unsafe_code)]

pub mod adapters;
pub mod backtest;
pub mod migrations;
pub mod profile;
pub mod server;
pub mod static_files;
