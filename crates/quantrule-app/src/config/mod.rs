//! Strict JSON configuration for one run.
//!
//! The application service owns the schema because it owns the file boundary: the
//! engine crates receive validated Rust values and never parse a file. One
//! module covers each file, and a shared reader enforces the contract's common
//! rules.
//!
//! The loader reads and validates everything before a candle is read or an
//! output byte is written. A rejection names the file and the JSON path, never
//! a stack trace or an absolute path.

mod data;
mod error;
mod execution;
mod model;
mod model_v4;
pub use model_v4::effective_json;
mod portfolio;
mod qualification;
mod read;
mod run;
mod strategy;

#[cfg(test)]
mod tests;

pub use error::ConfigError;
pub use qualification::{Correction, CostModel, Dimension, MultipleTesting, Operator};
pub use qualification::{
    Criteria, Criterion, EvidenceArtifact, Partition, load as load_qualification,
};
pub use run::{RUN_SCHEMA, RunConfig};
pub use strategy::parse as parse_strategy;
pub use strategy::{StrategyConfig, load as load_strategy};
