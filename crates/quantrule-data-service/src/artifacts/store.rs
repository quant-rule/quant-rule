//! Artifact storage behind one trait.

use quantrule_api::{
    ArtifactKind, ArtifactRecord, ArtifactReference, CollectedArtifact, Digest, ProfileId,
};
use quantrule_runtime::adapters::BoxFuture;

/// An artifact where the catalog stores it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewArtifact {
    /// The SHA-256 of the stored bytes.
    pub artifact_id: Digest,
    /// The artifact kind.
    pub kind: ArtifactKind,
    /// The object key the bytes are stored under.
    pub object_key: String,
    /// The stored byte count.
    pub bytes: u64,
    /// The creation time, RFC 3339 UTC.
    pub created_at: String,
}

/// Why an artifact operation failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactError {
    /// The artifact is unknown in this profile.
    NotFound,
    /// A consumer reference still protects this artifact.
    Referenced(u64),
    /// The store refused the operation.
    Store(String),
}

impl std::fmt::Display for ArtifactError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => formatter.write_str("the artifact is unknown"),
            Self::Referenced(count) => {
                write!(formatter, "the artifact still has {count} reference(s)")
            }
            Self::Store(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for ArtifactError {}

/// Where computed research artifacts live.
pub trait ArtifactCatalog: Send + Sync {
    /// Register one artifact. The same content is idempotent.
    fn register(
        &self,
        profile: ProfileId,
        artifact: NewArtifact,
    ) -> BoxFuture<Result<ArtifactRecord, ArtifactError>>;

    /// Read one artifact by content id within a profile.
    fn get(
        &self,
        profile: ProfileId,
        kind: ArtifactKind,
        artifact_id: String,
    ) -> BoxFuture<Result<Option<ArtifactRecord>, ArtifactError>>;

    /// List a profile's artifacts of one kind, ordered by id.
    fn list(
        &self,
        profile: ProfileId,
        kind: ArtifactKind,
    ) -> BoxFuture<Result<Vec<ArtifactRecord>, ArtifactError>>;

    /// Register one consumer reference. Repeated registration is idempotent.
    fn add_reference(
        &self,
        profile: ProfileId,
        kind: ArtifactKind,
        artifact_id: String,
        reference: ArtifactReference,
    ) -> BoxFuture<Result<u64, ArtifactError>>;

    /// Release one consumer reference. Returns the remaining count.
    fn release_reference(
        &self,
        profile: ProfileId,
        kind: ArtifactKind,
        artifact_id: String,
        reference_id: String,
    ) -> BoxFuture<Result<u64, ArtifactError>>;

    /// List every consumer reference to one artifact.
    fn references(
        &self,
        profile: ProfileId,
        kind: ArtifactKind,
        artifact_id: String,
    ) -> BoxFuture<Result<Vec<ArtifactReference>, ArtifactError>>;

    /// Collect unreferenced artifacts created at or before `cutoff`.
    ///
    /// The row delete and its retention tombstone commit together. The caller
    /// deletes each unshared object after the commit.
    fn collect(
        &self,
        profile: ProfileId,
        kind: ArtifactKind,
        cutoff: String,
        collected_at: String,
    ) -> BoxFuture<Result<Vec<CollectedArtifact>, ArtifactError>>;
}
