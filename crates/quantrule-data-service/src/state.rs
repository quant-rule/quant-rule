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
    /// Serialises dataset insert, archive, and erase with object deletion.
    ///
    /// The catalog row and the object store cannot share a transaction. The
    /// gate makes the check-then-delete window single-writer inside this
    /// service, so a concurrent upload cannot observe an object that an erase
    /// is about to remove.
    pub dataset_gate: Arc<tokio::sync::Mutex<()>>,
    /// Serialises pack registration and lifecycle with its object write.
    ///
    /// The registry row and the object store cannot share a transaction; the
    /// gate makes the check-then-write window single-writer inside this
    /// service.
    pub pack_gate: Arc<tokio::sync::Mutex<()>>,
    /// Serialises artifact registration, reference changes, and collection
    /// with object deletion.
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
