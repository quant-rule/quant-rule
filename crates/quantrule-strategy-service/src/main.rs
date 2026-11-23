//! The strategy service binary.
//!
//! It connects to the object store, reads packs and registers artifacts through
//! the data service, and asks the model service for a frozen bundle reference.
//! It applies no migration, owns no database, and declares no queue.

#![forbid(unsafe_code)]

use std::sync::Arc;

use quantrule_runtime::adapters::ObjectStore;
use quantrule_runtime::backends::Objects;
use quantrule_runtime::client::{
    ArtifactClient, ClientError, HttpServiceClient, ModelClient, PackClient, Service,
};
use quantrule_runtime::config::ConfigError;
use quantrule_runtime::metrics::{self, Metrics};
use quantrule_runtime::request_id::RequestIds;
use quantrule_runtime::{
    Config, Drain, HealthRegistry, RuntimeError, Shutdown, logging, observability, serve_config,
    watch_signal,
};
use quantrule_strategy_service::{StrategyServices, StrategyState};

#[tokio::main]
async fn main() -> Result<(), RuntimeError> {
    let config = Config::from_env()?;
    logging::init("strategy", config.image_digest.as_ref())?;

    let (trigger, shutdown) = Shutdown::channel();
    let metrics = Metrics::new(
        "strategy",
        env!("CARGO_PKG_VERSION"),
        config.image_digest.as_ref().map(ToString::to_string),
    );
    let drain = Drain::new(
        shutdown.clone(),
        Some(metrics.clone()),
        config.drain_timeout_http,
    );
    metrics::start(&config, &metrics).await?;
    let mut health = HealthRegistry::named("strategy")
        .with_metrics(metrics.clone())
        .with_shutdown(shutdown.clone());
    let backend = Objects::open(&config).await?;
    let objects: Arc<dyn ObjectStore> = Arc::new(backend.clone());
    health.register("object-store", move || {
        let backend = backend.clone();
        async move { backend.check().await.map_err(|error| error.to_string()) }
    });

    let data = HttpServiceClient::from_env(Service::Data)
        .map_err(data_config_error)?
        .with_retry(metrics.clone(), shutdown.clone());
    let model = HttpServiceClient::from_env(Service::Model)
        .map_err(data_config_error)?
        .with_retry(metrics.clone(), shutdown);
    let packs: Arc<dyn PackClient> = Arc::new(data.clone());
    let artifacts: Arc<dyn ArtifactClient> = Arc::new(data);
    let model: Arc<dyn ModelClient> = Arc::new(model);
    let state = Arc::new(StrategyState::with_services(
        &config,
        health,
        StrategyServices {
            packs: Some(packs),
            model: Some(model),
            artifacts: Some(artifacts),
            objects: Some(objects),
        },
    ));
    tracing::info!(bind = %config.bind, "starting strategy service");
    std::mem::drop(watch_signal(trigger));
    serve_config(
        &config,
        observability::instrument(
            quantrule_strategy_service::router(state),
            metrics,
            RequestIds::from_env("strategy"),
        ),
        &drain,
    )
    .await
}

/// Name the missing or invalid service URL at startup.
fn data_config_error(error: ClientError) -> RuntimeError {
    let service = match &error {
        ClientError::Unconfigured { service }
        | ClientError::Unreachable { service, .. }
        | ClientError::Timeout { service }
        | ClientError::Status { service, .. }
        | ClientError::InvalidResponse { service, .. }
        | ClientError::InvalidBase { service, .. }
        | ClientError::Interrupted { service } => *service,
    };
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
