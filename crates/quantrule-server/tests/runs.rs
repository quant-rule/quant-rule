//! Run submission, listing, detail, sharing, and cancellation.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{body_bytes, memory_profiles};
use quantrule_api::{Digest, PROFILE_HEADER, RunDetail, RunState, RunSummary, SubmitRunResponse};
use quantrule_runtime::adapters::{Backend, BoxFuture, Queue, QueueError};
use quantrule_runtime::client::MemoryDataClient;
use quantrule_runtime::{Config, HealthRegistry};
use quantrule_server::runs::MemoryRuns;
use quantrule_server::server::router;
use quantrule_server::server::{AppState, Services};
use quantrule_service_api::{ConfigBundle, DatasetObject};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

type Messages = Arc<Mutex<Vec<(String, Vec<u8>)>>>;

#[derive(Clone, Default)]
struct MemoryQueue {
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
    fn messages(&self) -> Vec<(String, Vec<u8>)> {
        self.inner.lock().expect("the queue lock").clone()
    }
}

fn digest(seed: &[u8]) -> Digest {
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

async fn harness() -> (Arc<AppState>, MemoryQueue) {
    let queue = MemoryQueue::default();
    let data = MemoryDataClient::default();
    let default = quantrule_api::ProfileId::parse("default").expect("default");
    let alpha = quantrule_api::ProfileId::parse("alpha").expect("alpha");
    for profile in [&default, &alpha] {
        for version in [digest(b"candles-a"), digest(b"candles-b")] {
            data.add_dataset(profile, dataset(&version));
        }
        data.add_config(profile, bundle("cfg_valid", b"valid", "M1.0"));
        data.add_config(profile, bundle("cfg_changed", b"changed", "M1.0"));
        data.add_config(
            profile,
            ConfigBundle {
                config_id: "cfg_invalid".to_owned(),
                files: BTreeMap::from([("run.json".to_owned(), "{}".to_owned())]),
                valid: false,
                message: Some("the bundle is invalid".to_owned()),
                config_hash: None,
            },
        );
    }
    let state = Arc::new(AppState::with_services(
        &Config::default(),
        HealthRegistry::new(),
        Services {
            profiles: Some(memory_profiles(&["default", "alpha"]).await),
            data: Some(Arc::new(data)),
            runs: Some(Arc::new(MemoryRuns::default())),
            queue: Some(Arc::new(queue.clone())),
            ..Services::default()
        },
    ));
    (state, queue)
}

fn seed() -> (Digest, String, String) {
    (
        digest(b"candles-a"),
        "cfg_valid".to_owned(),
        "cfg_invalid".to_owned(),
    )
}

async fn submit(
    state: Arc<AppState>,
    profile: &str,
    data_version: &Digest,
    config_id: &str,
) -> axum::response::Response {
    router(state)
        .oneshot(
            Request::post("/runs")
                .header(PROFILE_HEADER, profile)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "data_version": data_version,
                        "config_id": config_id,
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response")
}

async fn list(state: Arc<AppState>, profile: &str) -> Vec<RunSummary> {
    let response = router(state)
        .oneshot(
            Request::get("/runs")
                .header(PROFILE_HEADER, profile)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    serde_json::from_slice(&body_bytes(response).await).expect("runs")
}

#[tokio::test]
async fn a_submission_is_content_addressed_and_listed() {
    let (state, queue) = harness().await;
    let (data_version, config_id, _) = seed();

    let response = submit(Arc::clone(&state), "default", &data_version, &config_id).await;
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let created: SubmitRunResponse =
        serde_json::from_slice(&body_bytes(response).await).expect("submission");
    assert!(created.created);
    assert_eq!(created.status, RunState::Queued);

    let response = submit(Arc::clone(&state), "default", &data_version, &config_id).await;
    assert_eq!(response.status(), StatusCode::OK);
    let existing: SubmitRunResponse =
        serde_json::from_slice(&body_bytes(response).await).expect("submission");
    assert!(!existing.created);
    assert_eq!(created.run_id, existing.run_id);

    let runs = list(Arc::clone(&state), "default").await;
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].run_id, created.run_id);
    assert_eq!(runs[0].status, RunState::Queued);

    let messages = queue.messages();
    assert_eq!(messages.len(), 2);
    assert!(messages.iter().all(|(key, _)| key == "run.requested"));
    let payload: serde_json::Value = serde_json::from_slice(&messages[0].1).expect("payload");
    assert_eq!(
        payload["run_id"].as_str(),
        Some(created.run_id.to_string().as_str())
    );
    assert_eq!(
        payload.as_object().map(serde_json::Map::len),
        Some(1),
        "a queued message carries the run id only"
    );

    let response = router(state)
        .oneshot(
            Request::get(format!("/runs/{}", created.run_id))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let detail: RunDetail = serde_json::from_slice(&body_bytes(response).await).expect("detail");
    assert_eq!(detail.status, RunState::Queued);
    assert_eq!(detail.stages.len(), 4);
    assert_eq!(detail.provenance.model_version, "M1.0");
    assert_eq!(detail.provenance.data_version, data_version);
}

#[tokio::test]
async fn a_changed_dataset_gets_a_new_run() {
    let (state, _queue) = harness().await;
    let first_version = digest(b"candles-a");
    let second_version = digest(b"candles-b");
    assert_ne!(first_version, second_version);
    let config_id = "cfg_valid";

    let first = submit(Arc::clone(&state), "default", &first_version, config_id).await;
    let first: SubmitRunResponse = serde_json::from_slice(&body_bytes(first).await).expect("first");
    let second = submit(Arc::clone(&state), "default", &second_version, config_id).await;
    let second: SubmitRunResponse =
        serde_json::from_slice(&body_bytes(second).await).expect("second");
    assert_ne!(first.run_id, second.run_id);
    assert_eq!(list(state, "default").await.len(), 2);
}

#[tokio::test]
async fn a_changed_config_gets_a_new_run() {
    let (state, _queue) = harness().await;
    let dataset = digest(b"candles-a");
    let original = "cfg_valid";
    let changed = "cfg_changed";

    let first = submit(Arc::clone(&state), "default", &dataset, original).await;
    assert_eq!(first.status(), StatusCode::ACCEPTED);
    let first: SubmitRunResponse = serde_json::from_slice(&body_bytes(first).await).expect("first");
    let second = submit(Arc::clone(&state), "default", &dataset, changed).await;
    assert_eq!(second.status(), StatusCode::ACCEPTED);
    let second: SubmitRunResponse =
        serde_json::from_slice(&body_bytes(second).await).expect("second");
    assert_ne!(first.run_id, second.run_id);
    assert_eq!(list(state, "default").await.len(), 2);
}

#[tokio::test]
async fn an_invalid_config_creates_nothing() {
    let (state, queue) = harness().await;
    let (data_version, _, invalid) = seed();
    let response = submit(Arc::clone(&state), "default", &data_version, &invalid).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("config.invalid"), "{text}");
    assert!(list(state, "default").await.is_empty());
    assert!(queue.messages().is_empty());
}

#[tokio::test]
async fn an_unknown_dataset_is_not_found() {
    let (state, _queue) = harness().await;
    let (_, config_id, _) = seed();
    let response = submit(
        Arc::clone(&state),
        "default",
        &Digest::sha256(b"missing"),
        &config_id,
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("dataset.not_found"), "{text}");
    assert!(list(state, "default").await.is_empty());
}

#[tokio::test]
async fn two_profiles_share_one_run() {
    let (state, _queue) = harness().await;
    let dataset = digest(b"candles-a");
    let config_id = "cfg_valid";

    let first = submit(Arc::clone(&state), "default", &dataset, config_id).await;
    assert_eq!(first.status(), StatusCode::ACCEPTED);
    let first: SubmitRunResponse = serde_json::from_slice(&body_bytes(first).await).expect("first");

    let second = submit(Arc::clone(&state), "alpha", &dataset, config_id).await;
    assert_eq!(second.status(), StatusCode::OK);
    let second: SubmitRunResponse =
        serde_json::from_slice(&body_bytes(second).await).expect("second");
    assert_eq!(first.run_id, second.run_id);

    assert_eq!(list(Arc::clone(&state), "default").await.len(), 1);
    assert_eq!(list(state, "alpha").await.len(), 1);
}

#[tokio::test]
async fn a_profile_that_did_not_submit_cannot_read_the_run() {
    let (state, _queue) = harness().await;
    let (data_version, config_id, _) = seed();
    let response = submit(Arc::clone(&state), "default", &data_version, &config_id).await;
    let created: SubmitRunResponse =
        serde_json::from_slice(&body_bytes(response).await).expect("submission");

    let response = router(state)
        .oneshot(
            Request::get(format!("/runs/{}", created.run_id))
                .header(PROFILE_HEADER, "alpha")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("run.not_found"), "{text}");
}

#[tokio::test]
async fn a_run_can_be_cancelled_once() {
    let (state, queue) = harness().await;
    let (data_version, config_id, _) = seed();
    let response = submit(Arc::clone(&state), "default", &data_version, &config_id).await;
    let created: SubmitRunResponse =
        serde_json::from_slice(&body_bytes(response).await).expect("submission");

    let response = router(Arc::clone(&state))
        .oneshot(
            Request::post(format!("/runs/{}/cancel", created.run_id))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let detail: RunDetail = serde_json::from_slice(&body_bytes(response).await).expect("detail");
    assert_eq!(detail.status, RunState::Cancelled);
    let messages = queue.messages();
    let (key, payload) = messages
        .iter()
        .find(|(key, _)| key == "run.cancelled")
        .expect("cancel message");
    assert_eq!(key, "run.cancelled");
    let payload: serde_json::Value = serde_json::from_slice(payload).expect("payload");
    assert_eq!(
        payload,
        serde_json::json!({ "run_id": created.run_id.to_string() })
    );

    let response = router(state)
        .oneshot(
            Request::post(format!("/runs/{}/cancel", created.run_id))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("run.conflict"), "{text}");
}

#[tokio::test]
async fn an_unknown_status_filter_is_rejected() {
    let (state, _queue) = harness().await;
    let response = router(state)
        .oneshot(
            Request::get("/runs?status=bogus")
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("run.invalid"), "{text}");
}
