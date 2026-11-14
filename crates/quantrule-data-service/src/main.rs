//! The data service binary.
//!
//! Milestone M1 serves `/health` and `/version` only. The catalog, dataset,
//! config, and inspection routes stay in the gateway until P3; this binary
//! claims no data behavior yet.

#![forbid(unsafe_code)]

use quantrule_runtime::{Config, HealthRegistry, RuntimeError, logging, serve};

#[tokio::main]
async fn main() -> Result<(), RuntimeError> {
    let config = Config::from_env()?;
    logging::init("info")?;
    serve(
        &config.bind,
        quantrule_data_service::router(&config, HealthRegistry::new()),
    )
    .await
}
