//! The research ledger: pre-registration, pipeline link, decision, baselines.
//!
//! The v3 manifest is written once. A pipeline run and its canonical digest are
//! linked once, and the human decision is recorded once. The committed
//! `docs/improvement/` registry remains the human-owned snapshot; this store is
//! the evidence-service record behind the later research commands. A stage
//! artifact is not a pipeline run and is not accepted by the link method.

pub mod memory;
pub mod postgres;
pub mod sqlite;
pub mod store;

pub use memory::{MemoryImprovementStore, seeded_baselines};
pub use postgres::PgImprovementStore;
pub use sqlite::SqliteImprovementStore;
pub use store::{CreateOutcome, ImprovementStore, ImprovementStoreError};
