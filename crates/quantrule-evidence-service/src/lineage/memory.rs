//! The in-memory run lineage store.
//!
//! It carries the same rules as the Postgres store: one immutable reference set
//! per run, idempotent repetition, and a conflict for a different set.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use quantrule_api::{ProfileId, RunArtifactReference, RunId, RunLineage};
use quantrule_runtime::adapters::BoxFuture;

use super::{LineageError, RunLineageStore, normalize};

/// An in-memory run lineage store.
#[derive(Debug, Clone, Default)]
pub struct MemoryLineage {
    inner: Arc<Mutex<BTreeMap<(String, String), RunLineage>>>,
}

impl RunLineageStore for MemoryLineage {
    fn record(
        &self,
        profile: ProfileId,
        run_id: RunId,
        references: Vec<RunArtifactReference>,
        recorded_at: String,
    ) -> BoxFuture<Result<RunLineage, LineageError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let references = normalize(references)?;
            let key = (profile.as_str().to_owned(), run_id.to_string());
            let mut lineage = inner
                .lock()
                .map_err(|_| LineageError::Store("the lineage lock is poisoned".to_owned()))?;
            if let Some(existing) = lineage.get(&key) {
                if existing.references == references {
                    return Ok(existing.clone());
                }
                return Err(LineageError::Conflict);
            }
            let record = RunLineage {
                run_id,
                references,
                recorded_at,
            };
            lineage.insert(key, record.clone());
            Ok(record)
        })
    }

    fn get(
        &self,
        profile: ProfileId,
        run_id: RunId,
    ) -> BoxFuture<Result<Option<RunLineage>, LineageError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            Ok(inner
                .lock()
                .map_err(|_| LineageError::Store("the lineage lock is poisoned".to_owned()))?
                .get(&(profile.as_str().to_owned(), run_id.to_string()))
                .cloned())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quantrule_api::{Digest, PackKind, PackProvenance, PackState, ReferenceKind};

    fn profile(name: &str) -> ProfileId {
        ProfileId::parse(name).expect("profile")
    }

    fn run(seed: &[u8]) -> RunId {
        RunId(Digest::sha256(seed))
    }

    fn strategy(seed: &[u8]) -> RunArtifactReference {
        let id = Digest::sha256(seed);
        RunArtifactReference {
            kind: ReferenceKind::StrategyPack,
            id: id.clone(),
            hash: id,
            pack: Some(PackProvenance {
                kind: PackKind::Strategy,
                name: "compression-fade-5m".to_owned(),
                version: "1.0.0".to_owned(),
                owner: "operator".to_owned(),
                state: PackState::Frozen,
            }),
        }
    }

    #[tokio::test]
    async fn the_same_set_is_idempotent_and_a_different_one_conflicts() {
        let store = MemoryLineage::default();
        let run_id = run(b"run");
        let references = vec![strategy(b"pack")];
        let first = store
            .record(
                profile("default"),
                run_id.clone(),
                references.clone(),
                "2026-11-22T00:00:00Z".to_owned(),
            )
            .await
            .expect("record");
        let again = store
            .record(
                profile("default"),
                run_id.clone(),
                references,
                "2026-11-23T00:00:00Z".to_owned(),
            )
            .await
            .expect("idempotent");
        assert_eq!(again, first);
        assert_eq!(
            store
                .record(
                    profile("default"),
                    run_id.clone(),
                    vec![strategy(b"other")],
                    "2026-11-23T00:00:00Z".to_owned(),
                )
                .await,
            Err(LineageError::Conflict)
        );
        assert_eq!(
            store.get(profile("default"), run_id).await.expect("get"),
            Some(first)
        );
    }

    #[tokio::test]
    async fn a_reference_needs_one_content_address_and_its_own_pack_kind() {
        let store = MemoryLineage::default();
        let mut wrong = strategy(b"pack");
        wrong.hash = Digest::sha256(b"other");
        assert!(matches!(
            store
                .record(
                    profile("default"),
                    run(b"run"),
                    vec![wrong],
                    "2026-11-22T00:00:00Z".to_owned(),
                )
                .await,
            Err(LineageError::Invalid(_))
        ));
        let mut wrong_kind = strategy(b"pack");
        if let Some(pack) = wrong_kind.pack.as_mut() {
            pack.kind = PackKind::Portfolio;
        }
        assert!(matches!(
            store
                .record(
                    profile("default"),
                    run(b"run"),
                    vec![wrong_kind],
                    "2026-11-22T00:00:00Z".to_owned(),
                )
                .await,
            Err(LineageError::Invalid(_))
        ));
    }
}
