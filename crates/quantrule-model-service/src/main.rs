//! The model service binary.
//!
//! It connects to the object store, reads the catalog references through the data
//! service, and serves the public model compute and the internal model bundle.
//! It applies no migration and owns no database schema.

#![forbid(unsafe_code)]

use std::sync::Arc;

use quantrule_runtime::adapters::ObjectStore;
use quantrule_runtime::backends::Objects;
use quantrule_runtime::client::{ClientError, DataClient, HttpServiceClient, Service};
use quantrule_runtime::config::ConfigError;
use quantrule_runtime::metrics::{self, Metrics};
use quantrule_runtime::request_id::RequestIds;
use quantrule_runtime::{
    Config, HealthRegistry, RuntimeError, logging, observability, serve_config,
};

use quantrule_model_service::{ModelServices, ModelState};

#[tokio::main]
async fn main() -> Result<(), RuntimeError> {
    let config = Config::from_env()?;
    logging::init("model", config.image_digest.as_ref())?;

    let metrics = Metrics::new(
        "model",
        env!("CARGO_PKG_VERSION"),
        config.image_digest.as_ref().map(ToString::to_string),
    );
    metrics::start(&config, &metrics).await?;
    let mut health = HealthRegistry::named("model").with_metrics(metrics.clone());
    let backend = Objects::open(&config).await?;
    let objects: Arc<dyn ObjectStore> = Arc::new(backend.clone());
    health.register("object-store", move || {
        let backend = backend.clone();
        async move { backend.check().await.map_err(|error| error.to_string()) }
    });

    let data: Arc<dyn DataClient> =
        Arc::new(HttpServiceClient::from_env(Service::Data).map_err(data_config_error)?);
    let state = Arc::new(ModelState::with_services(
        &config,
        health,
        ModelServices {
            data: Some(data),
            objects: Some(objects),
        },
    ));
    tracing::info!(bind = %config.bind, "starting model service");
    serve_config(
        &config,
        observability::instrument(
            quantrule_model_service::router(state),
            metrics,
            RequestIds::from_env("model"),
        ),
    )
    .await
}

/// Name the missing or invalid `QR_DATA_URL` at startup.
fn data_config_error(error: ClientError) -> RuntimeError {
    let name = Service::Data.url_variable();
    RuntimeError::Config(match error {
        ClientError::Unconfigured { .. } => ConfigError::Missing { name },
        _ => ConfigError::Invalid {
            name,
            value: "<invalid>".to_owned(),
            reason: "must be an absolute HTTP URL",
        },
    })
}
