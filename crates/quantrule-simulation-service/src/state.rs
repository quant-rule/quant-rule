//! What the simulation routes need.
//!
//! The state holds one repository, the object store the recorded bundles live
//! in, and the version and health values. It holds no queue, no run store, no
//! profile store, and no model client, so the research plane cannot publish a
//! control-plane message, write a run artifact, or compute a model.

use std::sync::Arc;

use quantrule_api::VersionInfo;
use quantrule_runtime::Config;
use quantrule_runtime::adapters::ObjectStore;
use quantrule_runtime::health::HealthRegistry;

use crate::store::SimulationStore;

/// The backing services the simulation routes call.
#[derive(Default)]
pub struct SimulationServices {
    /// Where runs and marks are recorded.
    pub store: Option<Arc<dyn SimulationStore>>,
    /// Where recorded bundles and the CSV mirror live.
    pub objects: Option<Arc<dyn ObjectStore>>,
}

/// What a simulation handler needs.
pub struct SimulationState {
    /// Product and contract versions.
    pub version: VersionInfo,
    /// The dependency probes.
    pub health: HealthRegistry,
    /// The research repository.
    pub store: Option<Arc<dyn SimulationStore>>,
    /// The object store.
    pub objects: Option<Arc<dyn ObjectStore>>,
}

impl std::fmt::Debug for SimulationState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SimulationState")
            .field("version", &self.version)
            .field("health", &self.health)
            .field("store", &self.store.is_some())
            .field("objects", &self.objects.is_some())
            .finish()
    }
}

impl SimulationState {
    /// Build the state with whatever backing services are configured.
    #[must_use]
    pub fn with_services(
        config: &Config,
        health: HealthRegistry,
        services: SimulationServices,
    ) -> Self {
        Self {
            version: VersionInfo::current(env!("CARGO_PKG_VERSION"), config.image_digest.clone()),
            health,
            store: services.store,
            objects: services.objects,
        }
    }
}
