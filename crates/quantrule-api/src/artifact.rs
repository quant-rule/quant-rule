//! Computed research artifacts and their retention seam.
//!
//! A signal artifact is a computed, content-addressed output of a model
//! artifact and a signal-mode strategy pack. The strategy service writes the
//! bytes and registers the row; the catalog owns the artifact identity,
//! consumer references, and the retention tombstone.
//!
//! An artifact row is profile-scoped; the bytes are shared by content key. A
//! referenced artifact is never collected, and an unreferenced one is collected
//! only after the retention window, with an append-only tombstone.

use serde::{Deserialize, Serialize};

use crate::catalog::{
    DatasetReferenceCount, DatasetReferenceKind, DatasetReferenceRequest,
    ReleaseDatasetReferenceRequest,
};
use crate::digest::Digest;

/// The consumer-kind vocabulary shared with dataset references.
pub type ArtifactReferenceKind = DatasetReferenceKind;

/// The kind of computed artifact the catalog stores.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    /// A computed signal artifact.
    Signal,
}

impl ArtifactKind {
    /// The stable wire and storage token.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Signal => "signal",
        }
    }

    /// Parses the stable wire code.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "signal" => Some(Self::Signal),
            _ => None,
        }
    }
}

/// An artifact in a list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactSummary {
    /// The SHA-256 of the stored bytes.
    pub artifact_id: Digest,
    /// The artifact kind.
    pub kind: ArtifactKind,
    /// The stored byte count.
    pub bytes: u64,
    /// When the row was created, RFC 3339 UTC.
    pub created_at: String,
}

/// An artifact record, including where its bytes live.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactRecord {
    /// The list projection.
    pub summary: ArtifactSummary,
    /// The object key inside the content-addressed store.
    pub object_key: String,
    /// The hash of the stored bytes.
    pub sha256: Digest,
}

/// One registered consumer reference to an artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactReference {
    /// Stable identity of the consumer record, such as `run:<id>`.
    pub reference_id: String,
    /// What kind of record holds the reference.
    pub kind: ArtifactReferenceKind,
    /// When the reference was registered, RFC 3339 UTC.
    pub created_at: String,
}

/// A request to register or release an artifact reference.
pub type ArtifactReferenceRequest = DatasetReferenceRequest;

/// A request to release one registered artifact reference.
pub type ReleaseArtifactReferenceRequest = ReleaseDatasetReferenceRequest;

/// The number of references an artifact still has.
pub type ArtifactReferenceCount = DatasetReferenceCount;

/// One artifact collected by a retention pass.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectedArtifact {
    /// The collected artifact.
    pub artifact_id: Digest,
    /// The object key the caller may delete.
    pub object_key: String,
    /// The byte count the object occupied.
    pub bytes: u64,
    /// Whether another profile still points at the same object key.
    pub object_shared: bool,
}

/// The result of one retention pass.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CollectArtifactsResponse {
    /// Artifacts whose rows were removed.
    pub collected: u64,
    /// Bytes removed from the object store.
    pub bytes: u64,
    /// Object keys the caller deleted, in artifact-id order.
    pub objects: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_artifact_kind_round_trips_its_wire_code() {
        assert_eq!(
            ArtifactKind::from_code("signal"),
            Some(ArtifactKind::Signal)
        );
        assert_eq!(ArtifactKind::from_code("model"), None);
        let text = serde_json::to_string(&ArtifactKind::Signal).expect("encode");
        assert_eq!(text, "\"signal\"");
    }
}
