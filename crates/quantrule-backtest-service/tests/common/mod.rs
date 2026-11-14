//! Shared fixtures and fakes for the backtest worker tests.
//!
//! The worker tests build their objects in memory and canonicalize a small CSV
//! the way ingest does, so no database, object store, or broker is required.

#![allow(dead_code)]

use quantrule_runtime::adapters::{Backend, BoxFuture, ObjectStore, StoreError};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

/// An object store that keeps bytes in memory, so a worker write is tested
/// without S3.
#[derive(Clone, Default)]
pub struct MemoryObjects {
    inner: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
}

impl MemoryObjects {
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

/// Canonicalize a date-and-price CSV for a test, the way ingest does.
pub fn canonical(csv: &str) -> Vec<u8> {
    use quantrule_data::{Candle, RawCandle};

    let mut out = Vec::new();
    let mut index = 0_u64;
    for line in csv.lines() {
        let fields: Vec<&str> = line.split(',').map(str::trim).collect();
        if fields.len() < 5 || !fields[0].starts_with(|c: char| c.is_ascii_digit()) {
            continue;
        }
        let raw = RawCandle {
            date: fields[0],
            open: fields[1],
            high: fields[2],
            low: fields[3],
            close: fields[4],
            volume: fields.get(5).copied(),
        };
        let candle = Candle::parse(index, raw).expect("test candle");
        out.extend_from_slice(candle.canonical_line().as_bytes());
        out.push(b'\n');
        index += 1;
    }
    out
}
