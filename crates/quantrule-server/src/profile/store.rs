//! Profile storage behind one trait.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use quantrule_api::ProfileId;

use crate::runtime::adapters::BoxFuture;

/// One stored profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileRecord {
    /// The validated id.
    pub profile_id: ProfileId,
    /// A human label.
    pub name: String,
    /// Server-assigned creation time, RFC 3339 UTC.
    pub created_at: String,
}

/// Why a profile write failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileError {
    /// The id is already taken.
    AlreadyExists,
    /// The store refused the write.
    Store(String),
}

impl std::fmt::Display for ProfileError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyExists => formatter.write_str("the profile already exists"),
            Self::Store(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for ProfileError {}

/// Where profiles live.
pub trait ProfileStore: Send + Sync {
    /// Create a profile.
    fn create(&self, record: ProfileRecord) -> BoxFuture<Result<(), ProfileError>>;

    /// Read one by id.
    fn get(&self, profile_id: ProfileId) -> BoxFuture<Result<Option<ProfileRecord>, ProfileError>>;

    /// List every profile, ordered by id.
    fn list(&self) -> BoxFuture<Result<Vec<ProfileRecord>, ProfileError>>;
}

/// An in-memory store, for tests and for a local instance.
#[derive(Debug, Clone, Default)]
pub struct MemoryProfiles {
    inner: Arc<Mutex<BTreeMap<String, ProfileRecord>>>,
}

impl ProfileStore for MemoryProfiles {
    fn create(&self, record: ProfileRecord) -> BoxFuture<Result<(), ProfileError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let mut map = inner
                .lock()
                .map_err(|_| ProfileError::Store("the profile lock is poisoned".to_owned()))?;
            if map.contains_key(record.profile_id.as_str()) {
                return Err(ProfileError::AlreadyExists);
            }
            map.insert(record.profile_id.as_str().to_owned(), record);
            Ok(())
        })
    }

    fn get(&self, profile_id: ProfileId) -> BoxFuture<Result<Option<ProfileRecord>, ProfileError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            Ok(inner
                .lock()
                .map_err(|_| ProfileError::Store("the profile lock is poisoned".to_owned()))?
                .get(profile_id.as_str())
                .cloned())
        })
    }

    fn list(&self) -> BoxFuture<Result<Vec<ProfileRecord>, ProfileError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            Ok(inner
                .lock()
                .map_err(|_| ProfileError::Store("the profile lock is poisoned".to_owned()))?
                .values()
                .cloned()
                .collect())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(id: &str) -> ProfileRecord {
        ProfileRecord {
            profile_id: ProfileId::parse(id).expect("profile"),
            name: id.to_owned(),
            created_at: "2026-11-13T00:00:00Z".to_owned(),
        }
    }

    #[tokio::test]
    async fn a_profile_round_trips() {
        let store = MemoryProfiles::default();
        let record = record("alpha");
        store.create(record.clone()).await.expect("create");
        let loaded = store.get(record.profile_id.clone()).await.expect("get");
        assert_eq!(loaded, Some(record));
    }

    #[tokio::test]
    async fn the_same_id_is_refused_twice() {
        let store = MemoryProfiles::default();
        store.create(record("alpha")).await.expect("first");
        assert_eq!(
            store.create(record("alpha")).await,
            Err(ProfileError::AlreadyExists)
        );
    }

    #[tokio::test]
    async fn the_list_is_ordered_by_id() {
        let store = MemoryProfiles::default();
        store.create(record("beta")).await.expect("beta");
        store.create(record("alpha")).await.expect("alpha");
        let listed = store.list().await.expect("list");
        let ids: Vec<&str> = listed
            .iter()
            .map(|record| record.profile_id.as_str())
            .collect();
        assert_eq!(ids, vec!["alpha", "beta"]);
    }

    #[tokio::test]
    async fn an_unknown_id_is_none() {
        let store = MemoryProfiles::default();
        assert_eq!(
            store
                .get(ProfileId::parse("missing").expect("profile"))
                .await
                .expect("get"),
            None
        );
    }
}
