//! Artifact verification and the canonical digest.

use std::sync::Arc;

use quantrule_api::{ArtifactRef, Digest};

use crate::evidence::EvidenceError;
use crate::evidence::manifest::RunManifest;
use crate::runtime::adapters::ObjectStore;

/// Verify the run manifest and every artifact blob, and return the canonical
/// digest.
///
/// The manifest is the only publish point: a run with no manifest has no
/// artifact set, whatever blobs exist.
///
/// # Errors
///
/// Returns [`EvidenceError::MissingManifest`] or [`EvidenceError::Manifest`]
/// when the manifest is absent, malformed, or disagrees with the run record.
/// Returns [`EvidenceError::Missing`] or [`EvidenceError::Changed`] when a blob
/// cannot be trusted.
pub async fn verify(
    objects: &Arc<dyn ObjectStore>,
    run_id: &str,
    artifacts: &[ArtifactRef],
) -> Result<Digest, EvidenceError> {
    let key = RunManifest::key(run_id);
    let bytes = objects
        .get(&key)
        .await
        .map_err(|_| EvidenceError::MissingManifest)?;
    let manifest =
        RunManifest::from_bytes(&bytes).map_err(|error| EvidenceError::Manifest(error.0))?;
    if manifest.run_id != run_id {
        return Err(EvidenceError::Manifest(format!(
            "the manifest belongs to run {}, not {run_id}",
            manifest.run_id
        )));
    }
    if !manifest.matches(artifacts) {
        return Err(EvidenceError::Manifest(
            "the manifest does not match the run record".to_owned(),
        ));
    }
    let digest = manifest.computed_digest();
    if digest != manifest.digest {
        return Err(EvidenceError::Manifest(
            "the manifest digest does not match its entries".to_owned(),
        ));
    }
    for artifact in &manifest.artifacts {
        let blob = objects
            .get(&RunManifest::blob_key(&artifact.sha256))
            .await
            .map_err(|_| EvidenceError::Missing(artifact.name.clone()))?;
        if blob.len() as u64 != artifact.bytes || Digest::sha256(&blob) != artifact.sha256 {
            return Err(EvidenceError::Changed(artifact.name.clone()));
        }
    }
    Ok(digest)
}

/// The canonical digest over `results.json`, `*.csv`, and `*.ndjson`.
#[must_use]
pub fn digest(artifacts: &[ArtifactRef]) -> Digest {
    crate::evidence::manifest::canonical_digest(artifacts)
}
