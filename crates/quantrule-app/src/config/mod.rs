//! Strict JSON configuration for one run.
//!
//! The application service owns the schema because it owns the file boundary: the
//! engine crates receive validated Rust values and never parse a file. One
//! module covers each file; a shared reader enforces the contract's common
//! rules, and a shared cost module keeps fee, slippage, commission and funding
//! identical across the run files and the execution pack.
//!
//! The loader reads and validates everything before a candle is read or an
//! output byte is written. A rejection names the file and the JSON path, never
//! a stack trace or an absolute path.

pub(crate) mod costs;
mod data;
pub(crate) mod error;
mod execution;
mod model;
mod model_v4;
pub use model_v4::effective_json;
mod portfolio;
mod qualification;
pub(crate) mod read;
mod run;
mod run_v5;
pub mod spec;
mod strategy;

#[cfg(test)]
mod tests;

pub use data::DataConfig;
pub use error::ConfigError;
#[cfg(test)]
pub(crate) use error::ConfigErrorKind;
pub use qualification::{Correction, CostModel, Dimension, MultipleTesting, Operator};
pub use qualification::{
    Criteria, Criterion, EvidenceArtifact, Partition, load as load_qualification,
};
pub use run::{RUN_SCHEMA, RUN_SCHEMA_V3, RUN_SCHEMA_V5, RunConfig};
pub use run_v5::parse_references as parse_v5_references;
pub use run_v5::{RunAccount, RunV5};
pub use strategy::parse as parse_strategy;
pub use strategy::{StrategyConfig, load as load_strategy};
pub(crate) use strategy::{StrategyEngine, parse_risk, parse_timeframe};
