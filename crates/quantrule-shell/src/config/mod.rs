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

mod data;
mod error;
mod execution;
mod model;
mod model_v4;
pub(crate) use model_v4::effective_json;
mod policy;
mod portfolio;
mod qualification;
mod read;
mod run;
mod strategy;

#[cfg(test)]
mod tests;

pub(crate) use error::ConfigError;
#[cfg(test)]
pub(crate) use qualification::{Correction, CostModel, Dimension, MultipleTesting, Operator};
pub(crate) use qualification::{
    Criteria, Criterion, EvidenceArtifact, Partition, load as load_qualification,
};
pub(crate) use run::RunConfig;
#[cfg(test)]
pub(crate) use strategy::parse as parse_strategy;
pub(crate) use strategy::{StrategyConfig, load as load_strategy};
