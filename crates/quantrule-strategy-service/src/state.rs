//! What the strategy routes need.
//!
//! The state is narrower than a run service's: a pack reader, a model bundle
//! resolver, the artifact registrar and the object store. It holds no database,
//! queue, profile store or run store.

use std::sync::Arc;

use quantrule_api::VersionInfo;
use quantrule_runtime::Config;
use quantrule_runtime::adapters::ObjectStore;
use quantrule_runtime::client::{ArtifactClient, ModelClient, PackClient};
use quantrule_runtime::health::HealthRegistry;

/// The backing services the strategy routes call.
#[derive(Default)]
pub struct StrategyServices {
    /// The data service's pack reads.
    pub packs: Option<Arc<dyn PackClient>>,
    /// The model service's frozen bundle reference.
    pub model: Option<Arc<dyn ModelClient>>,
    /// The data service's metadata-only artifact registration.
    pub artifacts: Option<Arc<dyn ArtifactClient>>,
    /// Where signal artifacts live.
    pub objects: Option<Arc<dyn ObjectStore>>,
}

/// What a strategy handler needs.
pub struct StrategyState {
    /// Product and contract versions.
    pub version: VersionInfo,
    /// The dependency probes.
    pub health: HealthRegistry,
    /// The data service's pack reads; absent in a test that does not use them.
    pub packs: Option<Arc<dyn PackClient>>,
    /// The model service's bundle reference; absent in a test that does not use it.
    pub model: Option<Arc<dyn ModelClient>>,
    /// The data service's artifact registration; absent in a test that does not use it.
    pub artifacts: Option<Arc<dyn ArtifactClient>>,
    /// Where signal bytes live; absent when no object store is configured.
    pub objects: Option<Arc<dyn ObjectStore>>,
}

impl std::fmt::Debug for StrategyState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StrategyState")
            .field("version", &self.version)
            .field("health", &self.health)
            .field("packs", &self.packs.is_some())
            .field("model", &self.model.is_some())
            .field("artifacts", &self.artifacts.is_some())
            .field("objects", &self.objects.is_some())
            .finish()
    }
}

impl StrategyState {
    /// Build the state with whatever backing services are configured.
    #[must_use]
    pub fn with_services(
        config: &Config,
        health: HealthRegistry,
        services: StrategyServices,
    ) -> Self {
        Self {
            version: VersionInfo::current(env!("CARGO_PKG_VERSION"), config.image_digest.clone()),
            health,
            packs: services.packs,
            model: services.model,
            artifacts: services.artifacts,
            objects: services.objects,
        }
    }
}
