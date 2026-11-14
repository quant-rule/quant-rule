//! What the model routes need.
//!
//! The service state is narrower than the gateway's: the data client, the object
//! store, and the version and health values. It holds no profile store and no run
//! store, so the model service cannot read another service's schema.

use std::sync::Arc;

use quantrule_api::VersionInfo;
use quantrule_runtime::Config;
use quantrule_runtime::adapters::ObjectStore;
use quantrule_runtime::client::DataClient;
use quantrule_runtime::health::HealthRegistry;

/// The backing services the model routes call.
#[derive(Default)]
pub struct ModelServices {
    /// The data service's typed reads: the config bundle and the dataset object.
    pub data: Option<Arc<dyn DataClient>>,
    /// Where model objects and model-bundle artifacts live.
    pub objects: Option<Arc<dyn ObjectStore>>,
}

/// What a model handler needs.
pub struct ModelState {
    /// Product and contract versions.
    pub version: VersionInfo,
    /// The dependency probes.
    pub health: HealthRegistry,
    /// The data service's typed reads; absent in a test that does not use them.
    pub data: Option<Arc<dyn DataClient>>,
    /// Where canonical bytes live; absent when no object store is configured.
    pub objects: Option<Arc<dyn ObjectStore>>,
}

impl std::fmt::Debug for ModelState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ModelState")
            .field("version", &self.version)
            .field("health", &self.health)
            .field("data", &self.data.is_some())
            .field("objects", &self.objects.is_some())
            .finish()
    }
}

impl ModelState {
    /// Build the state with whatever backing services are configured.
    #[must_use]
    pub fn with_services(config: &Config, health: HealthRegistry, services: ModelServices) -> Self {
        Self {
            version: VersionInfo::current(env!("CARGO_PKG_VERSION"), config.image_digest.clone()),
            health,
            data: services.data,
            objects: services.objects,
        }
    }
}
