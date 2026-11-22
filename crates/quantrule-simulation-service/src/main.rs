//! The simulation service binary.
//!
//! It connects to Postgres and the object store, applies only the `simulation`
//! migration ledger, and serves the eleven research endpoints. It runs no queue
//! consumer and no worker loop: the research plane is synchronous and
//! interactive.

#![forbid(unsafe_code)]

use std::sync::Arc;

use quantrule_runtime::adapters::ObjectStore;
use quantrule_runtime::backends::{Objects, migrate_owner_database, open_service_database};
use quantrule_runtime::config::migrate_only_requested;
use quantrule_runtime::metrics::{self, Metrics};
use quantrule_runtime::request_id::RequestIds;
use quantrule_runtime::{Config, HealthRegistry, RuntimeError, logging, observability};
use quantrule_simulation_service::postgres::PgSimulationStore;
use quantrule_simulation_service::store::SimulationStore;
use quantrule_simulation_service::{SimulationServices, SimulationState, router};

#[tokio::main]
async fn main() -> Result<(), RuntimeError> {
    let config = Config::from_env()?;
    logging::init("simulation", config.image_digest.as_ref())?;

    if migrate_only_requested() {
        return migrate_owner_database(
            &config,
            "simulation",
            "simulation",
            &quantrule_simulation_service::migrations::available(),
        )
        .await;
    }

    let metrics = Metrics::new(
        "simulation",
        env!("CARGO_PKG_VERSION"),
        config.image_digest.as_ref().map(ToString::to_string),
    );
    metrics::start(&config, &metrics).await?;
    let mut health = HealthRegistry::named("simulation").with_metrics(metrics.clone());
    let opened = open_service_database(&config, &mut health).await?;
    let store: Option<Arc<dyn SimulationStore>> = opened.map(|database| {
        Arc::new(PgSimulationStore::new(database.pool().clone())) as Arc<dyn SimulationStore>
    });

    let backend = Objects::open(&config).await?;
    let objects: Arc<dyn ObjectStore> = Arc::new(backend.clone());
    health.register("object-store", move || {
        let backend = backend.clone();
        async move { backend.check().await.map_err(|error| error.to_string()) }
    });

    let state = Arc::new(SimulationState::with_services(
        &config,
        health,
        SimulationServices {
            store,
            objects: Some(objects),
        },
    ));
    tracing::info!(bind = %config.bind, "starting simulation service");
    quantrule_runtime::serve_config(
        &config,
        observability::instrument(router(state), metrics, RequestIds::from_env("simulation")),
    )
    .await
}
