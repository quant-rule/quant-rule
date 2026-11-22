//! The in-memory artifact catalog.
//!
//! It carries the same rules as the Postgres store: one content-addressed row
//! per profile, idempotent references, and a retention pass that removes only
//! unreferenced rows and records a tombstone.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use quantrule_api::{
    ArtifactKind, ArtifactRecord, ArtifactReference, ArtifactSummary, CollectedArtifact, Digest,
    ProfileId,
};
use quantrule_runtime::adapters::BoxFuture;

use super::store::{ArtifactCatalog, ArtifactError, NewArtifact};

#[derive(Debug, Clone, PartialEq, Eq)]
struct RetentionEvent {
    profile_id: String,
    artifact_id: Digest,
    bytes: u64,
}

#[derive(Debug, Default)]
struct MemoryState {
    rows: BTreeMap<(String, String, String), ArtifactRecord>,
    references: BTreeMap<(String, String, String), BTreeMap<String, ArtifactReference>>,
    events: Vec<RetentionEvent>,
}

/// An in-memory artifact catalog.
#[derive(Debug, Clone, Default)]
pub struct MemoryArtifacts {
    inner: Arc<Mutex<MemoryState>>,
}

impl MemoryArtifacts {
    /// The retention tombstones recorded for one profile.
    ///
    /// This exists so a fast test can prove the tombstone without a database.
    #[must_use]
    pub fn retention_events(&self, profile: ProfileId) -> Vec<(Digest, u64)> {
        self.inner
            .lock()
            .expect("the artifact lock")
            .events
            .iter()
            .filter(|event| event.profile_id == profile.as_str())
            .map(|event| (event.artifact_id.clone(), event.bytes))
            .collect()
    }
}

fn key(profile: &ProfileId, kind: ArtifactKind, artifact_id: &str) -> (String, String, String) {
    (
        profile.as_str().to_owned(),
        kind.code().to_owned(),
        artifact_id.to_owned(),
    )
}

fn reference_count(
    references: &BTreeMap<(String, String, String), BTreeMap<String, ArtifactReference>>,
    key: &(String, String, String),
) -> u64 {
    references
        .get(key)
        .map_or(0, |entries| entries.len() as u64)
}

impl ArtifactCatalog for MemoryArtifacts {
    fn register(
        &self,
        profile: ProfileId,
        artifact: NewArtifact,
    ) -> BoxFuture<Result<ArtifactRecord, ArtifactError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let mut state = inner
                .lock()
                .map_err(|_| ArtifactError::Store("the artifact lock is poisoned".to_owned()))?;
            let key = key(&profile, artifact.kind, &artifact.artifact_id.to_string());
            if let Some(existing) = state.rows.get(&key) {
                return Ok(existing.clone());
            }
            let record = ArtifactRecord {
                summary: ArtifactSummary {
                    artifact_id: artifact.artifact_id.clone(),
                    kind: artifact.kind,
                    bytes: artifact.bytes,
                    created_at: artifact.created_at,
                },
                object_key: artifact.object_key,
                sha256: artifact.artifact_id,
            };
            state.rows.insert(key, record.clone());
            Ok(record)
        })
    }

    fn get(
        &self,
        profile: ProfileId,
        kind: ArtifactKind,
        artifact_id: String,
    ) -> BoxFuture<Result<Option<ArtifactRecord>, ArtifactError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            Ok(inner
                .lock()
                .map_err(|_| ArtifactError::Store("the artifact lock is poisoned".to_owned()))?
                .rows
                .get(&key(&profile, kind, &artifact_id))
                .cloned())
        })
    }

    fn list(
        &self,
        profile: ProfileId,
        kind: ArtifactKind,
    ) -> BoxFuture<Result<Vec<ArtifactRecord>, ArtifactError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let state = inner
                .lock()
                .map_err(|_| ArtifactError::Store("the artifact lock is poisoned".to_owned()))?;
            let mut rows: Vec<&ArtifactRecord> = state
                .rows
                .iter()
                .filter(|((owner, stored_kind, _), _)| {
                    owner == profile.as_str() && stored_kind == kind.code()
                })
                .map(|(_, record)| record)
                .collect();
            rows.sort_by(|left, right| left.summary.artifact_id.cmp(&right.summary.artifact_id));
            Ok(rows.into_iter().cloned().collect())
        })
    }

    fn add_reference(
        &self,
        profile: ProfileId,
        kind: ArtifactKind,
        artifact_id: String,
        reference: ArtifactReference,
    ) -> BoxFuture<Result<u64, ArtifactError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let mut state = inner
                .lock()
                .map_err(|_| ArtifactError::Store("the artifact lock is poisoned".to_owned()))?;
            let key = key(&profile, kind, &artifact_id);
            if !state.rows.contains_key(&key) {
                return Err(ArtifactError::NotFound);
            }
            state
                .references
                .entry(key.clone())
                .or_default()
                .insert(reference.reference_id.clone(), reference);
            Ok(reference_count(&state.references, &key))
        })
    }

    fn release_reference(
        &self,
        profile: ProfileId,
        kind: ArtifactKind,
        artifact_id: String,
        reference_id: String,
    ) -> BoxFuture<Result<u64, ArtifactError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let mut state = inner
                .lock()
                .map_err(|_| ArtifactError::Store("the artifact lock is poisoned".to_owned()))?;
            let key = key(&profile, kind, &artifact_id);
            if !state.rows.contains_key(&key) {
                return Err(ArtifactError::NotFound);
            }
            if let Some(entries) = state.references.get_mut(&key) {
                entries.remove(&reference_id);
            }
            Ok(reference_count(&state.references, &key))
        })
    }

    fn references(
        &self,
        profile: ProfileId,
        kind: ArtifactKind,
        artifact_id: String,
    ) -> BoxFuture<Result<Vec<ArtifactReference>, ArtifactError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let state = inner
                .lock()
                .map_err(|_| ArtifactError::Store("the artifact lock is poisoned".to_owned()))?;
            let key = key(&profile, kind, &artifact_id);
            if !state.rows.contains_key(&key) {
                return Err(ArtifactError::NotFound);
            }
            Ok(state
                .references
                .get(&key)
                .map(|entries| entries.values().cloned().collect())
                .unwrap_or_default())
        })
    }

    fn collect(
        &self,
        profile: ProfileId,
        kind: ArtifactKind,
        cutoff: String,
        collected_at: String,
    ) -> BoxFuture<Result<Vec<CollectedArtifact>, ArtifactError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let mut state = inner
                .lock()
                .map_err(|_| ArtifactError::Store("the artifact lock is poisoned".to_owned()))?;
            let eligible: Vec<(String, String, String)> = state
                .rows
                .iter()
                .filter(|((owner, stored_kind, _), record)| {
                    owner == profile.as_str()
                        && stored_kind == kind.code()
                        && record.summary.created_at <= cutoff
                        && reference_count(
                            &state.references,
                            &(
                                owner.clone(),
                                stored_kind.clone(),
                                record.summary.artifact_id.to_string(),
                            ),
                        ) == 0
                })
                .map(|(key, _)| key.clone())
                .collect();
            let mut collected = Vec::new();
            for key in eligible {
                let record = state.rows.remove(&key).ok_or(ArtifactError::NotFound)?;
                state.references.remove(&key);
                let object_shared = state
                    .rows
                    .values()
                    .any(|other| other.object_key == record.object_key);
                state.events.push(RetentionEvent {
                    profile_id: key.0.clone(),
                    artifact_id: record.summary.artifact_id.clone(),
                    bytes: record.summary.bytes,
                });
                collected.push(CollectedArtifact {
                    artifact_id: record.summary.artifact_id,
                    object_key: record.object_key,
                    bytes: record.summary.bytes,
                    object_shared,
                });
            }
            let _ = collected_at;
            Ok(collected)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quantrule_api::DatasetReferenceKind;

    fn profile(name: &str) -> ProfileId {
        ProfileId::parse(name).expect("profile")
    }

    fn artifact(seed: &[u8], created_at: &str) -> NewArtifact {
        let id = Digest::sha256(seed);
        NewArtifact {
            artifact_id: id.clone(),
            kind: ArtifactKind::Signal,
            object_key: format!("artifacts/{}", id.as_hex()),
            bytes: 128,
            created_at: created_at.to_owned(),
        }
    }

    fn reference(id: &str) -> ArtifactReference {
        ArtifactReference {
            reference_id: id.to_owned(),
            kind: DatasetReferenceKind::Run,
            created_at: "2026-11-22T00:00:00Z".to_owned(),
        }
    }

    #[tokio::test]
    async fn a_referenced_artifact_is_retained_and_an_unreferenced_one_is_collected() {
        let catalog = MemoryArtifacts::default();
        let retained = artifact(b"retained", "2026-01-01T00:00:00Z");
        let collected = artifact(b"collected", "2026-01-01T00:00:00Z");
        catalog
            .register(profile("default"), retained.clone())
            .await
            .expect("retained");
        catalog
            .register(profile("default"), collected.clone())
            .await
            .expect("collected");
        assert_eq!(
            catalog
                .add_reference(
                    profile("default"),
                    ArtifactKind::Signal,
                    retained.artifact_id.to_string(),
                    reference("run:one"),
                )
                .await
                .expect("reference"),
            1
        );
        let rows = catalog
            .collect(
                profile("default"),
                ArtifactKind::Signal,
                "2026-11-01T00:00:00Z".to_owned(),
                "2026-11-22T00:00:00Z".to_owned(),
            )
            .await
            .expect("collect");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].artifact_id, collected.artifact_id);
        assert!(!rows[0].object_shared);
        assert_eq!(
            catalog.retention_events(profile("default")),
            vec![(collected.artifact_id.clone(), 128)]
        );
        assert!(
            catalog
                .get(
                    profile("default"),
                    ArtifactKind::Signal,
                    retained.artifact_id.to_string()
                )
                .await
                .expect("get")
                .is_some()
        );
    }

    #[tokio::test]
    async fn the_window_is_respected_and_references_are_idempotent() {
        let catalog = MemoryArtifacts::default();
        let fresh = artifact(b"fresh", "2026-11-20T00:00:00Z");
        catalog
            .register(profile("default"), fresh.clone())
            .await
            .expect("register");
        assert!(
            catalog
                .collect(
                    profile("default"),
                    ArtifactKind::Signal,
                    "2026-11-01T00:00:00Z".to_owned(),
                    "2026-11-22T00:00:00Z".to_owned(),
                )
                .await
                .expect("collect")
                .is_empty()
        );
        for _ in 0..2 {
            assert_eq!(
                catalog
                    .add_reference(
                        profile("default"),
                        ArtifactKind::Signal,
                        fresh.artifact_id.to_string(),
                        reference("run:one"),
                    )
                    .await
                    .expect("reference"),
                1
            );
        }
        assert_eq!(
            catalog
                .release_reference(
                    profile("default"),
                    ArtifactKind::Signal,
                    fresh.artifact_id.to_string(),
                    "run:one".to_owned(),
                )
                .await
                .expect("release"),
            0
        );
    }
}
