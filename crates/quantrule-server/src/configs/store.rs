//! Configuration storage behind one trait.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use quantrule_api::{ConfigOrigin, ConfigSummary, Digest};

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
    /// Insert or refresh one record.
    fn upsert(&self, record: ConfigRecord) -> BoxFuture<Result<(), ConfigStoreError>>;

    /// Every record, ordered by id.
    fn list(&self) -> BoxFuture<Result<Vec<ConfigRecord>, ConfigStoreError>>;

    /// One record by id.
    fn get(&self, config_id: String) -> BoxFuture<Result<Option<ConfigRecord>, ConfigStoreError>>;
}

/// An in-memory catalog, for tests and for a local instance.
#[derive(Debug, Clone, Default)]
pub struct MemoryConfigs {
    inner: Arc<Mutex<BTreeMap<String, ConfigRecord>>>,
}

impl ConfigCatalog for MemoryConfigs {
    fn upsert(&self, record: ConfigRecord) -> BoxFuture<Result<(), ConfigStoreError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            inner
                .lock()
                .map_err(|_| ConfigStoreError("the config lock is poisoned".to_owned()))?
                .insert(record.config_id.clone(), record);
            Ok(())
        })
    }

    fn list(&self) -> BoxFuture<Result<Vec<ConfigRecord>, ConfigStoreError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            Ok(inner
                .lock()
                .map_err(|_| ConfigStoreError("the config lock is poisoned".to_owned()))?
                .values()
                .cloned()
                .collect())
        })
    }

    fn get(&self, config_id: String) -> BoxFuture<Result<Option<ConfigRecord>, ConfigStoreError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            Ok(inner
                .lock()
                .map_err(|_| ConfigStoreError("the config lock is poisoned".to_owned()))?
                .get(&config_id)
                .cloned())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(id: &str, valid: bool) -> ConfigRecord {
        ConfigRecord {
            config_id: id.to_owned(),
            name: "run.json".to_owned(),
            origin: ConfigOrigin::Repository,
            valid,
            message: None,
            config_hash: valid.then(|| Digest::sha256(b"config")),
            body: "{}".to_owned(),
            files: BTreeMap::new(),
            created_at: "2026-11-13T00:00:00Z".to_owned(),
        }
    }

    #[tokio::test]
    async fn a_record_round_trips() {
        let store = MemoryConfigs::default();
        store.upsert(record("cfg_a", true)).await.expect("upsert");
        let loaded = store.get("cfg_a".to_owned()).await.expect("get");
        assert_eq!(loaded, Some(record("cfg_a", true)));
    }

    #[tokio::test]
    async fn an_upsert_replaces_by_id() {
        let store = MemoryConfigs::default();
        store.upsert(record("cfg_a", false)).await.expect("first");
        store.upsert(record("cfg_a", true)).await.expect("second");
        let listed = store.list().await.expect("list");
        assert_eq!(listed.len(), 1);
        assert!(listed[0].valid);
    }

    #[tokio::test]
    async fn the_list_is_ordered_by_id() {
        let store = MemoryConfigs::default();
        store.upsert(record("cfg_b", true)).await.expect("b");
        store.upsert(record("cfg_a", true)).await.expect("a");
        let ids: Vec<String> = store
            .list()
            .await
            .expect("list")
            .into_iter()
            .map(|record| record.config_id)
            .collect();
        assert_eq!(ids, vec!["cfg_a".to_owned(), "cfg_b".to_owned()]);
    }
}
