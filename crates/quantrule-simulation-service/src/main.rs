//! The simulation service binary.
//!
//! On the server profile it connects to Postgres and the object store, applies
//! only the `simulation` migration ledger, and serves the eleven research
//! endpoints. On the embedded desktop profile it opens `quantrule.sqlite` under
//! `QR_DATA_DIR`, applies the simulation SQLite baseline to the shared ledger,
//! and serves the same endpoints through the existing repository trait. It runs
//! no queue consumer and no worker loop: the research plane is synchronous and
//! interactive.

#![forbid(unsafe_code)]

use std::sync::Arc;

use quantrule_runtime::adapters::Database;
use quantrule_runtime::adapters::ObjectStore;
use quantrule_runtime::adapters::postgres::{PgDatabase, apply_migrations_in};
use quantrule_runtime::adapters::s3::S3Store;
use quantrule_runtime::adapters::sqlite::{
    SqliteDatabase, apply_migrations_in as apply_sqlite_migrations,
};
use quantrule_runtime::config::ConfigError;
use quantrule_runtime::{Config, DeploymentProfile, HealthRegistry, RuntimeError, logging};
use quantrule_simulation_service::postgres::PgSimulationStore;
use quantrule_simulation_service::sqlite::SqliteSimulationStore;
use quantrule_simulation_service::store::SimulationStore;
use quantrule_simulation_service::{SimulationServices, SimulationState, router};

#[tokio::main]
async fn main() -> Result<(), RuntimeError> {
    let config = Config::from_env()?;
    logging::init("info")?;

    let mut health = HealthRegistry::new();
    let mut store: Option<Arc<dyn SimulationStore>> = None;

    if config.profile == DeploymentProfile::Desktop {
        let data_dir = config.data_dir.as_deref().ok_or(ConfigError::Missing {
            name: "QR_DATA_DIR",
        })?;
        let path = data_dir.join(quantrule_runtime::adapters::sqlite::DATABASE_FILE);
        let database = SqliteDatabase::open(&path)
            .map_err(|error| RuntimeError::Database(error.to_string()))?;
        if config.migrate {
            let applied = apply_sqlite_migrations(
                &database,
                "simulation",
                &quantrule_simulation_service::migrations::sqlite(),
            )
            .await
            .map_err(|error| RuntimeError::Migrations(error.to_string()))?;
            tracing::info!(applied, "simulation SQLite migrations applied");
        }
        store = Some(Arc::new(SqliteSimulationStore::new(database.clone())));
        health.register("sqlite", move || {
            let database = database.clone();
            async move { database.ping().await.map_err(|error| error.to_string()) }
        });
    } else {
        match PgDatabase::connect(&config.database_url).await {
            Ok(database) => {
                if config.migrate {
                    let applied = apply_migrations_in(
                        database.pool(),
                        "simulation",
                        &quantrule_simulation_service::migrations::available(),
                    )
                    .await
                    .map_err(|error| RuntimeError::Migrations(error.to_string()))?;
                    tracing::info!(applied, "simulation migrations applied");
                }
                store = Some(Arc::new(PgSimulationStore::new(database.pool().clone())));
                health.register("postgres", move || {
                    let database = database.clone();
                    async move { database.ping().await.map_err(|error| error.to_string()) }
                })
            }
            Err(error) => health.register("postgres", move || {
                let message = error.to_string();
                async move { Err(message) }
            }),
        }
    }

    let s3 = S3Store::new(&config.object_store_url, &config.object_store_bucket).await;
    let objects: Arc<dyn ObjectStore> = Arc::new(s3.clone());
    health.register("object-store", move || {
        let store = s3.clone();
        async move { store.check().await.map_err(|error| error.to_string()) }
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
