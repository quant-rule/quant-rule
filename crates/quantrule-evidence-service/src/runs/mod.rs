//! The run store and identity.
//!
//! A run is content-addressed and shared. A submission row binds a profile to a
//! run, so two profiles can point at one run without duplicating it. The run
//! identity and retry budget are part of the `quantrule.internal/v1` contract,
//! so the worker derives the same values evidence stores.

pub mod store;

pub use quantrule_service_api::{
    MAX_ATTEMPTS, derive_run_id as derive, derive_run_id_with_model as derive_with_model,
};
pub use store::{
    CancelOutcome, ClaimOutcome, MemoryRuns, NewRun, RunArchiveOutcome, RunDatasetReference,
    RunEraseOutcome, RunEraseReceipt, RunRecord, RunStore, RunStoreError, SubmitOutcome,
};
