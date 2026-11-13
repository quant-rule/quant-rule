//! Artifact storage and the canonical digest.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use quantrule_api::{ArtifactRef, Digest};

use crate::evidence::EvidenceError;
use crate::runtime::adapters::ObjectStore;

/// Store every file under `root` in CAS and return its reference.
///
/// Names are relative to `root`, sorted, and use `/`.
///
/// # Errors
///
/// Returns [`EvidenceError::Store`] for an I/O or object-store failure.
pub async fn store(
    objects: &Arc<dyn ObjectStore>,
    run_id: &str,
    root: &Path,
) -> Result<Vec<ArtifactRef>, EvidenceError> {
    let mut files = Vec::new();
    walk(root, root, &mut files)?;
    files.sort_by(|left, right| left.0.cmp(&right.0));
    let mut artifacts = Vec::with_capacity(files.len());
    for (name, bytes) in files {
        let key = format!("runs/{run_id}/{name}");
        let present = objects
            .exists(&key)
            .await
            .map_err(|error| EvidenceError::Store(error.to_string()))?;
        if !present {
            objects
                .put(&key, bytes.clone())
                .await
                .map_err(|error| EvidenceError::Store(error.to_string()))?;
        }
        artifacts.push(ArtifactRef {
            name,
            bytes: bytes.len() as u64,
            sha256: Digest::sha256(&bytes),
        });
    }
    Ok(artifacts)
}

/// Verify every artifact and return the recomputed canonical digest.
///
/// # Errors
///
/// Returns [`EvidenceError::Missing`] or [`EvidenceError::Changed`] when the
/// stored bytes cannot be trusted.
pub async fn verify(
    objects: &Arc<dyn ObjectStore>,
    run_id: &str,
    artifacts: &[ArtifactRef],
) -> Result<Digest, EvidenceError> {
    for artifact in artifacts {
        let key = format!("runs/{run_id}/{}", artifact.name);
        let bytes = objects
            .get(&key)
            .await
            .map_err(|_| EvidenceError::Missing(artifact.name.clone()))?;
        if bytes.len() as u64 != artifact.bytes || Digest::sha256(&bytes) != artifact.sha256 {
            return Err(EvidenceError::Changed(artifact.name.clone()));
        }
    }
    Ok(digest(artifacts))
}

/// The canonical digest over `results.json`, `*.csv`, and `*.ndjson`.
#[must_use]
pub fn digest(artifacts: &[ArtifactRef]) -> Digest {
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

fn walk(
    root: &Path,
    directory: &Path,
    files: &mut Vec<(String, Vec<u8>)>,
) -> Result<(), EvidenceError> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(directory)
        .map_err(|error| EvidenceError::Store(format!("read the artifact bundle: {error}")))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<_, _>>()
        .map_err(|error| EvidenceError::Store(format!("read the artifact bundle: {error}")))?;
    entries.sort();
    for path in entries {
        if path.is_dir() {
            walk(root, &path, files)?;
        } else if path.is_file() {
            let name = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            let bytes = std::fs::read(&path)
                .map_err(|error| EvidenceError::Store(format!("read {name}: {error}")))?;
            files.push((name, bytes));
        }
    }
    Ok(())
}
