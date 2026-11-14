//! Profiles.
//!
//! A profile is an organisational scope, not a security boundary. The store
//! records which scopes exist; every catalog row carries one, and the objects
//! themselves are shared.

pub mod routes;
pub mod store;

pub use routes::{create_profile, list_profiles};
pub use store::{MemoryProfiles, ProfileError, ProfileRecord, ProfileStore};
