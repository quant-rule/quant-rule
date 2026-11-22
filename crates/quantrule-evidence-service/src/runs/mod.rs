//! The run store and identity.
//!
//! A run is content-addressed and shared. A submission row binds a profile to a
//! run, so two profiles can point at one run without duplicating it. The run
//! identity and retry budget are part of the `quantrule.internal/v1` contract,
//! so the worker derives the same values evidence stores.
//!
//! The store also carries the two records that make a cross-database step safe:
//! the notification a live run needs, and the dataset-reference release a
//! deleted consumer promised. Both commit with the run write and are retried by
//! the reconciler.

mod memory;
pub mod store;

pub use memory::MemoryRuns;
pub use quantrule_service_api::{
    MAX_ATTEMPTS, derive_run_id as derive, derive_run_id_with_model as derive_with_model,
};
pub use store::{
    CancelOutcome, ClaimOutcome, DatasetReferenceRelease, HeartbeatOutcome, NewRun,
    NotificationKind, RunArchiveOutcome, RunDatasetReference, RunEraseOutcome, RunEraseReceipt,
    RunNotification, RunRecord, RunStore, RunStoreError, SubmitOutcome,
};
