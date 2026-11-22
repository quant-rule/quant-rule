//! The data service binary.
//!
//! It connects to Postgres and the object store, applies only the catalog
//! migrations, and serves the routes the gateway proxies. The profile header is
//! trusted because the gateway validates and injects it.

#![forbid(unsafe_code)]

use std::sync::Arc;

use quantrule_runtime::adapters::ObjectStore;
use quantrule_runtime::backends::{Objects, migrate_owner_database, open_service_database};
use quantrule_runtime::config::migrate_only_requested;
use quantrule_runtime::metrics::{self, Metrics};
use quantrule_runtime::request_id::RequestIds;
use quantrule_runtime::{
    Config, HealthRegistry, RuntimeError, logging, observability, serve_config,
};

use quantrule_data_service::artifacts::PgArtifacts;
use quantrule_data_service::packs::PgPacks;
use quantrule_data_service::postgres::{PgConfigStore, PgDatasetCatalog};
use quantrule_data_service::{DataServices, DataState};

/// The catalog a route handler reads.
type CatalogHandle = Arc<dyn quantrule_data_service::DatasetCatalog>;
/// The config catalog a route handler reads.
type ConfigHandle = Arc<dyn quantrule_data_service::ConfigCatalog>;
/// The pack registry a route handler reads.
type PackHandle = Arc<dyn quantrule_data_service::PackRegistry>;
/// The artifact catalog a route handler reads.
type ArtifactHandle = Arc<dyn quantrule_data_service::ArtifactCatalog>;

#[tokio::main]
async fn main() -> Result<(), RuntimeError> {
    let config = Config::from_env()?;
    logging::init("data", config.image_digest.as_ref())?;

    if migrate_only_requested() {
        return migrate_owner_database(
            &config,
            "catalog",
            "catalog",
            &quantrule_data_service::migrations::available(),
        )
        .await;
    }

    let metrics = Metrics::new(
        "data",
        env!("CARGO_PKG_VERSION"),
        config.image_digest.as_ref().map(ToString::to_string),
    );
    metrics::start(&config, &metrics).await?;
    let mut health = HealthRegistry::named("data").with_metrics(metrics.clone());
    let opened = open_service_database(&config, &mut health).await?;
    let (catalog, configs, packs, artifacts): (
        Option<CatalogHandle>,
        Option<ConfigHandle>,
        Option<PackHandle>,
        Option<ArtifactHandle>,
    ) = match opened {
        Some(database) => {
            let pool = database.pool().clone();
            (
                Some(Arc::new(PgDatasetCatalog::new(pool.clone()))),
                Some(Arc::new(PgConfigStore::new(pool.clone()))),
                Some(Arc::new(PgPacks::new(pool.clone()))),
                Some(Arc::new(PgArtifacts::new(pool))),
            )
        }
        None => (None, None, None, None),
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
            packs,
            artifacts,
            objects: Some(objects),
        },
    ));
    tracing::info!(bind = %config.bind, "starting data service");
    serve_config(
        &config,
        observability::instrument(
            quantrule_data_service::router(state),
            metrics,
            RequestIds::from_env("data"),
        ),
    )
    .await
}
