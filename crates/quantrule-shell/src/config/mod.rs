//! Strict JSON configuration for one run.
//!
//! The shell owns the schema because the shell owns the file boundary: the
//! engine crates receive validated Rust values and never parse a file. One
//! module covers each file, and a shared reader enforces the contract's common
//! rules.
//!
//! The loader reads and validates everything before a candle is read or an
//! output byte is written. A rejection names the file and the JSON path, never
//! a stack trace or an absolute path.
//!
//! The backtest and report commands consume the loader in #172-#176; until
//! then the public surface of this module is exercised only by its tests.
#![allow(dead_code, unused_imports)]

mod analytics;
mod data;
mod error;
mod execution;
mod model;
mod portfolio;
mod read;
mod run;
mod strategy;

#[cfg(test)]
mod tests;

pub(crate) use analytics::AnalyticsConfig;
pub(crate) use data::{DataConfig, DataSource};
pub(crate) use error::ConfigError;
pub(crate) use execution::ExecutionRunConfig;
pub(crate) use run::{ConfigPaths, RunConfig};
