//! The dataset catalog.
//!
//! A dataset is catalogued by its content version. The bytes themselves live in
//! the object store; the catalog row is what a client lists and what a run
//! refers to. The row carries the profile, the object does not: two profiles
//! that upload the same bytes share one object and keep one row each.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use quantrule_api::{DatasetRecord, ProfileId};

use crate::runtime::adapters::BoxFuture;

/// Why a catalog write failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogError {
    /// The same bytes are already catalogued in this profile.
    AlreadyExists,
    /// The store refused the write.
    Store(String),
}

impl std::fmt::Display for CatalogError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyExists => formatter.write_str("the dataset already exists"),
            Self::Store(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for CatalogError {}

/// Where datasets are catalogued.
pub trait DatasetCatalog: Send + Sync {
    /// Add a dataset to a profile's catalog.
    fn insert(
        &self,
        profile: ProfileId,
        record: DatasetRecord,
    ) -> BoxFuture<Result<(), CatalogError>>;

    /// Read one by version within a profile.
    fn get(
        &self,
        profile: ProfileId,
        version: String,
    ) -> BoxFuture<Result<Option<DatasetRecord>, CatalogError>>;

    /// List a profile's datasets, ordered by version.
    fn list(&self, profile: ProfileId) -> BoxFuture<Result<Vec<DatasetRecord>, CatalogError>>;

    /// How many datasets a profile holds.
    fn count(&self, profile: ProfileId) -> BoxFuture<Result<u64, CatalogError>>;
}

/// An in-memory catalog, for tests and for a local instance.
#[derive(Debug, Clone, Default)]
pub struct MemoryCatalog {
    inner: Arc<Mutex<BTreeMap<(String, String), DatasetRecord>>>,
}

impl DatasetCatalog for MemoryCatalog {
    fn insert(
        &self,
        profile: ProfileId,
        record: DatasetRecord,
    ) -> BoxFuture<Result<(), CatalogError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let mut map = inner
                .lock()
                .map_err(|_| CatalogError::Store("the catalog lock is poisoned".to_owned()))?;
            let key = (
                profile.as_str().to_owned(),
                record.summary.data_version.to_string(),
            );
            if map.contains_key(&key) {
                return Err(CatalogError::AlreadyExists);
            }
            map.insert(key, record);
            Ok(())
        })
    }

    fn get(
        &self,
        profile: ProfileId,
        version: String,
    ) -> BoxFuture<Result<Option<DatasetRecord>, CatalogError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            Ok(inner
                .lock()
                .map_err(|_| CatalogError::Store("the catalog lock is poisoned".to_owned()))?
                .get(&(profile.as_str().to_owned(), version))
                .cloned())
        })
    }

    fn list(&self, profile: ProfileId) -> BoxFuture<Result<Vec<DatasetRecord>, CatalogError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let map = inner
                .lock()
                .map_err(|_| CatalogError::Store("the catalog lock is poisoned".to_owned()))?;
            Ok(map
                .iter()
                .filter(|((owner, _), _)| owner == profile.as_str())
                .map(|(_, record)| record.clone())
                .collect())
        })
    }

    fn count(&self, profile: ProfileId) -> BoxFuture<Result<u64, CatalogError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let map = inner
                .lock()
                .map_err(|_| CatalogError::Store("the catalog lock is poisoned".to_owned()))?;
            Ok(map
                .keys()
                .filter(|(owner, _)| owner == profile.as_str())
                .count() as u64)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quantrule_api::{DatasetSummary, Digest};

    fn profile(name: &str) -> ProfileId {
        ProfileId::parse(name).expect("profile")
    }

    fn record(seed: &[u8]) -> DatasetRecord {
        let version = Digest::sha256(seed);
        DatasetRecord {
            summary: DatasetSummary {
                data_version: version.clone(),
                name: "candles.csv".to_owned(),
                candles: 10,
                first_bar: 1,
                last_bar: 10,
                bytes: 100,
            },
            object_key: format!("datasets/{}", version.as_hex()),
            sha256: version,
            quality: None,
            created_at: "2026-11-13T00:00:00Z".to_owned(),
        }
    }

    #[tokio::test]
    async fn a_dataset_round_trips() {
        let catalog = MemoryCatalog::default();
        let record = record(b"candles");
        catalog
            .insert(profile("default"), record.clone())
            .await
            .expect("insert");
        let loaded = catalog
            .get(profile("default"), record.summary.data_version.to_string())
            .await
            .expect("get");
        assert_eq!(loaded, Some(record));
    }

    #[tokio::test]
    async fn the_same_bytes_are_refused_twice_in_one_profile() {
        let catalog = MemoryCatalog::default();
        catalog
            .insert(profile("default"), record(b"candles"))
            .await
            .expect("first");
        assert_eq!(
            catalog.insert(profile("default"), record(b"candles")).await,
            Err(CatalogError::AlreadyExists)
        );
    }

    #[tokio::test]
    async fn the_list_is_ordered_by_version() {
        let catalog = MemoryCatalog::default();
        let first = record(b"a");
        let second = record(b"b");
        let (low, high) = if first.summary.data_version < second.summary.data_version {
            (first, second)
        } else {
            (second, first)
        };
        catalog
            .insert(profile("default"), high.clone())
            .await
            .expect("high");
        catalog
            .insert(profile("default"), low.clone())
            .await
            .expect("low");
        let listed = catalog.list(profile("default")).await.expect("list");
        assert_eq!(listed, vec![low, high]);
    }

    #[tokio::test]
    async fn the_count_is_per_profile() {
        let catalog = MemoryCatalog::default();
        catalog
            .insert(profile("default"), record(b"one"))
            .await
            .expect("one");
        catalog
            .insert(profile("default"), record(b"two"))
            .await
            .expect("two");
        assert_eq!(catalog.count(profile("default")).await.expect("count"), 2);
        assert_eq!(catalog.count(profile("other")).await.expect("count"), 0);
    }

    #[tokio::test]
    async fn two_profiles_keep_independent_catalogs_over_shared_bytes() {
        let catalog = MemoryCatalog::default();
        let one = record(b"shared");
        let other = record(b"shared");
        catalog
            .insert(profile("default"), one.clone())
            .await
            .expect("default");
        catalog
            .insert(profile("research"), other.clone())
            .await
            .expect("research");

        assert_eq!(one.object_key, other.object_key);
        assert_eq!(
            catalog.list(profile("default")).await.expect("list").len(),
            1
        );
        assert_eq!(
            catalog.list(profile("research")).await.expect("list").len(),
            1
        );
        assert_eq!(catalog.list(profile("empty")).await.expect("list").len(), 0);
    }
}
