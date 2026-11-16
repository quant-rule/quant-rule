//! The simulation service binary.
//!
//! On the server profile it connects to Postgres and the object store, applies
//! only the `simulation` migration ledger, and serves the eleven research
//! endpoints. On the embedded desktop profile it opens `quantrule.sqlite` under
//! `QR_DATA_DIR`, applies the simulation SQLite baseline to the shared ledger,
//! and serves the same endpoints through the existing repository trait. The
//! object store is selected by the same profile, so a recorded bundle read on
//! the desktop comes from `<QR_DATA_DIR>/objects`. It runs no queue consumer and
//! no worker loop: the research plane is synchronous and interactive.

#![forbid(unsafe_code)]

use std::sync::Arc;

use quantrule_runtime::adapters::ObjectStore;
use quantrule_runtime::backends::{Objects, ServiceDatabase, open_service_database};
use quantrule_runtime::{Config, HealthRegistry, RuntimeError, logging};
use quantrule_simulation_service::postgres::PgSimulationStore;
use quantrule_simulation_service::sqlite::SqliteSimulationStore;
use quantrule_simulation_service::store::SimulationStore;
use quantrule_simulation_service::{SimulationServices, SimulationState, router};

#[tokio::main]
async fn main() -> Result<(), RuntimeError> {
    let config = Config::from_env()?;
    logging::init("info")?;

    let mut health = HealthRegistry::new();
    let opened = open_service_database(
        &config,
        "simulation",
        "simulation",
        &quantrule_simulation_service::migrations::sqlite(),
        &quantrule_simulation_service::migrations::available(),
        &mut health,
    )
    .await?;
    let store: Option<Arc<dyn SimulationStore>> = match opened {
        Some(ServiceDatabase::Sqlite(database)) => {
            Some(Arc::new(SqliteSimulationStore::new(database)))
        }
        Some(ServiceDatabase::Postgres(database)) => {
            Some(Arc::new(PgSimulationStore::new(database.pool().clone())))
        }
        None => None,
    };

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
    quantrule_runtime::serve(&config.bind, router(state)).await
}
