//! Shared in-memory services and request helpers for the data-service tests.

#![allow(dead_code)]

use axum::body::Body;
use axum::http::Request;
use quantrule_api::{DatasetRecord, PROFILE_HEADER};
use quantrule_data_service::{DataServices, DataState, MemoryCatalog, MemoryConfigs};
use quantrule_runtime::adapters::{Backend, BoxFuture, ObjectStore, StoreError};
use quantrule_runtime::{Config, HealthRegistry};
use std::collections::BTreeMap;
use std::path::Path;
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

pub fn state(config: &Config, services: DataServices) -> Arc<DataState> {
    Arc::new(DataState::with_services(
        config,
        HealthRegistry::new(),
        services,
    ))
}

pub async fn state_with_catalog_and_objects() -> (Arc<DataState>, MemoryObjects) {
    let objects = MemoryObjects::default();
    let state = state(
        &Config::default(),
        DataServices {
            catalog: Some(Arc::new(MemoryCatalog::default())),
            objects: Some(Arc::new(objects.clone())),
            configs: None,
        },
    );
    (state, objects)
}

pub async fn state_with_catalog() -> Arc<DataState> {
    state_with_catalog_and_objects().await.0
}

pub async fn state_with_small_upload() -> Arc<DataState> {
    let config = Config {
        upload_limit_bytes: 16,
        ..Config::default()
    };
    state(
        &config,
        DataServices {
            catalog: Some(Arc::new(MemoryCatalog::default())),
            objects: Some(Arc::new(MemoryObjects::default())),
            configs: None,
        },
    )
}

pub async fn state_with_configs() -> Arc<DataState> {
    let config = Config {
        config_root: Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/config"),
        ..Config::default()
    };
    state(
        &config,
        DataServices {
            catalog: None,
            objects: None,
            configs: Some(Arc::new(MemoryConfigs::default())),
        },
    )
}

pub const CSV: &str = "date,open,high,low,close,volume\n\
2024-01-01 00:00:00+00:00,100.0,101.0,99.0,100.5,10\n\
2024-01-01 00:01:00+00:00,100.5,101.5,100.0,101.0,12\n";

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
