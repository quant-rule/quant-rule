//! Hypothesis storage behind one trait.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use quantrule_api::ProfileId;
use quantrule_research::SealedHypothesis;

use crate::runtime::adapters::BoxFuture;

/// A storage failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreError(pub String);

impl std::fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for StoreError {}

/// Where a sealed hypothesis lives.
pub trait HypothesisStore: Send + Sync {
    /// Write a sealed hypothesis into a profile.
    fn insert(
        &self,
        profile: ProfileId,
        sealed: SealedHypothesis,
    ) -> BoxFuture<Result<(), StoreError>>;

    /// Read one by fingerprint within a profile.
    fn load(
        &self,
        profile: ProfileId,
        fingerprint: String,
    ) -> BoxFuture<Result<Option<SealedHypothesis>, StoreError>>;
}

/// An in-memory store, for tests and for a local instance.
#[derive(Debug, Clone, Default)]
pub struct MemoryStore {
    inner: Arc<Mutex<BTreeMap<(String, String), SealedHypothesis>>>,
}

impl HypothesisStore for MemoryStore {
    fn insert(
        &self,
        profile: ProfileId,
        sealed: SealedHypothesis,
    ) -> BoxFuture<Result<(), StoreError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            inner
                .lock()
                .map_err(|_| StoreError("the store lock is poisoned".to_owned()))?
                .insert(
                    (profile.as_str().to_owned(), sealed.fingerprint().to_owned()),
                    sealed,
                );
            Ok(())
        })
    }

    fn load(
        &self,
        profile: ProfileId,
        fingerprint: String,
    ) -> BoxFuture<Result<Option<SealedHypothesis>, StoreError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            Ok(inner
                .lock()
                .map_err(|_| StoreError("the store lock is poisoned".to_owned()))?
                .get(&(profile.as_str().to_owned(), fingerprint))
                .cloned())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quantrule_research::{CostModel, Hypothesis, Partition, Split};

    fn profile(name: &str) -> ProfileId {
        ProfileId::parse(name).expect("profile")
    }

    fn sealed() -> SealedHypothesis {
        let split = Split::new(
            Partition::new("dev", 100, 200).expect("dev"),
            Partition::new("val", 200, 300).expect("val"),
        )
        .expect("split");
        Hypothesis::new(
            "a retest holds",
            "strategy/retest.toml",
            "sha256:abc",
            CostModel::new(1.0, 0.0, 0.0).expect("costs"),
            split,
            "expectancy above zero",
        )
        .expect("draft")
        .seal("2026-11-13T00:00:00Z")
        .expect("seal")
    }

    #[tokio::test]
    async fn a_sealed_hypothesis_round_trips() {
        let store = MemoryStore::default();
        let hypothesis = sealed();
        let fingerprint = hypothesis.fingerprint().to_owned();
        store
            .insert(profile("default"), hypothesis.clone())
            .await
            .expect("insert");
        let loaded = store
            .load(profile("default"), fingerprint)
            .await
            .expect("load");
        assert_eq!(loaded, Some(hypothesis));
    }

    #[tokio::test]
    async fn the_same_fingerprint_is_independent_per_profile() {
        let store = MemoryStore::default();
        let hypothesis = sealed();
        let fingerprint = hypothesis.fingerprint().to_owned();
        store
            .insert(profile("alpha"), hypothesis)
            .await
            .expect("insert");
        assert_eq!(
            store
                .load(profile("beta"), fingerprint)
                .await
                .expect("load"),
            None
        );
    }

    #[tokio::test]
    async fn an_unknown_fingerprint_is_none() {
        let store = MemoryStore::default();
        assert_eq!(
            store
                .load(profile("default"), "missing".to_owned())
                .await
                .expect("load"),
            None
        );
    }
}
