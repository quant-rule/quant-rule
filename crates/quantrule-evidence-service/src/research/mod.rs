//! The research stores.
//!
//! A sealed hypothesis is written once and read by fingerprint. The improvement
//! ledger holds the v3 pre-registration, its one-time pipeline link, and its
//! one-time human decision, with append-only baseline and invalidation
//! registries. Both are traits so the routes are testable without a database and
//! so the embedded desktop profile can supply their own implementations.

pub mod ledger;
pub mod store;

pub use ledger::{
    CreateOutcome, DecisionWrite, ImprovementStore, ImprovementStoreError, MemoryImprovementStore,
    PgImprovementStore, SqliteImprovementStore, seeded_baselines,
};
pub use store::{HypothesisStore, MemoryStore, StoreError};
