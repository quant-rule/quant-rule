//! The research store.
//!
//! A sealed hypothesis is written once and read by fingerprint. The store is a
//! trait so the route is testable without a database, and so the embedded
//! desktop profile can supply its own implementation.

pub mod store;

pub use store::{HypothesisStore, MemoryStore, StoreError};
