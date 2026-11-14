//! The gateway binary.
//!
//! The gateway owns profiles, the static client, the proxy surface, health, and
//! the run and hypothesis routes until their services land. It reads no
//! catalog, config, or model schema: data and model calls go through the
//! internal client, and the public data and model routes are proxied to `qr-data`
//! and `qr-model`.

use std::sync::Arc;

use quantrule_runtime::adapters::postgres::{PgDatabase, apply_migrations_in};
use quantrule_runtime::adapters::rabbitmq::RabbitQueue;
use quantrule_runtime::adapters::{Database, Queue};
use quantrule_runtime::client::{ClientError, DataClient, HttpServiceClient, Service};
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

    let data_upstream = HttpServiceClient::from_env(Service::Data)
        .map_err(|error| service_config_error(Service::Data, error))?;
    let data: Arc<dyn DataClient> = Arc::new(data_upstream.clone());
    let data_upstream = Arc::new(data_upstream);
    let model_upstream = HttpServiceClient::from_env(Service::Model)
        .map_err(|error| service_config_error(Service::Model, error))?;

    let state = Arc::new(AppState::with_services(
        &config,
        health,
        Services {
            hypotheses,
            profiles,
            runs,
            queue: run_queue,
            data: Some(data),
            upstream: Some(data_upstream),
            model_upstream: Some(Arc::new(model_upstream)),
        },
    ));
    tracing::info!(bind = %config.bind, "starting gateway");
    server::serve(&config, state).await
}

/// Name the missing or invalid `QR_*_URL` at startup.
fn service_config_error(service: Service, error: ClientError) -> RuntimeError {
    let name = service.url_variable();
    RuntimeError::Config(match error {
        ClientError::Unconfigured { .. } => ConfigError::Missing { name },
        _ => ConfigError::Invalid {
            name,
            value: "<invalid>".to_owned(),
            reason: "must be an absolute HTTP URL",
        },
    })
}
