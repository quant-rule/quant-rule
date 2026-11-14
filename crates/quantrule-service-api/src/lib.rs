//! The `quantrule.internal/v1` contract between services.
//!
//! This crate defines what one service sends another: the run lifecycle, the
//! config bundle, the dataset object reference, the model bundle manifest, and
//! the canonical digest over a run's artifacts. It carries references, never a
//! candle, a reading, or a payment payload.
//!
//! The crate depends on [`quantrule_api`] for the shared digest and run
//! vocabulary and on no engine, no runtime, and no service crate. A service
//! may decode these types without linking the server that produced them.

#![forbid(unsafe_code)]

pub mod manifest;
pub mod run;
pub mod storage;

pub use manifest::{
    ManifestArtifact, ManifestError, RunManifest, SCHEMA as RUN_MANIFEST_SCHEMA, canonical_digest,
};
pub use run::{
    NewRun, RunCancelOutcome, RunClaimOutcome, RunClaimRequest, RunComputed, RunFailed, RunRecord,
};
pub use storage::{ConfigBundle, DatasetObject, MODEL_BUNDLE_SCHEMA, ModelBundleManifest};

/// The internal contract version carried by every service call.
pub const INTERNAL_API_VERSION: &str = "quantrule.internal/v1";
