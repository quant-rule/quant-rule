//! The dataset catalog.
//!
//! A dataset is catalogued by its content version. The bytes themselves live in
//! the object store; the catalog row is what a client lists and what a run
//! refers to. The row carries the profile, the object does not: two profiles
//! that upload the same bytes share one object and keep one row each.
//!
//! Runtime history adds two relations here. `dataset_references` records every
//! consumer that still needs the bytes, so erase never has to read another
//! service's schema. `retention_events` is the append-only audit trail that
//! survives the dataset row. Archive and erase check both in one transaction.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use quantrule_api::{DatasetRecord, DatasetReference, ProfileId};

use quantrule_runtime::adapters::BoxFuture;

/// Why a catalog write failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogError {
    /// The same bytes are already catalogued in this profile.
    AlreadyExists,
    /// The profile holds no such dataset.
    NotFound,
    /// The dataset is already archived.
    Archived,
    /// The dataset has not been archived, and erase requires archive first.
    NotArchived,
    /// A consumer still holds this many references to the dataset.
    Referenced(u64),
    /// The store refused the write.
    Store(String),
}

impl std::fmt::Display for CatalogError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyExists => formatter.write_str("the dataset already exists"),
            Self::NotFound => formatter.write_str("the dataset is unknown"),
            Self::Archived => formatter.write_str("the dataset is already archived"),
            Self::NotArchived => formatter.write_str("archive the dataset before erasing it"),
            Self::Referenced(count) => {
                write!(formatter, "the dataset still has {count} reference(s)")
            }
            Self::Store(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for CatalogError {}

/// The outcome of erasing one profile's dataset row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EraseOutcome {
    /// The erased row. Its summary is the last recorded state.
    pub record: DatasetRecord,
    /// Whether another profile still points at the same object key.
    pub object_shared: bool,
}

/// The outcome of an idempotent dataset insert.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasetInsert {
    /// The stored row.
    pub record: DatasetRecord,
    /// Whether this call created the row.
    pub created: bool,
}

/// A database-backed guard for one content-addressed object key.
///
/// The guard serialises the object write and the row insert against the erase
/// path across replicas. It is not an object-store lock: the catalog's database
/// decides the order, and the caller releases the guard after the object and
/// row are consistent.
pub trait ObjectGuard: Send {
    /// Release the guard. A dropped guard must also be safe; releasing is the
    /// explicit path that reports a database failure.
    fn release(self: Box<Self>) -> BoxFuture<Result<(), CatalogError>>;
}

/// Where datasets are catalogued.
pub trait DatasetCatalog: Send + Sync {
    /// Add a dataset to a profile's catalog.
    fn insert(
        &self,
        profile: ProfileId,
        record: DatasetRecord,
    ) -> BoxFuture<Result<(), CatalogError>>;

    /// Add a dataset, or return the stored row for the same content.
    ///
    /// The unique `(profile_id, data_version)` key is the authority. A
    /// concurrent duplicate returns the winner's row with `created == false`;
    /// it is never an error, so two replicas can answer a retry identically.
    fn insert_or_get(
        &self,
        profile: ProfileId,
        record: DatasetRecord,
    ) -> BoxFuture<Result<DatasetInsert, CatalogError>>;

    /// Acquire the per-object guard for one object key.
    ///
    /// Upload takes it before the object write and the row insert. Erase takes
    /// it after its row delete and rechecks the object's references before
    /// deleting bytes. Two replicas therefore cannot publish a row whose bytes
    /// a concurrent erase removes.
    fn object_guard(
        &self,
        object_key: String,
    ) -> BoxFuture<Result<Box<dyn ObjectGuard>, CatalogError>>;

    /// Whether any dataset row still names this object key.
    fn object_referenced(&self, object_key: String) -> BoxFuture<Result<bool, CatalogError>>;

    /// Read one by version within a profile.
    fn get(
        &self,
        profile: ProfileId,
        version: String,
    ) -> BoxFuture<Result<Option<DatasetRecord>, CatalogError>>;

    /// List a profile's active datasets, ordered by version.
    fn list(&self, profile: ProfileId) -> BoxFuture<Result<Vec<DatasetRecord>, CatalogError>>;

    /// List active and archived datasets, ordered by version.
    fn list_including_archived(
        &self,
        profile: ProfileId,
    ) -> BoxFuture<Result<Vec<DatasetRecord>, CatalogError>>;

    /// How many active datasets a profile holds.
    fn count(&self, profile: ProfileId) -> BoxFuture<Result<u64, CatalogError>>;

    /// Archive one dataset. The bytes and the row remain.
    fn archive(
        &self,
        profile: ProfileId,
        version: String,
        archived_at: String,
    ) -> BoxFuture<Result<DatasetRecord, CatalogError>>;

    /// Erase one archived dataset whose references are all released.
    ///
    /// The row, its references, and the retention event commit together. The
    /// caller deletes the object afterwards only when `object_shared` is false;
    /// a leaked object is safe, a missing shared object is not.
    fn erase(
        &self,
        profile: ProfileId,
        version: String,
    ) -> BoxFuture<Result<EraseOutcome, CatalogError>>;

    /// Register one consumer reference. Repeated registration is idempotent.
    fn add_reference(
        &self,
        profile: ProfileId,
        version: String,
        reference: DatasetReference,
    ) -> BoxFuture<Result<u64, CatalogError>>;

    /// Release one consumer reference. Returns the remaining reference count.
    fn release_reference(
        &self,
        profile: ProfileId,
        version: String,
        reference_id: String,
    ) -> BoxFuture<Result<u64, CatalogError>>;

    /// List every consumer reference to one dataset.
    fn references(
        &self,
        profile: ProfileId,
        version: String,
    ) -> BoxFuture<Result<Vec<DatasetReference>, CatalogError>>;
}

#[derive(Debug, Default)]
struct MemoryInner {
    datasets: BTreeMap<(String, String), DatasetRecord>,
    references: BTreeMap<(String, String), BTreeMap<String, DatasetReference>>,
    events: Vec<MemoryRetentionEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MemoryRetentionEvent {
    profile_id: String,
    record_id: String,
    action: &'static str,
    bytes: u64,
}

/// An in-memory catalog, for tests and for a local instance.
#[derive(Debug, Clone, Default)]
pub struct MemoryCatalog {
    inner: Arc<Mutex<MemoryInner>>,
    object_gate: Arc<tokio::sync::Mutex<()>>,
}

/// The in-memory counterpart of a database object guard.
struct MemoryObjectGuard {
    _gate: tokio::sync::OwnedMutexGuard<()>,
}

impl ObjectGuard for MemoryObjectGuard {
    fn release(self: Box<Self>) -> BoxFuture<Result<(), CatalogError>> {
        Box::pin(async move {
            drop(self);
            Ok(())
        })
    }
}

impl MemoryCatalog {
    /// The retention events recorded for one profile, in insertion order.
    ///
    /// This accessor exists so a fast test can prove the tombstone without a
    /// database. The production stores keep the same rows in
    /// `catalog.retention_events`.
    #[must_use]
    pub fn retention_events(&self, profile: ProfileId) -> Vec<(String, &'static str, u64)> {
        self.inner
            .lock()
            .expect("the catalog lock")
            .events
            .iter()
            .filter(|event| event.profile_id == profile.as_str())
            .map(|event| (event.record_id.clone(), event.action, event.bytes))
            .collect()
    }
}

fn reference_count(
    references: &BTreeMap<(String, String), BTreeMap<String, DatasetReference>>,
    profile: &str,
    version: &str,
) -> u64 {
    references
        .get(&(profile.to_owned(), version.to_owned()))
        .map_or(0, |entries| entries.len() as u64)
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
            if map.datasets.contains_key(&key) {
                return Err(CatalogError::AlreadyExists);
            }
            map.datasets.insert(key, record);
            Ok(())
        })
    }

    fn insert_or_get(
        &self,
        profile: ProfileId,
        record: DatasetRecord,
    ) -> BoxFuture<Result<DatasetInsert, CatalogError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let mut map = inner
                .lock()
                .map_err(|_| CatalogError::Store("the catalog lock is poisoned".to_owned()))?;
            let key = (
                profile.as_str().to_owned(),
                record.summary.data_version.to_string(),
            );
            if let Some(existing) = map.datasets.get(&key) {
                return Ok(DatasetInsert {
                    record: existing.clone(),
                    created: false,
                });
            }
            map.datasets.insert(key, record.clone());
            Ok(DatasetInsert {
                record,
                created: true,
            })
        })
    }

    fn object_guard(
        &self,
        _object_key: String,
    ) -> BoxFuture<Result<Box<dyn ObjectGuard>, CatalogError>> {
        let gate = Arc::clone(&self.object_gate);
        Box::pin(async move {
            let guard = gate.lock_owned().await;
            Ok(Box::new(MemoryObjectGuard { _gate: guard }) as Box<dyn ObjectGuard>)
        })
    }

    fn object_referenced(&self, object_key: String) -> BoxFuture<Result<bool, CatalogError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            Ok(inner
                .lock()
                .map_err(|_| CatalogError::Store("the catalog lock is poisoned".to_owned()))?
                .datasets
                .values()
                .any(|record| record.object_key == object_key))
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
                .datasets
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
                .datasets
                .iter()
                .filter(|((owner, _), record)| {
                    owner == profile.as_str() && record.summary.archived_at.is_none()
                })
                .map(|(_, record)| record.clone())
                .collect())
        })
    }

    fn list_including_archived(
        &self,
        profile: ProfileId,
    ) -> BoxFuture<Result<Vec<DatasetRecord>, CatalogError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let map = inner
                .lock()
                .map_err(|_| CatalogError::Store("the catalog lock is poisoned".to_owned()))?;
            Ok(map
                .datasets
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
                .datasets
                .iter()
                .filter(|((owner, _), record)| {
                    owner == profile.as_str() && record.summary.archived_at.is_none()
                })
                .count() as u64)
        })
    }

    fn archive(
        &self,
        profile: ProfileId,
        version: String,
        archived_at: String,
    ) -> BoxFuture<Result<DatasetRecord, CatalogError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let mut map = inner
                .lock()
                .map_err(|_| CatalogError::Store("the catalog lock is poisoned".to_owned()))?;
            let key = (profile.as_str().to_owned(), version);
            let Some(record) = map.datasets.get_mut(&key) else {
                return Err(CatalogError::NotFound);
            };
            if record.summary.archived_at.is_some() {
                return Err(CatalogError::Archived);
            }
            record.summary.archived_at = Some(archived_at);
            let record = record.clone();
            map.events.push(MemoryRetentionEvent {
                profile_id: key.0,
                record_id: key.1,
                action: "archive",
                bytes: record.summary.bytes,
            });
            Ok(record)
        })
    }

    fn erase(
        &self,
        profile: ProfileId,
        version: String,
    ) -> BoxFuture<Result<EraseOutcome, CatalogError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let mut map = inner
                .lock()
                .map_err(|_| CatalogError::Store("the catalog lock is poisoned".to_owned()))?;
            let key = (profile.as_str().to_owned(), version);
            let Some(record) = map.datasets.get(&key) else {
                return Err(CatalogError::NotFound);
            };
            if record.summary.archived_at.is_none() {
                return Err(CatalogError::NotArchived);
            }
            let count = reference_count(&map.references, &key.0, &key.1);
            if count > 0 {
                return Err(CatalogError::Referenced(count));
            }
            let record = map.datasets.remove(&key).ok_or(CatalogError::NotFound)?;
            map.references.remove(&key);
            let object_shared = map
                .datasets
                .values()
                .any(|other| other.object_key == record.object_key);
            map.events.push(MemoryRetentionEvent {
                profile_id: key.0,
                record_id: key.1,
                action: "erase",
                bytes: record.summary.bytes,
            });
            Ok(EraseOutcome {
                record,
                object_shared,
            })
        })
    }

    fn add_reference(
        &self,
        profile: ProfileId,
        version: String,
        reference: DatasetReference,
    ) -> BoxFuture<Result<u64, CatalogError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let mut map = inner
                .lock()
                .map_err(|_| CatalogError::Store("the catalog lock is poisoned".to_owned()))?;
            let key = (profile.as_str().to_owned(), version.clone());
            if !map.datasets.contains_key(&key) {
                return Err(CatalogError::NotFound);
            }
            let profile_id = profile.as_str().to_owned();
            map.references
                .entry(key)
                .or_default()
                .insert(reference.reference_id.clone(), reference);
            Ok(reference_count(&map.references, &profile_id, &version))
        })
    }

    fn release_reference(
        &self,
        profile: ProfileId,
        version: String,
        reference_id: String,
    ) -> BoxFuture<Result<u64, CatalogError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let mut map = inner
                .lock()
                .map_err(|_| CatalogError::Store("the catalog lock is poisoned".to_owned()))?;
            let key = (profile.as_str().to_owned(), version.clone());
            if !map.datasets.contains_key(&key) {
                return Err(CatalogError::NotFound);
            }
            if let Some(entries) = map.references.get_mut(&key) {
                entries.remove(&reference_id);
            }
            Ok(reference_count(&map.references, profile.as_str(), &version))
        })
    }

    fn references(
        &self,
        profile: ProfileId,
        version: String,
    ) -> BoxFuture<Result<Vec<DatasetReference>, CatalogError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let map = inner
                .lock()
                .map_err(|_| CatalogError::Store("the catalog lock is poisoned".to_owned()))?;
            let key = (profile.as_str().to_owned(), version);
            if !map.datasets.contains_key(&key) {
                return Err(CatalogError::NotFound);
            }
            Ok(map
                .references
                .get(&key)
                .map(|entries| entries.values().cloned().collect())
                .unwrap_or_default())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quantrule_api::{DatasetReferenceKind, DatasetSummary, Digest};

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
                archived_at: None,
            },
            object_key: format!("datasets/{}", version.as_hex()),
            sha256: version,
            quality: None,
            created_at: "2026-11-13T00:00:00Z".to_owned(),
        }
    }

    fn reference(id: &str) -> DatasetReference {
        DatasetReference {
            reference_id: id.to_owned(),
            kind: DatasetReferenceKind::Run,
            created_at: "2026-11-20T00:00:00Z".to_owned(),
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

    #[tokio::test]
    async fn archive_hides_a_dataset_and_keeps_the_bytes() {
        let catalog = MemoryCatalog::default();
        let record = record(b"archive");
        let version = record.summary.data_version.to_string();
        catalog
            .insert(profile("default"), record.clone())
            .await
            .expect("insert");
        let archived = catalog
            .archive(
                profile("default"),
                version.clone(),
                "2026-11-20T00:00:00Z".to_owned(),
            )
            .await
            .expect("archive");
        assert_eq!(
            archived.summary.archived_at.as_deref(),
            Some("2026-11-20T00:00:00Z")
        );
        assert!(
            catalog
                .list(profile("default"))
                .await
                .expect("list")
                .is_empty()
        );
        assert_eq!(
            catalog
                .list_including_archived(profile("default"))
                .await
                .expect("history")
                .len(),
            1
        );
        assert_eq!(catalog.count(profile("default")).await.expect("count"), 0);
        assert_eq!(
            catalog.retention_events(profile("default")),
            vec![(version, "archive", 100)]
        );
    }

    #[tokio::test]
    async fn erase_requires_archive_and_no_references() {
        let catalog = MemoryCatalog::default();
        let record = record(b"erase");
        let version = record.summary.data_version.to_string();
        catalog
            .insert(profile("default"), record)
            .await
            .expect("insert");
        assert_eq!(
            catalog.erase(profile("default"), version.clone()).await,
            Err(CatalogError::NotArchived)
        );
        catalog
            .archive(
                profile("default"),
                version.clone(),
                "2026-11-20T00:00:00Z".to_owned(),
            )
            .await
            .expect("archive");
        assert_eq!(
            catalog
                .add_reference(profile("default"), version.clone(), reference("run-1"))
                .await
                .expect("reference"),
            1
        );
        assert_eq!(
            catalog.erase(profile("default"), version.clone()).await,
            Err(CatalogError::Referenced(1))
        );
        assert_eq!(
            catalog
                .release_reference(profile("default"), version.clone(), "run-1".to_owned())
                .await
                .expect("release"),
            0
        );
        let outcome = catalog
            .erase(profile("default"), version.clone())
            .await
            .expect("erase");
        assert!(!outcome.object_shared);
        assert_eq!(
            catalog.get(profile("default"), version).await.expect("get"),
            None
        );
    }

    #[tokio::test]
    async fn an_object_shared_by_another_profile_is_not_deleted() {
        let catalog = MemoryCatalog::default();
        let record = record(b"shared-erase");
        let version = record.summary.data_version.to_string();
        catalog
            .insert(profile("default"), record.clone())
            .await
            .expect("default");
        catalog
            .insert(profile("research"), record)
            .await
            .expect("research");
        catalog
            .archive(
                profile("default"),
                version.clone(),
                "2026-11-20T00:00:00Z".to_owned(),
            )
            .await
            .expect("archive");
        let outcome = catalog
            .erase(profile("default"), version)
            .await
            .expect("erase");
        assert!(outcome.object_shared);
    }

    #[tokio::test]
    async fn registering_a_reference_twice_keeps_one_row() {
        let catalog = MemoryCatalog::default();
        let record = record(b"idempotent");
        let version = record.summary.data_version.to_string();
        catalog
            .insert(profile("default"), record)
            .await
            .expect("insert");
        for _ in 0..2 {
            assert_eq!(
                catalog
                    .add_reference(profile("default"), version.clone(), reference("run-1"))
                    .await
                    .expect("reference"),
                1
            );
        }
        assert_eq!(
            catalog
                .references(profile("default"), version)
                .await
                .expect("references")
                .len(),
            1
        );
    }
}
