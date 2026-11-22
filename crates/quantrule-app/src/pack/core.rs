//! The content identity and lifecycle shared by every pack kind.
//!
//! Every pack has the same envelope, the same canonical bytes and the same
//! lifecycle. Extracting that seam keeps a new pack kind from growing its own
//! hash or state machine, so a strategy pack and a portfolio policy pack are
//! named and frozen by exactly the same rules. Only the metadata, the canonical
//! bytes, the content hash and the lifecycle state live here; the kind-specific
//! payload stays with its parser.

use quantrule_api::Digest;
use serde_json::Value;

use super::canonical::canonical_json;
use super::envelope::PackMetadata;
use super::error::PackError;
use super::lifecycle::{PackState, PackStateError};

/// The envelope identity and lifecycle state of one pack.
#[derive(Debug, Clone)]
pub(crate) struct PackCore {
    metadata: PackMetadata,
    canonical: String,
    id: Digest,
    state: PackState,
}

impl PackCore {
    /// Identifies a freshly parsed pack.
    ///
    /// The canonical bytes are taken once, here, and stored. A later identity
    /// check hashes the stored bytes rather than re-serialising the document,
    /// so a serializer change cannot silently move every pack ID.
    pub(crate) fn new(value: &Value, metadata: &PackMetadata) -> Self {
        let canonical = canonical_json(value);
        let id = Digest::sha256(canonical.as_bytes());
        Self {
            metadata: metadata.clone(),
            canonical,
            id,
            state: PackState::Draft,
        }
    }

    /// Re-identifies a pack from the bytes that were actually stored.
    ///
    /// # Errors
    ///
    /// Returns [`PackError::NonCanonical`] when the bytes parse but differ from
    /// the canonical form of the document, so stored bytes cannot acquire a
    /// second identity through whitespace or key order.
    pub(crate) fn reidentify(&mut self, bytes: &[u8], file: &str) -> Result<(), PackError> {
        if self.canonical.as_bytes() != bytes {
            return Err(PackError::NonCanonical {
                file: file.to_owned(),
            });
        }
        self.id = Digest::sha256(bytes);
        Ok(())
    }

    /// The immutable envelope metadata.
    pub(crate) const fn metadata(&self) -> &PackMetadata {
        &self.metadata
    }

    /// The content hash of the canonical bytes.
    pub(crate) const fn id(&self) -> &Digest {
        &self.id
    }

    /// The canonical bytes the identity covers.
    pub(crate) fn canonical_bytes(&self) -> &[u8] {
        self.canonical.as_bytes()
    }

    /// The canonical JSON text, including its trailing newline.
    pub(crate) fn canonical_text(&self) -> &str {
        &self.canonical
    }

    /// The lifecycle state.
    pub(crate) const fn state(&self) -> PackState {
        self.state
    }

    /// Marks the pack validated after the schema check.
    pub(crate) fn validate(&mut self) -> Result<(), PackStateError> {
        self.transition(PackState::Validated)
    }

    /// Rejects a validated pack and returns it to draft for editing.
    pub(crate) fn reject(&mut self) -> Result<(), PackStateError> {
        self.transition(PackState::Draft)
    }

    /// Freezes a validated pack.
    pub(crate) fn freeze(&mut self) -> Result<(), PackStateError> {
        self.transition(PackState::Frozen)
    }

    /// Retires a frozen pack from new runs.
    pub(crate) fn retire(&mut self) -> Result<(), PackStateError> {
        self.transition(PackState::Retired)
    }

    fn transition(&mut self, to: PackState) -> Result<(), PackStateError> {
        let from = self.state;
        if from.can_transition_to(to) {
            self.state = to;
            Ok(())
        } else {
            Err(PackStateError::new(from, to))
        }
    }
}
