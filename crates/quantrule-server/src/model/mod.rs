//! The model service.
//!
//! The service loads a dataset by reference, materializes a validated run
//! config, computes each ladder timeframe, and stores one JSON object in CAS.
//! It never reads a strategy, execution, or portfolio value.

pub mod compute;
pub mod config;
pub mod routes;

pub use compute::{Computed, ModelError, compute_full, compute_sharded};
pub use config::{ModelConfig, load_config};
pub use routes::compute_model;
