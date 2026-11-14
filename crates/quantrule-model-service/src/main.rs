//! The model service binary.
//!
//! Milestone M1 serves `/health` and `/version` only. Model compute stays in
//! the gateway until P4; this binary claims no model behavior yet.

#![forbid(unsafe_code)]

use quantrule_runtime::{Config, HealthRegistry, RuntimeError, logging, serve};

#[tokio::main]
async fn main() -> Result<(), RuntimeError> {
    let config = Config::from_env()?;
    logging::init("info")?;
    serve(
        &config.bind,
        quantrule_model_service::router(&config, HealthRegistry::new()),
    )
    .await
}
