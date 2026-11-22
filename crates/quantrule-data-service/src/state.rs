//! What the data routes need.
//!
//! The service state is narrower than the gateway's: a catalog, a config
//! catalog, the object store, the read-only config mount, and the upload cap.
//! It holds no profile store and no run store, so the data service cannot read
//! another service's schema.

use std::path::PathBuf;
use std::sync::Arc;

use quantrule_api::VersionInfo;
use quantrule_runtime::Config;
use quantrule_runtime::adapters::ObjectStore;
use quantrule_runtime::health::HealthRegistry;

use crate::artifacts::ArtifactCatalog;
use crate::catalog::DatasetCatalog;
use crate::configs::ConfigCatalog;
use crate::packs::PackRegistry;

/// The backing services the data routes call.
#[derive(Default)]
pub struct DataServices {
    /// Where datasets are catalogued.
    pub catalog: Option<Arc<dyn DatasetCatalog>>,
    /// Where run configs are catalogued.
    pub configs: Option<Arc<dyn ConfigCatalog>>,
    /// Where research packs are catalogued.
    pub packs: Option<Arc<dyn PackRegistry>>,
    /// Where computed research artifacts are catalogued.
    pub artifacts: Option<Arc<dyn ArtifactCatalog>>,
    /// Where dataset bytes live.
    pub objects: Option<Arc<dyn ObjectStore>>,
}

/// What a data handler needs.
pub struct DataState {
    /// Product and contract versions.
    pub version: VersionInfo,
    /// The dependency probes.
    pub health: HealthRegistry,
    /// Where datasets are catalogued; absent when no database is configured.
    pub catalog: Option<Arc<dyn DatasetCatalog>>,
    /// Where run configs are catalogued; absent when no database is configured.
    pub configs: Option<Arc<dyn ConfigCatalog>>,
    /// Where research packs are catalogued; absent when no database is configured.
    pub packs: Option<Arc<dyn PackRegistry>>,
    /// Where computed research artifacts are catalogued; absent when no
    /// database is configured.
    pub artifacts: Option<Arc<dyn ArtifactCatalog>>,
    /// Where dataset bytes live; absent when no object store is configured.
    pub objects: Option<Arc<dyn ObjectStore>>,
    /// Where the read-only run-config mount lives.
    pub config_root: PathBuf,
    /// The largest upload the service accepts, in bytes.
    pub max_upload: u64,
    /// Serialises dataset insert, archive, and erase inside this process.
    ///
    /// The catalog row and the object store cannot share a transaction. Across
    /// replicas the catalog's object guard orders the upload and erase; this
    /// gate is a local fast path and is never the cross-replica authority.
    pub dataset_gate: Arc<tokio::sync::Mutex<()>>,
    /// Serialises pack registration and lifecycle inside this process.
    ///
    /// Across replicas the unique pack and label keys are the authority; this
    /// gate only keeps one process's object and row writes in a local order.
    pub pack_gate: Arc<tokio::sync::Mutex<()>>,
    /// Serialises artifact registration, reference changes, and collection
    /// inside this process. The artifact row locks are the cross-replica
    /// authority for references.
    pub artifact_gate: Arc<tokio::sync::Mutex<()>>,
}

impl std::fmt::Debug for DataState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DataState")
            .field("version", &self.version)
            .field("health", &self.health)
            .field("catalog", &self.catalog.is_some())
            .field("configs", &self.configs.is_some())
            .field("packs", &self.packs.is_some())
            .field("artifacts", &self.artifacts.is_some())
            .field("objects", &self.objects.is_some())
            .field("config_root", &self.config_root)
            .field("max_upload", &self.max_upload)
            .field("dataset_gate", &Arc::strong_count(&self.dataset_gate))
            .field("pack_gate", &Arc::strong_count(&self.pack_gate))
            .field("artifact_gate", &Arc::strong_count(&self.artifact_gate))
            .finish()
    }
}

impl DataState {
    /// Build the state with whatever backing services are configured.
    #[must_use]
    pub fn with_services(config: &Config, health: HealthRegistry, services: DataServices) -> Self {
        Self {
            version: VersionInfo::current(env!("CARGO_PKG_VERSION"), config.image_digest.clone()),
            health,
            catalog: services.catalog,
            configs: services.configs,
            packs: services.packs,
            artifacts: services.artifacts,
            objects: services.objects,
            config_root: config.config_root.clone(),
            max_upload: config.upload_limit_bytes,
            dataset_gate: Arc::new(tokio::sync::Mutex::new(())),
            pack_gate: Arc::new(tokio::sync::Mutex::new(())),
            artifact_gate: Arc::new(tokio::sync::Mutex::new(())),
        }
    }
}
