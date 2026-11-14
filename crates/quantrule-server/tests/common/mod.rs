//! Shared in-memory services and request helpers for the gateway tests.
//!
//! Each integration target compiles this module separately, and a target need
//! not use every helper, so unused helpers are allowed here.

#![allow(dead_code)]

//! The gateway routes, exercised over HTTP with in-memory services.
//!
//! The tests assert on status, envelope code, and body, not on a handler's
//! internals, so a route change is visible the way a client would see it.

use axum::body::Body;
use axum::http::Request;
use quantrule_api::{DatasetRecord, PROFILE_HEADER, ProfileId};
use quantrule_research::{CostModel, Partition, Split};
use quantrule_server::data::MemoryCatalog;
use quantrule_server::profile::{MemoryProfiles, ProfileRecord, ProfileStore};
use quantrule_server::research::MemoryStore;
use quantrule_server::runtime::adapters::{Backend, BoxFuture, ObjectStore, StoreError};
use quantrule_server::runtime::{AppState, Config, HealthRegistry, Services};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

/// An object store that keeps bytes in memory, so the route is tested
/// without S3.
#[derive(Clone, Default)]
pub struct MemoryObjects {
    inner: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
}

impl MemoryObjects {
    /// Used by the gateway target's integrity test; the ladder target does not
    /// need it.
    #[allow(dead_code)]
    pub fn remove(&self, key: &str) {
        self.inner.lock().expect("the object lock").remove(key);
    }

    /// Every key currently stored; the worker's partial-publish test needs to
    /// prove that no run prefix exists.
    #[allow(dead_code)]
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

pub const CSV: &str = "date,open,high,low,close,volume\n\
2024-01-01 00:00:00+00:00,100.0,101.0,99.0,100.5,10\n\
2024-01-01 00:01:00+00:00,100.5,101.5,100.0,101.0,12\n";

pub fn state() -> Arc<AppState> {
    Arc::new(AppState::new(&Config::default(), HealthRegistry::new()))
}

pub async fn state_with_store() -> Arc<AppState> {
    Arc::new(AppState::with_services(
        &Config::default(),
        HealthRegistry::new(),
        Services {
            hypotheses: Some(Arc::new(MemoryStore::default())),
            catalog: None,
            objects: None,
            profiles: Some(memory_profiles(&["default", "alpha", "beta"]).await),
            configs: None,
            runs: None,
            queue: None,
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

pub async fn state_with_catalog_and_objects() -> (Arc<AppState>, MemoryObjects) {
    let objects = MemoryObjects::default();
    let state = Arc::new(AppState::with_services(
        &Config::default(),
        HealthRegistry::new(),
        Services {
            hypotheses: None,
            catalog: Some(Arc::new(MemoryCatalog::default())),
            objects: Some(Arc::new(objects.clone())),
            profiles: Some(memory_profiles(&["default", "alpha", "beta", "gamma"]).await),
            configs: None,
            runs: None,
            queue: None,
        },
    ));
    (state, objects)
}

pub async fn state_with_catalog() -> Arc<AppState> {
    state_with_catalog_and_objects().await.0
}

pub async fn state_with_small_upload() -> Arc<AppState> {
    let config = Config {
        upload_limit_bytes: 16,
        ..Config::default()
    };
    Arc::new(AppState::with_services(
        &config,
        HealthRegistry::new(),
        Services {
            hypotheses: None,
            catalog: Some(Arc::new(MemoryCatalog::default())),
            objects: Some(Arc::new(MemoryObjects::default())),
            profiles: Some(memory_profiles(&["default"]).await),
            configs: None,
            runs: None,
            queue: None,
        },
    ))
}

pub fn body() -> serde_json::Value {
    let split = Split::new(
        Partition::new("dev", 100, 200).expect("dev"),
        Partition::new("val", 200, 300).expect("val"),
    )
    .expect("split");
    serde_json::json!({
        "premise": "a retest holds",
        "rule": "strategy/retest.toml",
        "dataset": "sha256:abc",
        "costs": CostModel::new(1.0, 0.0, 0.0).expect("costs"),
        "split": split,
        "criteria": "expectancy above zero",
    })
}

pub fn upload(name: &str) -> Request<Body> {
    Request::post(format!("/catalog/datasets?name={name}"))
        .header(PROFILE_HEADER, "default")
        .body(Body::from(CSV))
        .expect("request")
}

pub fn read_dataset(bytes: &[u8]) -> DatasetRecord {
    serde_json::from_slice(bytes).expect("dataset record")
}

pub async fn body_bytes(response: axum::response::Response) -> Vec<u8> {
    axum::body::to_bytes(response.into_body(), 65_536)
        .await
        .expect("body")
        .to_vec()
}

pub fn minutes(count: usize) -> String {
    let mut text = String::from("date,open,high,low,close,volume\n");
    for offset in 0..count {
        let open = 100.0 + offset as f64;
        text.push_str(&format!(
            "2024-01-01 00:{offset:02}:00+00:00,{open},{},{},{},10\n",
            open + 1.0,
            open - 1.0,
            open + 0.5
        ));
    }
    text
}
