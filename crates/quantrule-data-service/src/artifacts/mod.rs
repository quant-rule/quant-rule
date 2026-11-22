//! Computed artifact lineage, references and retention.
//!
//! A computed artifact is content-addressed and profile-scoped; the bytes live
//! in the object store under the same content key. The strategy service writes
//! the bytes and registers the row through the metadata-only route; this module
//! owns registration by content id, consumer references, and the retention pass
//! that removes only unreferenced artifacts after the window, writing a
//! tombstone in the same transaction.
//!
//! A referenced artifact is never collected. A leaked object is safe; a missing
//! referenced object is not, so the row and tombstone commit first and the
//! caller deletes an unshared object afterwards.

pub mod memory;
pub mod postgres;
pub mod routes;
pub mod store;

pub use memory::MemoryArtifacts;
pub use postgres::PgArtifacts;
pub use routes::routes;
pub use store::{ArtifactCatalog, ArtifactError, ArtifactRegisterOutcome, NewArtifact};

/// How long an unreferenced computed artifact is retained, in days.
///
/// The window is fixed in this slice. A later configuration change can make it
/// operator-tunable without changing the pass contract.
pub const ARTIFACT_RETENTION_DAYS: u64 = 30;
