//! The gateway binary.
//!
//! The gateway owns profiles, the static client, the proxy surface, and health.
//! It reads no catalog, run, research, or model schema: data, model,
//! evidence, and simulation calls go through the internal client, and the
//! public routes are proxied to `qr-data`, `qr-model`, `qr-evidence`, and
//! `qr-simulation`.

use std::sync::Arc;

use quantrule_runtime::adapters::Database;
use quantrule_runtime::adapters::postgres::{PgDatabase, apply_migrations_in};
use quantrule_runtime::client::{ClientError, DataClient, HttpServiceClient, Service};
use quantrule_runtime::config::ConfigError;
use quantrule_runtime::{Config, HealthRegistry, RuntimeError, logging};
use quantrule_server::adapters::postgres::PgProfileStore;
use quantrule_server::profile::ProfileStore;
use quantrule_server::server::{self, AppState, Services};

#[tokio::main]
async fn main() -> Result<(), RuntimeError> {
    let config = Config::from_env()?;
    logging::init("info")?;

    let mut health = HealthRegistry::new();
    let mut profiles: Option<Arc<dyn ProfileStore>> = None;

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
                tracing::info!(gateway, "gateway migrations applied");
            }
            profiles = Some(Arc::new(PgProfileStore::new(database.pool().clone())));
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

    let data_upstream = HttpServiceClient::from_env(Service::Data)
        .map_err(|error| service_config_error(Service::Data, error))?;
    let data: Arc<dyn DataClient> = Arc::new(data_upstream.clone());
    let data_upstream = Arc::new(data_upstream);
    let model_upstream = HttpServiceClient::from_env(Service::Model)
        .map_err(|error| service_config_error(Service::Model, error))?;
    let evidence_upstream = HttpServiceClient::from_env(Service::Evidence)
        .map_err(|error| service_config_error(Service::Evidence, error))?;
    let simulation_upstream = HttpServiceClient::from_env(Service::Simulation)
        .map_err(|error| service_config_error(Service::Simulation, error))?;

    let state = Arc::new(AppState::with_services(
        &config,
        health,
        Services {
            profiles,
            data: Some(data),
            upstream: Some(data_upstream),
            model_upstream: Some(Arc::new(model_upstream)),
            evidence_upstream: Some(Arc::new(evidence_upstream)),
            simulation_upstream: Some(Arc::new(simulation_upstream)),
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
