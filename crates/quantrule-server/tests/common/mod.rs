//! Shared in-memory services and request helpers for the gateway tests.
//!
//! Each integration target compiles this module separately, and a target need
//! not use every helper, so unused helpers are allowed here.

#![allow(dead_code)]

use axum::body::Body;
use axum::http::Request;
use quantrule_api::{PROFILE_HEADER, ProfileId};
use quantrule_runtime::adapters::{Backend, BoxFuture, ObjectStore, StoreError};
use quantrule_runtime::client::MemoryDataClient;
use quantrule_runtime::{Config, HealthRegistry};
use quantrule_server::profile::{MemoryProfiles, ProfileRecord, ProfileStore};
use quantrule_server::server::{AppState, Services};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

/// An object store that keeps bytes in memory, so a route is tested without S3.
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

pub fn state() -> Arc<AppState> {
    Arc::new(AppState::new(&Config::default(), HealthRegistry::new()))
}

pub async fn state_with_data(data: MemoryDataClient) -> Arc<AppState> {
    Arc::new(AppState::with_services(
        &Config::default(),
        HealthRegistry::new(),
        Services {
            profiles: Some(memory_profiles(&["default", "alpha", "beta", "gamma"]).await),
            data: Some(Arc::new(data)),
            ..Services::default()
        },
    ))
}

pub async fn memory_profiles(ids: &[&str]) -> Arc<MemoryProfiles> {
    let store = Arc::new(MemoryProfiles::default());
    for id in ids {
        store
            .create(ProfileRecord {
                profile_id: ProfileId::parse(id).expect("profile"),
                name: (*id).to_owned(),
                created_at: "2026-11-13T00:00:00Z".to_owned(),
            })
            .await
            .expect("create profile");
    }
    store
}

pub async fn body_bytes(response: axum::response::Response) -> Vec<u8> {
    axum::body::to_bytes(response.into_body(), 65_536)
        .await
        .expect("body")
        .to_vec()
}

/// A request builder the tests share for the profile header.
pub fn with_profile(request: Request<Body>, profile: &str) -> Request<Body> {
    let (mut parts, body) = request.into_parts();
    parts
        .headers
        .insert(PROFILE_HEADER, profile.parse().expect("header"));
    Request::from_parts(parts, body)
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
