//! The `quantrule.internal/v1` contract between services.
//!
//! This crate defines what one service sends another: the run lifecycle, the
//! config bundle, the dataset object reference, the model bundle manifest, the
//! signal artifact registration, the diagnostics stage lifecycle and manifest,
//! and the canonical digest over a run's artifacts. The signal request and
//! response are shared with the clients through [`quantrule_api`] and re-exported
//! here. The crate carries references, never a candle, a reading, or a payment
//! payload.
//!
//! The crate depends on [`quantrule_api`] for the shared digest and run
//! vocabulary and on no engine, no runtime, and no service crate. A service
//! may decode these types without linking the server that produced them.

#![forbid(unsafe_code)]

pub mod artifact;
pub mod manifest;
pub mod run;
pub mod signal;
pub mod stage;
pub mod storage;

pub use artifact::{ArtifactRegistration, ArtifactRegistrationRequest};
pub use manifest::{
    ManifestArtifact, ManifestError, RunManifest, SCHEMA as RUN_MANIFEST_SCHEMA, canonical_digest,
};
pub use run::{
    MAX_ATTEMPTS, NewRun, RUN_LEASE_RENEW_SECONDS, RUN_LEASE_TTL_SECONDS, RunCancelOutcome,
    RunClaimOutcome, RunClaimRequest, RunComputed, RunFailed, RunHeartbeat, RunRecord,
    derive_run_id, derive_run_id_with_model,
};
pub use signal::{SIGNAL_REQUEST_SCHEMA, SIGNAL_RESPONSE_SCHEMA, SignalRequest, SignalResponse};
pub use stage::{
    NewStageRun, STAGE_MANIFEST_SCHEMA, StageCancelOutcome, StageClaim, StageClaimOutcome,
    StageComputed, StageFailed, StageManifest, StageManifestError, StageRecord,
    derive_stage_run_id,
};
pub use storage::{ConfigBundle, DatasetObject, MODEL_BUNDLE_SCHEMA, ModelBundleManifest};

/// The internal contract version carried by every service call.
pub const INTERNAL_API_VERSION: &str = "quantrule.internal/v1";
