//! The gateway binary.
//!
//! The gateway owns profiles, the static client, the proxy surface, and health.
//! It reads no catalog, run, research, or model schema: data, model, evidence,
//! simulation, and strategy calls go through the internal client, and the
//! public routes are proxied to `qr-data`, `qr-model`, `qr-evidence`, and
//! `qr-simulation`. The strategy service is probed only when its optional URL is
//! configured. The profile store is Postgres.

use std::sync::Arc;

use quantrule_gateway::adapters::postgres::PgProfileStore;
use quantrule_gateway::profile::ProfileStore;
use quantrule_gateway::server::{self, AppState, Services};
use quantrule_runtime::backends::{migrate_owner_database, open_service_database};
use quantrule_runtime::client::{ClientError, DataClient, HttpServiceClient, Service};
use quantrule_runtime::config::{ConfigError, migrate_only_requested};
use quantrule_runtime::{Config, HealthRegistry, RuntimeError, logging};

#[tokio::main]
async fn main() -> Result<(), RuntimeError> {
    let config = Config::from_env()?;
    logging::init("info")?;

    if migrate_only_requested() {
        return migrate_owner_database(
            &config,
            "gateway",
            "gateway",
            &quantrule_gateway::migrations::gateway(),
        )
        .await;
    }

    tracing::info!("starting gateway");

    let mut health = HealthRegistry::named("gateway");
    let profiles: Option<Arc<dyn ProfileStore>> = open_service_database(&config, &mut health)
        .await?
        .map(|database| {
            Arc::new(PgProfileStore::new(database.pool().clone())) as Arc<dyn ProfileStore>
        });

    let data_upstream = HttpServiceClient::from_env(Service::Data)
        .map_err(|error| service_config_error(Service::Data, error))?;
    let data: Arc<dyn DataClient> = Arc::new(data_upstream.clone());
    let data_upstream = Arc::new(data_upstream);
    let model_upstream = HttpServiceClient::from_env(Service::Model)
        .map_err(|error| service_config_error(Service::Model, error))?;
    let evidence_upstream = Arc::new(
        HttpServiceClient::from_env(Service::Evidence)
            .map_err(|error| service_config_error(Service::Evidence, error))?,
    );
    let simulation_upstream = Arc::new(
        HttpServiceClient::from_env(Service::Simulation)
            .map_err(|error| service_config_error(Service::Simulation, error))?,
    );
    // The strategy service is the seventh unit. A v2 profile does not deploy it
    // and leaves the URL absent, so the gateway starts and skips its probe.
    let strategy_upstream = match HttpServiceClient::from_env(Service::Strategy) {
        Ok(client) => Some(Arc::new(client)),
        Err(ClientError::Unconfigured { .. }) => None,
        Err(error) => return Err(service_config_error(Service::Strategy, error)),
    };

    let state = Arc::new(AppState::with_services(
        &config,
        health,
        Services {
            profiles,
            data: Some(data),
            upstream: Some(data_upstream),
            model_upstream: Some(Arc::new(model_upstream)),
            evidence_upstream: Some(evidence_upstream.clone()),
            simulation_upstream: Some(simulation_upstream.clone()),
            strategy_upstream,
            runs: Some(evidence_upstream),
            simulations: Some(simulation_upstream),
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
