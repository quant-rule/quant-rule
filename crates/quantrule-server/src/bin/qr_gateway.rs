//! The gateway binary.
//!
//! The gateway owns profiles, the static client, the proxy surface, health, and
//! the run, hypothesis, and model routes until their services land. It reads no
//! catalog or config schema: data calls go through the internal client and the
//! public data routes are proxied to `qr-data`.

use std::sync::Arc;

use quantrule_runtime::adapters::postgres::{PgDatabase, apply_migrations_in};
use quantrule_runtime::adapters::rabbitmq::RabbitQueue;
use quantrule_runtime::adapters::s3::S3Store;
use quantrule_runtime::adapters::{Database, ObjectStore, Queue};
use quantrule_runtime::client::{DataClient, HttpServiceClient, Service};
use quantrule_runtime::config::ConfigError;
use quantrule_runtime::{Config, HealthRegistry, RuntimeError, logging};
use quantrule_server::adapters::postgres::{PgHypothesisStore, PgProfileStore, PgRunStore};
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
    let mut profiles: Option<Arc<dyn ProfileStore>> = None;
    let mut runs: Option<Arc<dyn RunStore>> = None;
    let mut run_queue: Option<Arc<dyn Queue>> = None;

    match PgDatabase::connect(&config.database_url).await {
        Ok(database) => {
            if config.migrate {
                let gateway = apply_migrations_in(
                    database.pool(),
                    "gateway",
                    &quantrule_server::migrations::gateway(),
                )
                .await
                .map_err(|error| RuntimeError::Migrations(error.to_string()))?;
                let service = apply_migrations_in(
                    database.pool(),
                    "runs",
                    &quantrule_server::migrations::service(),
                )
                .await
                .map_err(|error| RuntimeError::Migrations(error.to_string()))?;
                tracing::info!(gateway, service, "migrations applied");
            }
            hypotheses = Some(Arc::new(PgHypothesisStore::new(database.pool().clone())));
            profiles = Some(Arc::new(PgProfileStore::new(database.pool().clone())));
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

    let upstream = HttpServiceClient::from_env(Service::Data).map_err(data_config_error)?;
    let data: Arc<dyn DataClient> = Arc::new(upstream.clone());
    let upstream = Arc::new(upstream);

    let state = Arc::new(AppState::with_services(
        &config,
        health,
        Services {
            hypotheses,
            objects: Some(objects),
            profiles,
            runs,
            queue: run_queue,
            data: Some(data),
            upstream: Some(upstream),
        },
    ));
    tracing::info!(bind = %config.bind, "starting gateway");
    server::serve(&config, state).await
}

/// Name the missing or invalid `QR_DATA_URL` at startup.
fn data_config_error(error: quantrule_runtime::client::ClientError) -> RuntimeError {
    RuntimeError::Config(match error {
        quantrule_runtime::client::ClientError::Unconfigured { .. } => ConfigError::Missing {
            name: "QR_DATA_URL",
        },
        _ => ConfigError::Invalid {
            name: "QR_DATA_URL",
            value: "<invalid>".to_owned(),
            reason: "must be an absolute HTTP URL",
        },
    })
}
