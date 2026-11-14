//! The data service binary.
//!
//! It connects to Postgres and the object store, applies only the catalog
//! migrations, and serves the routes the gateway proxies. The profile header is
//! trusted because the gateway validates and injects it.

#![forbid(unsafe_code)]

use std::sync::Arc;

use quantrule_runtime::adapters::postgres::{PgDatabase, apply_migrations_in};
use quantrule_runtime::adapters::s3::S3Store;
use quantrule_runtime::adapters::{Database, ObjectStore};
use quantrule_runtime::{Config, HealthRegistry, RuntimeError, logging, serve};

use quantrule_data_service::postgres::{PgConfigStore, PgDatasetCatalog};
use quantrule_data_service::{DataServices, DataState};

#[tokio::main]
async fn main() -> Result<(), RuntimeError> {
    let config = Config::from_env()?;
    logging::init("info")?;

    let mut health = HealthRegistry::new();
    let mut catalog: Option<Arc<dyn quantrule_data_service::DatasetCatalog>> = None;
    let mut configs: Option<Arc<dyn quantrule_data_service::ConfigCatalog>> = None;

    match PgDatabase::connect(&config.database_url).await {
        Ok(database) => {
            if config.migrate {
                let applied = apply_migrations_in(
                    database.pool(),
                    "catalog",
                    &quantrule_data_service::migrations::available(),
                )
                .await
                .map_err(|error| RuntimeError::Migrations(error.to_string()))?;
                tracing::info!(applied, "catalog migrations applied");
            }
            catalog = Some(Arc::new(PgDatasetCatalog::new(database.pool().clone())));
            configs = Some(Arc::new(PgConfigStore::new(database.pool().clone())));
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

    let s3 = S3Store::new(&config.object_store_url, &config.object_store_bucket).await;
    let objects: Arc<dyn ObjectStore> = Arc::new(s3.clone());
    health.register("object-store", move || {
        let store = s3.clone();
        async move { store.check().await.map_err(|error| error.to_string()) }
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
