//! The model service binary.
//!
//! It connects to the object store, reads the catalog references through the data
//! service, and serves the public model compute and the internal model bundle.
//! It applies no migration and owns no database schema.

#![forbid(unsafe_code)]

use std::sync::Arc;

use quantrule_runtime::adapters::ObjectStore;
use quantrule_runtime::adapters::s3::S3Store;
use quantrule_runtime::client::{ClientError, DataClient, HttpServiceClient, Service};
use quantrule_runtime::config::ConfigError;
use quantrule_runtime::{Config, HealthRegistry, RuntimeError, logging, serve};

use quantrule_model_service::{ModelServices, ModelState};

#[tokio::main]
async fn main() -> Result<(), RuntimeError> {
    let config = Config::from_env()?;
    logging::init("info")?;

    let mut health = HealthRegistry::new();
    let s3 = S3Store::new(&config.object_store_url, &config.object_store_bucket).await;
    let objects: Arc<dyn ObjectStore> = Arc::new(s3.clone());
    health.register("object-store", move || {
        let store = s3.clone();
        async move { store.check().await.map_err(|error| error.to_string()) }
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
    serve(&config.bind, quantrule_model_service::router(state)).await
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
