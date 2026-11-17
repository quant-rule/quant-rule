//! An in-memory object store for route and worker tests.
//!
//! The store implements [`ObjectStore`] over a `BTreeMap`, so a route, a
//! worker, or a repository can be exercised without S3 or a filesystem. It is
//! the object-store counterpart of [`crate::client::MemoryDataClient`]: one
//! fake per trait, in the crate that owns the trait, instead of a copy per test
//! suite.
//!
//! Keys are stored verbatim. The fake makes no attempt to canonicalize them and
//! performs no content-addressed verification; a test that needs those rules
//! exercises the filesystem adapter instead.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};

use super::{Backend, BoxFuture, ObjectStore, StoreError};

/// An object store that keeps bytes in memory.
#[derive(Debug, Clone, Default)]
pub struct MemoryObjects {
    inner: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
}

impl MemoryObjects {
    /// Every key currently stored, in map order.
    #[must_use]
    pub fn keys(&self) -> Vec<String> {
        self.lock().keys().cloned().collect()
    }

    /// Read one object, or `None`.
    #[must_use]
    pub fn read(&self, key: &str) -> Option<Vec<u8>> {
        self.lock().get(key).cloned()
    }

    /// Remove one object.
    pub fn remove(&self, key: &str) {
        self.lock().remove(key);
    }

    /// Store one object for a test that does not go through the worker.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] when the lock is poisoned.
    pub async fn store(&self, key: &str, bytes: Vec<u8>) -> Result<(), StoreError> {
        self.put(key, bytes).await
    }

    /// The map, recovering from a poisoned lock.
    ///
    /// A test that poisoned the lock already panicked; recovering keeps the
    /// helper callable from a `Drop` or a cleanup path instead of panicking a
    /// second time. The trait methods below still report poisoning as an error,
    /// which is what the real adapters do.
    fn lock(&self) -> std::sync::MutexGuard<'_, BTreeMap<String, Vec<u8>>> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl ObjectStore for MemoryObjects {
    fn backend(&self) -> Backend {
        Backend::Filesystem
    }

    fn put(&self, key: &str, bytes: Vec<u8>) -> BoxFuture<Result<(), StoreError>> {
        let inner = Arc::clone(&self.inner);
        let key = key.to_owned();
        Box::pin(async move {
            inner
                .lock()
                .map_err(|_| StoreError("the object lock is poisoned".to_owned()))?
                .insert(key, bytes);
            Ok(())
        })
    }

    fn get(&self, key: &str) -> BoxFuture<Result<Vec<u8>, StoreError>> {
        let inner = Arc::clone(&self.inner);
        let key = key.to_owned();
        Box::pin(async move {
            inner
                .lock()
                .map_err(|_| StoreError("the object lock is poisoned".to_owned()))?
                .get(&key)
                .cloned()
                .ok_or_else(|| StoreError(format!("{key} is absent")))
        })
    }

    fn exists(&self, key: &str) -> BoxFuture<Result<bool, StoreError>> {
        let inner = Arc::clone(&self.inner);
        let key = key.to_owned();
        Box::pin(async move {
            Ok(inner
                .lock()
                .map_err(|_| StoreError("the object lock is poisoned".to_owned()))?
                .contains_key(&key))
        })
    }

    fn delete(&self, key: &str) -> BoxFuture<Result<(), StoreError>> {
        let inner = Arc::clone(&self.inner);
        let key = key.to_owned();
        Box::pin(async move {
            inner
                .lock()
                .map_err(|_| StoreError("the object lock is poisoned".to_owned()))?
                .remove(&key);
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::MemoryObjects;
    use crate::adapters::ObjectStore;

    #[tokio::test]
    async fn put_get_exists_and_remove_round_trip() {
        let objects = MemoryObjects::default();
        objects.put("a", b"one".to_vec()).await.expect("put");
        assert!(objects.exists("a").await.expect("exists"));
        assert_eq!(objects.get("a").await.expect("get"), b"one");
        assert_eq!(objects.read("a"), Some(b"one".to_vec()));
        objects.remove("a");
        assert_eq!(objects.read("a"), None);
        assert!(!objects.exists("a").await.expect("exists"));
        assert!(objects.get("a").await.is_err());
    }

    #[tokio::test]
    async fn store_returns_the_write_result() {
        let objects = MemoryObjects::default();
        objects.store("b", b"two".to_vec()).await.expect("store");
        assert_eq!(objects.keys(), vec!["b".to_owned()]);
    }
}
