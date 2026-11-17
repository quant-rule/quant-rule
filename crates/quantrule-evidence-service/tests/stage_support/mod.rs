//! Shared harness for the evidence stage route tests.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use crate::common::body_bytes;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use quantrule_api::{ArtifactRef, Digest, PROFILE_HEADER, StageArtifact, StageSubmitResponse};
use quantrule_evidence_service::runs::MemoryRuns;
use quantrule_evidence_service::stage::{MemoryStages, StageStore};
use quantrule_evidence_service::{EvidenceServices, EvidenceState, router};
use quantrule_runtime::adapters::memory::MemoryObjects;
use quantrule_runtime::adapters::{Backend, BoxFuture, ObjectStore, Queue, QueueError};
use quantrule_runtime::client::MemoryDataClient;
use quantrule_runtime::{Config, HealthRegistry};
use quantrule_service_api::{ConfigBundle, DatasetObject, StageManifest};
use tower::ServiceExt;

type Messages = Arc<Mutex<Vec<(String, Vec<u8>)>>>;

#[derive(Clone, Default)]
pub(crate) struct MemoryQueue {
    inner: Messages,
}

impl Queue for MemoryQueue {
    fn backend(&self) -> Backend {
        Backend::InProcess
    }

    fn publish(&self, key: &str, payload: Vec<u8>) -> BoxFuture<Result<(), QueueError>> {
        let inner = Arc::clone(&self.inner);
        let key = key.to_owned();
        Box::pin(async move {
            inner
                .lock()
                .map_err(|_| QueueError("the queue lock is poisoned".to_owned()))?
                .push((key, payload));
            Ok(())
        })
    }
}

impl MemoryQueue {
    pub(crate) fn messages(&self) -> Vec<(String, Vec<u8>)> {
        self.inner.lock().expect("the queue lock").clone()
    }
}

pub(crate) fn digest(seed: &[u8]) -> Digest {
    Digest::sha256(seed)
}

fn dataset(version: &Digest) -> DatasetObject {
    DatasetObject {
        data_version: version.clone(),
        object_key: format!("datasets/{}", version.as_hex()),
        sha256: version.clone(),
        candles: 2,
        first_bar: 1_704_067_200,
        last_bar: 1_704_067_260,
        bytes: 100,
    }
}

fn bundle(config_id: &str, hash_seed: &[u8], model: &str) -> ConfigBundle {
    ConfigBundle {
        config_id: config_id.to_owned(),
        files: BTreeMap::from([(
            "run.json".to_owned(),
            format!(r#"{{"model_id":"{model}"}}"#),
        )]),
        valid: true,
        message: None,
        config_hash: Some(digest(hash_seed)),
    }
}

fn invalid_bundle() -> ConfigBundle {
    ConfigBundle {
        config_id: "cfg_invalid".to_owned(),
        files: BTreeMap::from([("run.json".to_owned(), "{}".to_owned())]),
        valid: false,
        message: Some("the bundle is invalid".to_owned()),
        config_hash: None,
    }
}

pub(crate) struct Harness {
    pub(crate) state: Arc<EvidenceState>,
    pub(crate) queue: MemoryQueue,
    pub(crate) stages: MemoryStages,
    pub(crate) objects: MemoryObjects,
}

pub(crate) fn harness() -> Harness {
    let queue = MemoryQueue::default();
    let stages = MemoryStages::default();
    let objects = MemoryObjects::default();
    let data = MemoryDataClient::default();
    let default = quantrule_api::ProfileId::parse("default").expect("default");
    let alpha = quantrule_api::ProfileId::parse("alpha").expect("alpha");
    for profile in [&default, &alpha] {
        for version in [digest(b"candles-a"), digest(b"candles-b")] {
            data.add_dataset(profile, dataset(&version));
        }
        data.add_config(profile, bundle("cfg_valid", b"valid", "M1.3"));
        data.add_config(profile, bundle("cfg_m99", b"m99", "M9.9"));
        data.add_config(profile, invalid_bundle());
    }
    let config = Config {
        image_digest: Some(Digest::sha256(b"no-image-digest")),
        ..Config::default()
    };
    let state = Arc::new(EvidenceState::with_services(
        &config,
        HealthRegistry::new(),
        EvidenceServices {
            data: Some(Arc::new(data)),
            runs: Some(Arc::new(MemoryRuns::default())),
            stages: Some(Arc::new(stages.clone())),
            queue: Some(Arc::new(queue.clone())),
            objects: Some(Arc::new(objects.clone())),
            ..EvidenceServices::default()
        },
    ));
    Harness {
        state,
        queue,
        stages,
        objects,
    }
}

pub(crate) fn model_body(data_version: &Digest) -> serde_json::Value {
    serde_json::json!({
        "kind": "model",
        "config_id": "cfg_valid",
        "input": { "kind": "data_version", "data_version": data_version },
    })
}

pub(crate) async fn submit(
    state: Arc<EvidenceState>,
    profile: &str,
    kind: &str,
    body: &serde_json::Value,
) -> axum::response::Response {
    router(state)
        .oneshot(
            Request::post(format!("/stages/{kind}/runs"))
                .header(PROFILE_HEADER, profile)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .expect("request"),
        )
        .await
        .expect("response")
}

pub(crate) async fn json<T: serde::de::DeserializeOwned>(response: axum::response::Response) -> T {
    serde_json::from_slice(&body_bytes(response).await).expect("json")
}

/// Submit, claim, and complete one stage with the given files.
pub(crate) async fn completed_stage(
    harness: &Harness,
    files: &[(&str, &[u8])],
) -> (String, StageArtifact) {
    completed_stage_with(harness, "cfg_valid", files).await
}

pub(crate) async fn completed_stage_with(
    harness: &Harness,
    config_id: &str,
    files: &[(&str, &[u8])],
) -> (String, StageArtifact) {
    let data_version = digest(b"candles-a");
    let mut body = model_body(&data_version);
    body["config_id"] = serde_json::json!(config_id);
    let response = submit(Arc::clone(&harness.state), "default", "model", &body).await;
    let created: StageSubmitResponse = json(response).await;
    let id = created.stage_run_id.to_string();
    harness
        .stages
        .claim(id.clone(), false)
        .await
        .expect("claim");
    let artifact = StageArtifact::from_files(
        files
            .iter()
            .map(|(name, bytes)| ArtifactRef {
                name: (*name).to_owned(),
                bytes: bytes.len() as u64,
                sha256: Digest::sha256(bytes),
            })
            .collect(),
    )
    .expect("artifact");
    for (name, bytes) in files {
        assert!(!name.is_empty());
        let file = artifact
            .files
            .iter()
            .find(|file| file.name == *name)
            .expect("file");
        harness
            .objects
            .put(&StageManifest::blob_key(&file.sha256), bytes.to_vec())
            .await
            .expect("blob");
    }
    harness
        .stages
        .complete(id.clone(), artifact.clone())
        .await
        .expect("complete");
    (id, artifact)
}

/// Submit, claim, and complete one strategy stage with a small file set.
pub(crate) async fn completed_strategy_stage(
    harness: &Harness,
    model_artifact: &Digest,
    config_id: &str,
) -> (String, StageArtifact) {
    let body = serde_json::json!({
        "kind": "strategy",
        "config_id": config_id,
        "input": {
            "kind": "model_artifact",
            "model_artifact_id": model_artifact,
        },
    });
    let response = submit(Arc::clone(&harness.state), "default", "strategy", &body).await;
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let created: StageSubmitResponse = json(response).await;
    let id = created.stage_run_id.to_string();
    harness
        .stages
        .claim(id.clone(), false)
        .await
        .expect("claim");
    let artifact = StageArtifact::from_files(vec![
        ArtifactRef {
            name: "decisions.ndjson".to_owned(),
            bytes: 1,
            sha256: Digest::sha256(b"decisions"),
        },
        ArtifactRef {
            name: "provenance.json".to_owned(),
            bytes: 1,
            sha256: Digest::sha256(b"provenance"),
        },
        ArtifactRef {
            name: "summary.txt".to_owned(),
            bytes: 1,
            sha256: Digest::sha256(b"summary"),
        },
    ])
    .expect("the strategy artifact");
    harness
        .stages
        .complete(id.clone(), artifact.clone())
        .await
        .expect("complete");
    (id, artifact)
}
