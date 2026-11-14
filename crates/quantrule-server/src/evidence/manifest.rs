//! The run artifact manifest.
//!
//! The manifest is the atomic publish point for a run's artifact set. Artifact
//! bytes live under their own SHA-256, so a retry reuses them; nothing is part
//! of the run until this one object exists.

use quantrule_api::{ArtifactRef, Digest};

/// The manifest schema version.
pub const SCHEMA: &str = "quantrule.run-manifest/v1";

/// One artifact entry.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ManifestArtifact {
    /// The path inside the run bundle.
    pub name: String,
    /// The number of bytes.
    pub bytes: u64,
    /// The SHA-256 of the bytes.
    pub sha256: Digest,
}

/// The complete artifact set of one run.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RunManifest {
    /// The schema token.
    pub schema: String,
    /// The run it belongs to.
    pub run_id: String,
    /// The canonical digest over the canonical artifacts.
    pub digest: Digest,
    /// The entries, sorted by name.
    pub artifacts: Vec<ManifestArtifact>,
}

/// Why a manifest could not be decoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestError(pub String);

impl std::fmt::Display for ManifestError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ManifestError {}

impl RunManifest {
    /// Build a manifest from a run's artifact references.
    #[must_use]
    pub fn from_artifacts(run_id: &str, artifacts: &[ArtifactRef]) -> Self {
        let mut sorted = artifacts.to_vec();
        sorted.sort_by(|left, right| left.name.cmp(&right.name));
        let entries: Vec<ManifestArtifact> = sorted
            .iter()
            .map(|artifact| ManifestArtifact {
                name: artifact.name.clone(),
                bytes: artifact.bytes,
                sha256: artifact.sha256.clone(),
            })
            .collect();
        Self {
            schema: SCHEMA.to_owned(),
            run_id: run_id.to_owned(),
            digest: canonical_digest(&sorted),
            artifacts: entries,
        }
    }

    /// Decode and validate a manifest.
    ///
    /// # Errors
    ///
    /// Returns [`ManifestError`] when the bytes are not JSON, the schema is not
    /// the current one, the entries are not sorted, or a run id is empty.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ManifestError> {
        let manifest: Self = serde_json::from_slice(bytes)
            .map_err(|error| ManifestError(format!("the manifest is not JSON: {error}")))?;
        if manifest.schema != SCHEMA {
            return Err(ManifestError(format!(
                "the manifest schema is {}, not {SCHEMA}",
                manifest.schema
            )));
        }
        if manifest.run_id.is_empty() {
            return Err(ManifestError("the manifest has no run id".to_owned()));
        }
        if manifest
            .artifacts
            .windows(2)
            .any(|pair| pair[0].name >= pair[1].name)
        {
            return Err(ManifestError(
                "the manifest entries are not sorted by name".to_owned(),
            ));
        }
        Ok(manifest)
    }

    /// Encode the manifest deterministically.
    ///
    /// # Errors
    ///
    /// Returns [`ManifestError`] when serialization fails.
    pub fn to_bytes(&self) -> Result<Vec<u8>, ManifestError> {
        serde_json::to_vec(self).map_err(|error| ManifestError(error.to_string()))
    }

    /// The object key of the manifest for a run.
    #[must_use]
    pub fn key(run_id: &str) -> String {
        format!("runs/{run_id}/manifest.json")
    }

    /// The object key of one content-addressed blob.
    #[must_use]
    pub fn blob_key(sha256: &Digest) -> String {
        format!("artifacts/{}", sha256.as_hex())
    }

    /// The artifact references in the manifest.
    #[must_use]
    pub fn refs(&self) -> Vec<ArtifactRef> {
        self.artifacts
            .iter()
            .map(|artifact| ArtifactRef {
                name: artifact.name.clone(),
                bytes: artifact.bytes,
                sha256: artifact.sha256.clone(),
            })
            .collect()
    }

    /// Whether the entries equal exactly these artifact references.
    #[must_use]
    pub fn matches(&self, artifacts: &[ArtifactRef]) -> bool {
        let mut sorted = artifacts.to_vec();
        sorted.sort_by(|left, right| left.name.cmp(&right.name));
        self.artifacts.len() == sorted.len()
            && self
                .artifacts
                .iter()
                .zip(sorted.iter())
                .all(|(entry, artifact)| {
                    entry.name == artifact.name
                        && entry.bytes == artifact.bytes
                        && entry.sha256 == artifact.sha256
                })
    }

    /// The canonical digest recomputed from the entries.
    #[must_use]
    pub fn computed_digest(&self) -> Digest {
        let entries: Vec<(String, Digest)> = self
            .artifacts
            .iter()
            .filter(|artifact| canonical(&artifact.name))
            .map(|artifact| (artifact.name.clone(), artifact.sha256.clone()))
            .collect();
        Digest::canonical(&entries)
    }
}

/// The canonical digest over the data artifacts in a set.
///
/// `results.json`, `*.csv`, and `*.ndjson` are included; provenance, logs, and
/// any file that records where or when the run happened are excluded.
#[must_use]
pub fn canonical_digest(artifacts: &[ArtifactRef]) -> Digest {
    let entries: Vec<(String, Digest)> = artifacts
        .iter()
        .filter(|artifact| canonical(&artifact.name))
        .map(|artifact| (artifact.name.clone(), artifact.sha256.clone()))
        .collect();
    Digest::canonical(&entries)
}

fn canonical(name: &str) -> bool {
    name == "results.json" || name.ends_with(".csv") || name.ends_with(".ndjson")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn artifact(name: &str, bytes: &[u8]) -> ArtifactRef {
        ArtifactRef {
            name: name.to_owned(),
            bytes: bytes.len() as u64,
            sha256: Digest::sha256(bytes),
        }
    }

    #[test]
    fn the_manifest_round_trips_and_ignores_input_order() {
        let a = artifact("results.json", b"a");
        let b = artifact("trades.csv", b"b");
        let first = RunManifest::from_artifacts("run", &[a.clone(), b.clone()]);
        let second = RunManifest::from_artifacts("run", &[b, a]);
        assert_eq!(first, second);
        let decoded = RunManifest::from_bytes(&first.to_bytes().expect("bytes")).expect("decode");
        assert_eq!(decoded, first);
        assert_eq!(decoded.computed_digest(), first.digest);
    }

    #[test]
    fn provenance_and_logs_are_not_in_the_digest() {
        let results = artifact("results.json", b"a");
        let provenance = artifact("provenance.json", b"b");
        assert_eq!(
            canonical_digest(std::slice::from_ref(&results)),
            canonical_digest(&[results, provenance])
        );
    }

    #[test]
    fn a_mismatched_entry_is_rejected() {
        let results = artifact("results.json", b"a");
        let manifest = RunManifest::from_artifacts("run", std::slice::from_ref(&results));
        assert!(manifest.matches(&[results]));
        assert!(!manifest.matches(&[artifact("results.json", b"changed")]));
        assert!(!manifest.matches(&[artifact("other.csv", b"a")]));
    }

    #[test]
    fn an_unsorted_or_wrong_schema_manifest_is_rejected() {
        let results = artifact("results.json", b"a");
        let mut manifest = RunManifest::from_artifacts("run", &[results]);
        manifest.schema = "quantrule.run-manifest/v2".to_owned();
        assert!(RunManifest::from_bytes(&manifest.to_bytes().expect("bytes")).is_err());
    }
}
