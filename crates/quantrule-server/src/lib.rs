//! The QuantRule gateway and backtest worker.
//!
//! P5 moved the run, research, and evidence modules to
//! `quantrule-evidence-service`. P6 moved the worker onto the HTTP data plane:
//! it reads data and the model bundle through the service clients and runs the
//! staged backtest from that bundle. The gateway owns profiles, static files,
//! health, and the proxy table; the worker owns backtest orchestration. P8
//! retires this crate in favour of `quantrule-gateway` and
//! `quantrule-backtest-service`.

#![forbid(unsafe_code)]

pub mod adapters;
pub mod backtest;
pub mod migrations;
pub mod profile;
pub mod server;
pub mod static_files;
