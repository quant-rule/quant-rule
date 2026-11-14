//! The simulation service binary.
//!
//! Milestone M1 serves `/health` and `/version` only. The research plane stays
//! out of this binary until P7; it claims no simulation behavior yet.

#![forbid(unsafe_code)]

use quantrule_runtime::{Config, HealthRegistry, RuntimeError, logging, serve};

#[tokio::main]
async fn main() -> Result<(), RuntimeError> {
    let config = Config::from_env()?;
    logging::init("info")?;
    serve(
        &config.bind,
        quantrule_simulation_service::router(&config, HealthRegistry::new()),
    )
    .await
}
