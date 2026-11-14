//! The model engine surface.
//!
//! The module loads a dataset by reference, materializes a validated run config,
//! computes each ladder timeframe, and renders one JSON object. It never reads a
//! strategy, execution, or portfolio value.

pub mod compute;
pub mod config;
pub mod series;

pub use compute::{ModelError, compute_sharded};
pub use config::{ModelConfig, load_run_config};
