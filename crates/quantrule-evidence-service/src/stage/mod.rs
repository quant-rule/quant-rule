//! The diagnostic stage store.
//!
//! A stage run executes one engine outside the coupled run. Its record lives in
//! the `stage` schema, is content-addressed by the stage identity, and is bound
//! to a profile through a submission row, so two profiles can point at one
//! stage without duplicating it. Artifacts are content-addressed files.
//!
//! The store is deliberately separate from [`crate::runs`]: no stage row is
//! visible through the run store, no stage record carries a canonical digest,
//! and the stage schema is never read by evidence finalization. A stage is
//! diagnostic, and the coupled run remains the only evidence path.
//!
//! The retry budget is the control plane's [`MAX_ATTEMPTS`]: a stage and a run
//! fail over the same three deliveries, so a client does not have to learn a
//! second retry rule.

pub mod memory;
pub mod postgres;
pub mod sqlite;
pub mod store;

pub use memory::MemoryStages;
pub use postgres::PgStageStore;
pub use quantrule_service_api::{
    MAX_ATTEMPTS, NewStageRun, StageCancelOutcome, StageClaimOutcome, StageRecord,
};
pub use sqlite::SqliteStageStore;
pub use store::{StageCreateOutcome, StageStore, StageStoreError};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_store_types_are_distinct_from_the_run_store() {
        // A compile-time proof that the two record types are not
        // interchangeable: each function accepts only its own record.
        fn stage_only(_: &StageRecord) {}
        fn run_only(_: &crate::runs::RunRecord) {}
        let _ = stage_only;
        let _ = run_only;
        assert_ne!(
            std::any::type_name::<StageRecord>(),
            std::any::type_name::<crate::runs::RunRecord>()
        );
    }
}
