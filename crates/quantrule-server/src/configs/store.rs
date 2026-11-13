//! Configuration storage behind one trait.
//!
//! A config row is scoped by profile; the file bundle it points at is shared by
//! content id. Two profiles that use the same configuration keep two rows and
//! one set of files.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use quantrule_api::{ConfigOrigin, ConfigSummary, Digest, ProfileId};

use crate::runtime::adapters::BoxFuture;

/// One catalogued configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigRecord {
    /// Stable content-derived id.
    pub config_id: String,
    /// Repository-relative name.
    pub name: String,
    /// Repository or upload.
    pub origin: ConfigOrigin,
    /// Whether the run config and its references validate.
    pub valid: bool,
    /// The parser message when invalid.
    pub message: Option<String>,
    /// SHA-256 over the five files, when valid.
    pub config_hash: Option<Digest>,
    /// The run file body as stored.
    pub body: String,
    /// The bundle's relative path to body, including `run.json`.
    pub files: BTreeMap<String, String>,
    /// Server-assigned first-seen time, RFC 3339 UTC.
    pub created_at: String,
}

impl ConfigRecord {
    /// The list projection.
    #[must_use]
    pub fn summary(&self) -> ConfigSummary {
        ConfigSummary {
            config_id: self.config_id.clone(),
            name: self.name.clone(),
            origin: self.origin,
            valid: self.valid,
            message: self.message.clone(),
            config_hash: self.config_hash.clone(),
        }
    }
}

/// Why a configuration write failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigStoreError(pub String);

impl std::fmt::Display for ConfigStoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ConfigStoreError {}

/// Where configurations are catalogued.
pub trait ConfigCatalog: Send + Sync {
    /// Insert or refresh one record for a profile.
    fn upsert(
        &self,
        profile: ProfileId,
        record: ConfigRecord,
    ) -> BoxFuture<Result<(), ConfigStoreError>>;

    /// Every record for a profile, ordered by id.
    fn list(&self, profile: ProfileId) -> BoxFuture<Result<Vec<ConfigRecord>, ConfigStoreError>>;

    /// One record by id within a profile.
    fn get(
        &self,
        profile: ProfileId,
        config_id: String,
    ) -> BoxFuture<Result<Option<ConfigRecord>, ConfigStoreError>>;
}

#[derive(Debug, Default)]
struct MemoryState {
    rows: BTreeMap<(String, String), ConfigRecord>,
    bundles: BTreeMap<String, BTreeMap<String, String>>,
}

/// An in-memory catalog, for tests and for a local instance.
#[derive(Debug, Clone, Default)]
pub struct MemoryConfigs {
    inner: Arc<Mutex<MemoryState>>,
}

fn with_bundle(state: &MemoryState, mut record: ConfigRecord) -> ConfigRecord {
    record.files = state
        .bundles
        .get(&record.config_id)
        .cloned()
        .unwrap_or_default();
    record
}

impl ConfigCatalog for MemoryConfigs {
    fn upsert(
        &self,
        profile: ProfileId,
        record: ConfigRecord,
    ) -> BoxFuture<Result<(), ConfigStoreError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let mut state = inner
                .lock()
                .map_err(|_| ConfigStoreError("the config lock is poisoned".to_owned()))?;
            state
                .bundles
                .insert(record.config_id.clone(), record.files.clone());
            let mut row = record;
            row.files = BTreeMap::new();
            state
                .rows
                .insert((profile.as_str().to_owned(), row.config_id.clone()), row);
            Ok(())
        })
    }

    fn list(&self, profile: ProfileId) -> BoxFuture<Result<Vec<ConfigRecord>, ConfigStoreError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let state = inner
                .lock()
                .map_err(|_| ConfigStoreError("the config lock is poisoned".to_owned()))?;
            Ok(state
                .rows
                .iter()
                .filter(|((owner, _), _)| owner == profile.as_str())
                .map(|(_, record)| with_bundle(&state, record.clone()))
                .collect())
        })
    }

    fn get(
        &self,
        profile: ProfileId,
        config_id: String,
    ) -> BoxFuture<Result<Option<ConfigRecord>, ConfigStoreError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let state = inner
                .lock()
                .map_err(|_| ConfigStoreError("the config lock is poisoned".to_owned()))?;
            Ok(state
                .rows
                .get(&(profile.as_str().to_owned(), config_id))
                .map(|record| with_bundle(&state, record.clone())))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(name: &str) -> ProfileId {
        ProfileId::parse(name).expect("profile")
    }

    fn record(id: &str, valid: bool) -> ConfigRecord {
        ConfigRecord {
            config_id: id.to_owned(),
            name: "run.json".to_owned(),
            origin: ConfigOrigin::Repository,
            valid,
            message: None,
            config_hash: valid.then(|| Digest::sha256(b"config")),
            body: "{}".to_owned(),
            files: BTreeMap::from([("run.json".to_owned(), "{}".to_owned())]),
            created_at: "2026-11-13T00:00:00Z".to_owned(),
        }
    }

    #[tokio::test]
    async fn a_record_round_trips_with_its_files() {
        let store = MemoryConfigs::default();
        store
            .upsert(profile("default"), record("cfg_a", true))
            .await
            .expect("upsert");
        let loaded = store
            .get(profile("default"), "cfg_a".to_owned())
            .await
            .expect("get");
        assert_eq!(loaded, Some(record("cfg_a", true)));
    }

    #[tokio::test]
    async fn an_upsert_replaces_by_id() {
        let store = MemoryConfigs::default();
        store
            .upsert(profile("default"), record("cfg_a", false))
            .await
            .expect("first");
        store
            .upsert(profile("default"), record("cfg_a", true))
            .await
            .expect("second");
        let listed = store.list(profile("default")).await.expect("list");
        assert_eq!(listed.len(), 1);
        assert!(listed[0].valid);
    }

    #[tokio::test]
    async fn the_list_is_ordered_by_id() {
        let store = MemoryConfigs::default();
        store
            .upsert(profile("default"), record("cfg_b", true))
            .await
            .expect("b");
        store
            .upsert(profile("default"), record("cfg_a", true))
            .await
            .expect("a");
        let ids: Vec<String> = store
            .list(profile("default"))
            .await
            .expect("list")
            .into_iter()
            .map(|record| record.config_id)
            .collect();
        assert_eq!(ids, vec!["cfg_a".to_owned(), "cfg_b".to_owned()]);
    }

    #[tokio::test]
    async fn two_profiles_keep_independent_rows_over_one_bundle() {
        let store = MemoryConfigs::default();
        store
            .upsert(profile("alpha"), record("cfg_a", true))
            .await
            .expect("alpha");
        store
            .upsert(profile("beta"), record("cfg_b", true))
            .await
            .expect("beta");
        assert_eq!(store.list(profile("alpha")).await.expect("alpha").len(), 1);
        assert_eq!(store.list(profile("beta")).await.expect("beta").len(), 1);
        assert_eq!(
            store
                .get(profile("gamma"), "cfg_a".to_owned())
                .await
                .expect("gamma"),
            None
        );
    }
}
