//! The run store and identity.
//!
//! A run is content-addressed and shared. A submission row binds a profile to a
//! run, so two profiles can point at one run without duplicating it.

pub mod identity;
pub mod store;

pub use identity::derive;
pub use store::{
    CancelOutcome, MemoryRuns, NewRun, RunRecord, RunStore, RunStoreError, SubmitOutcome,
};
