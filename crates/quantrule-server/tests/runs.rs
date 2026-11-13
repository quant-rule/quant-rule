//! Run submission, listing, detail, sharing, and cancellation.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{CSV, body_bytes, memory_profiles};
use quantrule_api::{
    ConfigSummary, DatasetRecord, Digest, PROFILE_HEADER, RunDetail, RunState, RunSummary,
    SubmitRunResponse,
};
use quantrule_server::configs::MemoryConfigs;
use quantrule_server::data::MemoryCatalog;
use quantrule_server::runs::MemoryRuns;
use quantrule_server::runtime::adapters::{Backend, BoxFuture, Queue, QueueError};
use quantrule_server::runtime::server::router;
use quantrule_server::runtime::{AppState, Config, HealthRegistry, Services};
use std::path::Path;
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

async fn harness() -> (Arc<AppState>, MemoryQueue) {
    let queue = MemoryQueue::default();
    let config = Config {
        config_root: Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/config"),
        ..Config::default()
    };
    let state = Arc::new(AppState::with_services(
        &config,
        HealthRegistry::new(),
        Services {
            catalog: Some(Arc::new(MemoryCatalog::default())),
            objects: Some(Arc::new(common::MemoryObjects::default())),
            profiles: Some(memory_profiles(&["default", "alpha"]).await),
            configs: Some(Arc::new(MemoryConfigs::default())),
            runs: Some(Arc::new(MemoryRuns::default())),
            queue: Some(Arc::new(queue.clone())),
            ..Services::default()
        },
    ));
    (state, queue)
}

async fn upload_csv(state: Arc<AppState>, profile: &str, csv: &str) -> DatasetRecord {
    let response = router(state)
        .oneshot(
            Request::post("/catalog/datasets?name=candles.csv")
                .header(PROFILE_HEADER, profile)
                .body(Body::from(csv.to_owned()))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);
    serde_json::from_slice(&body_bytes(response).await).expect("dataset")
}

async fn upload(state: Arc<AppState>, profile: &str) -> DatasetRecord {
    upload_csv(state, profile, CSV).await
}

async fn configs_for(state: Arc<AppState>, profile: &str) -> Vec<ConfigSummary> {
    let response = router(state)
        .oneshot(
            Request::get("/catalog/configs")
                .header(PROFILE_HEADER, profile)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    serde_json::from_slice(&body_bytes(response).await).expect("configs")
}

async fn configs(state: Arc<AppState>) -> Vec<ConfigSummary> {
    configs_for(state, "default").await
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

async fn seed(state: Arc<AppState>) -> (Digest, String, String) {
    let dataset = upload(Arc::clone(&state), "default").await;
    let configs = configs(state).await;
    let valid = configs
        .iter()
        .find(|config| config.valid)
        .expect("a valid config")
        .config_id
        .clone();
    let invalid = configs
        .iter()
        .find(|config| !config.valid)
        .expect("an invalid config")
        .config_id
        .clone();
    (dataset.summary.data_version, valid, invalid)
}

#[tokio::test]
async fn a_submission_is_content_addressed_and_listed() {
    let (state, queue) = harness().await;
    let (data_version, config_id, _) = seed(Arc::clone(&state)).await;

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
    let first = upload(Arc::clone(&state), "default").await;
    let changed = CSV.replace("100.5", "100.75");
    let second = upload_csv(Arc::clone(&state), "default", &changed).await;
    assert_ne!(first.summary.data_version, second.summary.data_version);
    let config_id = configs(Arc::clone(&state))
        .await
        .into_iter()
        .find(|config| config.valid)
        .expect("valid")
        .config_id;

    let first = submit(
        Arc::clone(&state),
        "default",
        &first.summary.data_version,
        &config_id,
    )
    .await;
    let first: SubmitRunResponse = serde_json::from_slice(&body_bytes(first).await).expect("first");
    let second = submit(
        Arc::clone(&state),
        "default",
        &second.summary.data_version,
        &config_id,
    )
    .await;
    let second: SubmitRunResponse =
        serde_json::from_slice(&body_bytes(second).await).expect("second");
    assert_ne!(first.run_id, second.run_id);
    assert_eq!(list(state, "default").await.len(), 2);
}

#[tokio::test]
async fn an_invalid_config_creates_nothing() {
    let (state, queue) = harness().await;
    let (data_version, _, invalid) = seed(Arc::clone(&state)).await;
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
    let (_, config_id, _) = seed(Arc::clone(&state)).await;
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
    let dataset = upload(Arc::clone(&state), "default").await;
    let same = upload(Arc::clone(&state), "alpha").await;
    assert_eq!(dataset.summary.data_version, same.summary.data_version);
    let default_configs = configs(Arc::clone(&state)).await;
    let alpha_configs = configs_for(Arc::clone(&state), "alpha").await;
    let config_id = default_configs
        .into_iter()
        .find(|config| config.valid)
        .expect("valid")
        .config_id;
    assert!(
        alpha_configs
            .iter()
            .any(|config| config.config_id == config_id)
    );

    let first = submit(
        Arc::clone(&state),
        "default",
        &dataset.summary.data_version,
        &config_id,
    )
    .await;
    assert_eq!(first.status(), StatusCode::ACCEPTED);
    let first: SubmitRunResponse = serde_json::from_slice(&body_bytes(first).await).expect("first");

    let second = submit(
        Arc::clone(&state),
        "alpha",
        &same.summary.data_version,
        &config_id,
    )
    .await;
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
    let (data_version, config_id, _) = seed(Arc::clone(&state)).await;
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
    let (state, _queue) = harness().await;
    let (data_version, config_id, _) = seed(Arc::clone(&state)).await;
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
