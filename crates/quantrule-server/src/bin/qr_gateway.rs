//! The gateway binary.
//!
//! Phase 1 serves health and version. The routes arrive with their services; the
//! startup, the probes, and the shutdown are final.

use std::sync::Arc;

use quantrule_server::runtime::{
    AppState, Config, HealthRegistry, RuntimeError, amqp, logging, pg, s3, server,
};

#[tokio::main]
async fn main() -> Result<(), RuntimeError> {
    let config = Config::from_env()?;
    logging::init("info")?;

    let mut health = HealthRegistry::new();

    let pool = pg::connect(&config.database_url).await.ok();
    if let Some(pool) = pool.clone() {
        health.register("postgres", move || {
            let pool = pool.clone();
            async move { pg::check(&pool).await }
        });
    } else {
        health.register("postgres", || async {
            Err("not connected at startup".to_owned())
        });
    }

    let store = s3::client(&config.object_store_url).await;
    health.register("object-store", move || {
        let store = store.clone();
        async move { s3::check(&store).await }
    });

    let broker = amqp::connect(&config.queue_url).await.ok();
    if let Some(connection) = broker {
        let connection = Arc::new(connection);
        health.register("queue", move || {
            let connection = Arc::clone(&connection);
            async move { amqp::check(&connection) }
        });
    } else {
        health.register("queue", || async {
            Err("not connected at startup".to_owned())
        });
    }

    let state = Arc::new(AppState::new(&config, health));
    tracing::info!(bind = %config.bind, "starting gateway");
    server::serve(&config, state).await
}
