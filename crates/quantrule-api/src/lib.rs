//! The `quantrule.api/v1` contract types.
//!
//! This crate is the single definition of what crosses a process boundary: the
//! run lifecycle, the catalog records, the profile header, the simulation
//! surface, the health report, and the error envelope. The gateway and the
//! clients depend on it; it depends on no engine crate.
//!
//! Types here are transport-free. HTTP status mapping is a method that returns a
//! number, not an adapter, so the crate stays usable from a test, a CLI, or a
//! server without pulling a runtime in.

#![forbid(unsafe_code)]

pub mod catalog;
pub mod digest;
pub mod error;
pub mod health;
pub mod profile;
pub mod run;
pub mod sha256;
pub mod simulation;
pub mod version;

pub use catalog::{
    ConfigOrigin, ConfigSummary, DatasetRecord, DatasetSummary, QualityReport, SkippedRow,
    TimeframeBars, TimeframeInspection, UploadConfigRequest,
};
pub use digest::{Digest, DigestError};
pub use error::{ApiError, ErrorCode, ErrorEnvelope};
pub use health::{HealthReport, ServiceHealth, ServiceState, VersionInfo};
pub use profile::{
    CreateProfileRequest, DEFAULT_PROFILE, PROFILE_HEADER, ProfileId, ProfileSummary,
};
pub use run::{
    ArtifactRef, Provenance, RunDetail, RunId, RunState, RunSummary, StageName, StageProgress,
    StageStatus, SubmitRunRequest, SubmitRunResponse,
};
pub use sha256::{sha256, sha256_hex};
pub use simulation::{
    CreateSimulationRequest, Mark, MarkAction, MarkOrigin, MarkQuality, Side, SimulationId,
    SimulationStatus, SimulationSummary,
};
pub use version::{
    API_VERSION, BUNDLE_CONTRACT, DATASET_CONTRACT, DEPLOY_CONTRACT, LABELS_CONTRACT,
    QUEUE_CONTRACT, RUN_CONTRACT,
};
