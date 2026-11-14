//! The gateway binary.
//!
//! Phase 1 serves health and version. The routes arrive with their services; the
//! startup, the probes, and the shutdown are final.

use std::sync::Arc;

use quantrule_runtime::adapters::postgres::{PgDatabase, apply_migrations};
use quantrule_runtime::adapters::rabbitmq::RabbitQueue;
use quantrule_runtime::adapters::s3::S3Store;
use quantrule_runtime::adapters::{Database, ObjectStore, Queue};
use quantrule_runtime::{Config, HealthRegistry, RuntimeError, logging};
use quantrule_server::adapters::postgres::{
    PgConfigStore, PgDatasetCatalog, PgHypothesisStore, PgProfileStore, PgRunStore,
};
use quantrule_server::configs::ConfigCatalog;
use quantrule_server::data::DatasetCatalog;
use quantrule_server::profile::ProfileStore;
use quantrule_server::research::HypothesisStore;
use quantrule_server::runs::RunStore;
use quantrule_server::server::{self, AppState, Services};

#[tokio::main]
async fn main() -> Result<(), RuntimeError> {
    let config = Config::from_env()?;
    logging::init("info")?;

    let mut health = HealthRegistry::new();
    let mut hypotheses: Option<Arc<dyn HypothesisStore>> = None;
    let mut catalog: Option<Arc<dyn DatasetCatalog>> = None;
    let mut profiles: Option<Arc<dyn ProfileStore>> = None;
    let mut configs: Option<Arc<dyn ConfigCatalog>> = None;
    let mut runs: Option<Arc<dyn RunStore>> = None;
    let mut run_queue: Option<Arc<dyn Queue>> = None;

    match PgDatabase::connect(&config.database_url).await {
        Ok(database) => {
            if config.migrate {
                let applied =
                    apply_migrations(database.pool(), &quantrule_server::migrations::available())
                        .await
                        .map_err(|error| RuntimeError::Migrations(error.to_string()))?;
                tracing::info!(applied, "migrations applied");
            }
            hypotheses = Some(Arc::new(PgHypothesisStore::new(database.pool().clone())));
            catalog = Some(Arc::new(PgDatasetCatalog::new(database.pool().clone())));
            profiles = Some(Arc::new(PgProfileStore::new(database.pool().clone())));
            configs = Some(Arc::new(PgConfigStore::new(database.pool().clone())));
            runs = Some(Arc::new(PgRunStore::new(database.pool().clone())));
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

    match RabbitQueue::connect(&config.queue_url).await {
        Ok(queue) => {
            let queue = Arc::new(queue);
            let trait_queue: Arc<dyn Queue> = queue.clone();
            run_queue = Some(trait_queue);
            health.register("queue", move || {
                let queue = Arc::clone(&queue);
                async move { queue.check().map_err(|error| error.to_string()) }
            });
        }
        Err(error) => health.register("queue", move || {
            let message = error.to_string();
            async move { Err(message) }
        }),
    }

    let state = Arc::new(AppState::with_services(
        &config,
        health,
        Services {
            hypotheses,
            catalog,
            objects: Some(objects),
            profiles,
            configs,
            runs,
            queue: run_queue,
        },
    ));
    tracing::info!(bind = %config.bind, "starting gateway");
    server::serve(&config, state).await
}
