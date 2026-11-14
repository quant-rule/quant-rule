//! Shared in-memory services and request helpers for the evidence tests.
//!
//! Each integration target compiles this module separately, and a target need
//! not use every helper, so unused helpers are allowed here.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use quantrule_api::ProfileId;
use quantrule_runtime::adapters::{Backend, BoxFuture, ObjectStore, StoreError};

/// An object store that keeps bytes in memory, so a route is tested without S3.
#[derive(Clone, Default)]
pub struct MemoryObjects {
    inner: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
}

impl MemoryObjects {
    /// Store one object for a test that does not go through the worker.
    pub async fn store(&self, key: &str, bytes: Vec<u8>) {
        <Self as ObjectStore>::put(self, key, bytes)
            .await
            .expect("put");
    }

    /// Remove one object.
    pub fn remove(&self, key: &str) {
        self.inner.lock().expect("the object lock").remove(key);
    }

    /// Every key currently stored.
    pub fn keys(&self) -> Vec<String> {
        self.inner
            .lock()
            .expect("the object lock")
            .keys()
            .cloned()
            .collect()
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
}

/// A profile id, failing loudly on a bad fixture.
pub fn profile(name: &str) -> ProfileId {
    ProfileId::parse(name).expect("profile")
}

pub async fn body_bytes(response: axum::response::Response) -> Vec<u8> {
    axum::body::to_bytes(response.into_body(), 65_536)
        .await
        .expect("body")
        .to_vec()
}
