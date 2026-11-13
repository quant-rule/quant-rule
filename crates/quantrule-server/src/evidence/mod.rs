//! The evidence boundary.
//!
//! The worker computes and stores artifacts; evidence verifies the hash chain,
//! recomputes the canonical digest, and finalizes the run. The module is the
//! seam that becomes `qr-evidence` without a worker change.

pub mod artifacts;
pub mod finalize;

pub use artifacts::{digest, store, verify};
pub use finalize::{EvidenceServices, FinalizeOutcome, finalize};

/// Why evidence could not accept a computed run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceError {
    /// The run has no artifacts.
    NoArtifacts,
    /// A required stage did not finish.
    StageIncomplete(String),
    /// An artifact is missing from the store.
    Missing(String),
    /// An artifact's bytes do not match its recorded hash.
    Changed(String),
    /// The stored digest does not match the recomputed one.
    DigestMismatch,
    /// A store failed.
    Store(String),
}

impl std::fmt::Display for EvidenceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoArtifacts => formatter.write_str("the run has no artifacts"),
            Self::StageIncomplete(stage) => write!(formatter, "the {stage} stage did not finish"),
            Self::Missing(name) => write!(formatter, "artifact {name} is missing"),
            Self::Changed(name) => write!(formatter, "artifact {name} does not match its hash"),
            Self::DigestMismatch => {
                formatter.write_str("the stored digest does not match the artifacts")
            }
            Self::Store(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for EvidenceError {}
