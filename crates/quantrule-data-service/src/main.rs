//! The data service binary.
//!
//! The server profile connects to Postgres and the object store, applies only
//! the catalog migrations, and serves the routes the gateway proxies. The
//! desktop profile opens the embedded SQLite file under `QR_DATA_DIR`, applies
//! the catalog SQLite baseline to the shared ledger, and serves the same routes
//! from the same traits. The object store is selected by the same profile: S3
//! on the server, the filesystem CAS under `<QR_DATA_DIR>/objects` on the
//! desktop. The profile header is trusted because the gateway validates and
//! injects it.

#![forbid(unsafe_code)]

use std::sync::Arc;

use quantrule_runtime::adapters::ObjectStore;
use quantrule_runtime::backends::{Objects, ServiceDatabase, open_service_database};
use quantrule_runtime::{Config, HealthRegistry, RuntimeError, logging, serve};

use quantrule_data_service::postgres::{PgConfigStore, PgDatasetCatalog};
use quantrule_data_service::sqlite::{SqliteConfigStore, SqliteDatasetCatalog};
use quantrule_data_service::{DataServices, DataState};

/// The catalog a route handler reads.
type CatalogHandle = Arc<dyn quantrule_data_service::DatasetCatalog>;
/// The config catalog a route handler reads.
type ConfigHandle = Arc<dyn quantrule_data_service::ConfigCatalog>;

#[tokio::main]
async fn main() -> Result<(), RuntimeError> {
    let config = Config::from_env()?;
    logging::init("info")?;

    let mut health = HealthRegistry::named("data");
    let opened = open_service_database(
        &config,
        "catalog",
        "catalog",
        &quantrule_data_service::migrations::sqlite(),
        &quantrule_data_service::migrations::available(),
        &mut health,
    )
    .await?;
    let (catalog, configs): (Option<CatalogHandle>, Option<ConfigHandle>) = match opened {
        Some(ServiceDatabase::Sqlite(database)) => (
            Some(Arc::new(SqliteDatasetCatalog::new(database.clone()))),
            Some(Arc::new(SqliteConfigStore::new(database))),
        ),
        Some(ServiceDatabase::Postgres(database)) => (
            Some(Arc::new(PgDatasetCatalog::new(database.pool().clone()))),
            Some(Arc::new(PgConfigStore::new(database.pool().clone()))),
        ),
        None => (None, None),
    };

    let backend = Objects::open(&config).await?;
    let objects: Arc<dyn ObjectStore> = Arc::new(backend.clone());
    health.register("object-store", move || {
        let backend = backend.clone();
        async move { backend.check().await.map_err(|error| error.to_string()) }
    });

    let state = Arc::new(DataState::with_services(
        &config,
        health,
        DataServices {
            catalog,
            configs,
            objects: Some(objects),
        },
    ));
    tracing::info!(bind = %config.bind, "starting data service");
    serve(&config.bind, quantrule_data_service::router(state)).await
}
