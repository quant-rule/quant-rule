//! What the evidence routes need.
//!
//! The service state is narrower than the old gateway's: a run store, a
//! hypothesis store, the object store, the control-plane queue, and the data
//! client used to validate a submission. It holds no profile store, so evidence
//! cannot read the `gateway` schema.

use std::sync::Arc;

use quantrule_api::VersionInfo;
use quantrule_runtime::Config;
use quantrule_runtime::adapters::{ObjectStore, Queue};
use quantrule_runtime::client::DataClient;
use quantrule_runtime::health::HealthRegistry;

use crate::operations::OperationsStatus;
use crate::research::{HypothesisStore, ImprovementStore};
use crate::runs::RunStore;
use crate::stage::StageStore;

/// The backing services the evidence routes call.
#[derive(Default)]
pub struct EvidenceServices {
    /// Where runs are recorded.
    pub runs: Option<Arc<dyn RunStore>>,
    /// Where diagnostic stage runs are recorded.
    pub stages: Option<Arc<dyn StageStore>>,
    /// Where sealed hypotheses live.
    pub hypotheses: Option<Arc<dyn HypothesisStore>>,
    /// Where improvement runs, baselines, and invalidations live.
    pub improvements: Option<Arc<dyn ImprovementStore>>,
    /// Where artifact bytes live.
    pub objects: Option<Arc<dyn ObjectStore>>,
    /// The control-plane queue; evidence publishes every run message.
    pub queue: Option<Arc<dyn Queue>>,
    /// The data service's typed reads, for submission validation.
    pub data: Option<Arc<dyn DataClient>>,
}

/// What an evidence handler needs.
pub struct EvidenceState {
    /// Product and contract versions.
    pub version: VersionInfo,
    /// The dependency probes.
    pub health: HealthRegistry,
    /// Where runs are recorded; absent when no database is configured.
    pub runs: Option<Arc<dyn RunStore>>,
    /// Where diagnostic stage runs are recorded; absent when no database is
    /// configured.
    pub stages: Option<Arc<dyn StageStore>>,
    /// Where sealed hypotheses live; absent when no database is configured.
    pub hypotheses: Option<Arc<dyn HypothesisStore>>,
    /// Where improvement runs, baselines, and invalidations live; absent when
    /// no database is configured.
    pub improvements: Option<Arc<dyn ImprovementStore>>,
    /// Where artifact bytes live; absent when no object store is configured.
    pub objects: Option<Arc<dyn ObjectStore>>,
    /// The control-plane queue; absent when the broker is unreachable.
    pub queue: Option<Arc<dyn Queue>>,
    /// The data service's typed reads; absent in a test that does not submit.
    pub data: Option<Arc<dyn DataClient>>,
    /// The configured retention window and the last cleanup.
    pub operations: Arc<OperationsStatus>,
}

impl std::fmt::Debug for EvidenceState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("EvidenceState")
            .field("version", &self.version)
            .field("health", &self.health)
            .field("runs", &self.runs.is_some())
            .field("stages", &self.stages.is_some())
            .field("hypotheses", &self.hypotheses.is_some())
            .field("improvements", &self.improvements.is_some())
            .field("objects", &self.objects.is_some())
            .field("queue", &self.queue.is_some())
            .field("data", &self.data.is_some())
            .finish()
    }
}

impl EvidenceState {
    /// Build the state with whatever backing services are configured.
    #[must_use]
    pub fn with_services(
        config: &Config,
        health: HealthRegistry,
        services: EvidenceServices,
    ) -> Self {
        let stage_store = services.stages.is_some();
        Self {
            version: VersionInfo::current(env!("CARGO_PKG_VERSION"), config.image_digest.clone()),
            health,
            runs: services.runs,
            stages: services.stages,
            hypotheses: services.hypotheses,
            improvements: services.improvements,
            objects: services.objects,
            queue: services.queue,
            data: services.data,
            operations: OperationsStatus::new(config.stage_retention_days, stage_store),
        }
    }
}
