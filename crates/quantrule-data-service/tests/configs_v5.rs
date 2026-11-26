//! The v5 config upload resolves its data-owned references and requires frozen packs.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{body_bytes, state_with_configs_and_artifacts};
use quantrule_api::{
    ConfigOrigin, ConfigSummary, Digest, ErrorCode, ErrorEnvelope, PROFILE_HEADER, PackRecord,
    ReferenceKind,
};
use quantrule_data_service::DataState;
use quantrule_data_service::router;
use quantrule_runtime::adapters::ObjectStore;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::path::Path;
use std::sync::Arc;
use tower::ServiceExt;

fn pack_path(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../config/examples/verify-v5/packs")
        .join(name)
}

fn ok<T: DeserializeOwned>(bytes: &[u8]) -> T {
    serde_json::from_slice(bytes).expect("response JSON")
}

async fn decode<T: DeserializeOwned>(response: axum::response::Response) -> T {
    ok(&body_bytes(response).await)
}

async fn create_pack(state: Arc<DataState>, name: &str) -> PackRecord {
    let body = std::fs::read(pack_path(name)).expect("pack fixture");
    let response = router(state)
        .oneshot(
            Request::post("/catalog/packs")
                .header(PROFILE_HEADER, "default")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::CREATED, "pack {name}");
    decode(response).await
}

async fn transition(state: Arc<DataState>, id: &Digest, action: &str) -> PackRecord {
    let response = router(state)
        .oneshot(
            Request::post(format!("/catalog/packs/{id}/{action}"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK, "pack {id} {action}");
    decode(response).await
}

fn reference(id: &Digest) -> Value {
    let wire = id.to_string();
    json!({ "id": wire, "hash": wire })
}

fn v5_bundle(strategy: &Digest, execution: &Digest, portfolio: &Digest) -> Value {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/config/valid-v5");
    let mut run: Value =
        serde_json::from_str(&std::fs::read_to_string(root.join("run.json")).expect("run"))
            .expect("run json");
    run["strategy_pack"] = reference(strategy);
    run["portfolio_policy"] = reference(portfolio);
    run["execution_model"] = reference(execution);
    let files = serde_json::Map::from_iter([
        (
            "run.json".to_owned(),
            json!(serde_json::to_string(&run).expect("run text")),
        ),
        (
            "data.json".to_owned(),
            json!(std::fs::read_to_string(root.join("data.json")).expect("data")),
        ),
        (
            "model.json".to_owned(),
            json!(std::fs::read_to_string(root.join("model.json")).expect("model")),
        ),
    ]);
    json!({ "name": "v5-upload", "files": files })
}

async fn upload(state: Arc<DataState>, payload: &Value) -> axum::response::Response {
    router(state)
        .oneshot(
            Request::post("/catalog/configs")
                .header(PROFILE_HEADER, "default")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .expect("request"),
        )
        .await
        .expect("response")
}

async fn uploads(state: Arc<DataState>) -> Vec<ConfigSummary> {
    let response = router(state)
        .oneshot(
            Request::get("/catalog/configs")
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    decode::<Vec<ConfigSummary>>(response)
        .await
        .into_iter()
        .filter(|config| config.origin == ConfigOrigin::Upload)
        .collect()
}

#[tokio::test]
async fn a_v5_upload_requires_frozen_packs() {
    let (state, _objects, _packs) = state_with_configs_and_artifacts().await;
    let strategy = create_pack(Arc::clone(&state), "strategy.json").await;
    let execution = create_pack(Arc::clone(&state), "execution.json").await;
    let portfolio = create_pack(Arc::clone(&state), "portfolio.json").await;
    let payload = v5_bundle(
        &strategy.summary.pack_id,
        &execution.summary.pack_id,
        &portfolio.summary.pack_id,
    );

    let response = upload(Arc::clone(&state), &payload).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let error: ErrorEnvelope = decode(response).await;
    assert_eq!(error.error.code, ErrorCode::PackInvalid);
    assert!(error.error.message.contains("draft"), "{error:?}");
    assert!(uploads(Arc::clone(&state)).await.is_empty());

    for record in [&strategy, &execution, &portfolio] {
        transition(Arc::clone(&state), &record.summary.pack_id, "validate").await;
        transition(Arc::clone(&state), &record.summary.pack_id, "freeze").await;
    }
    let response = upload(Arc::clone(&state), &payload).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let config: ConfigSummary = decode(response).await;
    assert!(config.valid);
    assert_eq!(uploads(Arc::clone(&state)).await.len(), 1);

    transition(Arc::clone(&state), &strategy.summary.pack_id, "retire").await;
    let response = upload(Arc::clone(&state), &payload).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let error: ErrorEnvelope = decode(response).await;
    assert_eq!(error.error.code, ErrorCode::PackInvalid);
    assert!(error.error.message.contains("retired"), "{error:?}");
}

#[tokio::test]
async fn a_v5_upload_with_a_missing_pack_is_refused() {
    let (state, _objects, _packs) = state_with_configs_and_artifacts().await;
    let missing = Digest::sha256(b"missing-strategy");
    let payload = v5_bundle(&missing, &Digest::sha256(b"missing-execution"), &missing);
    let response = upload(Arc::clone(&state), &payload).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let error: ErrorEnvelope = decode(response).await;
    assert_eq!(error.error.code, ErrorCode::PackNotFound);
    assert!(uploads(Arc::clone(&state)).await.is_empty());
}

#[tokio::test]
async fn a_v5_upload_with_a_missing_pack_object_is_refused() {
    let (state, objects, _packs) = state_with_configs_and_artifacts().await;
    let strategy = create_pack(Arc::clone(&state), "strategy.json").await;
    let execution = create_pack(Arc::clone(&state), "execution.json").await;
    let portfolio = create_pack(Arc::clone(&state), "portfolio.json").await;
    objects
        .delete(&strategy.object_key)
        .await
        .expect("the pack object is deleted");
    let payload = v5_bundle(
        &strategy.summary.pack_id,
        &execution.summary.pack_id,
        &portfolio.summary.pack_id,
    );
    let response = upload(Arc::clone(&state), &payload).await;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let error: ErrorEnvelope = decode(response).await;
    assert_eq!(error.error.code, ErrorCode::Internal);
    assert!(uploads(Arc::clone(&state)).await.is_empty());
}

#[tokio::test]
async fn the_v5_reference_kinds_are_the_three_packs_and_the_model() {
    let (state, _objects, _packs) = state_with_configs_and_artifacts().await;
    let strategy = create_pack(Arc::clone(&state), "strategy.json").await;
    let execution = create_pack(Arc::clone(&state), "execution.json").await;
    let portfolio = create_pack(Arc::clone(&state), "portfolio.json").await;
    let payload = v5_bundle(
        &strategy.summary.pack_id,
        &execution.summary.pack_id,
        &portfolio.summary.pack_id,
    );
    for record in [&strategy, &execution, &portfolio] {
        transition(Arc::clone(&state), &record.summary.pack_id, "validate").await;
        transition(Arc::clone(&state), &record.summary.pack_id, "freeze").await;
    }
    let response = upload(Arc::clone(&state), &payload).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let config: ConfigSummary = decode(response).await;
    let response = router(state)
        .oneshot(
            Request::get(format!("/internal/v1/configs/{}", config.config_id))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let bundle: quantrule_service_api::ConfigBundle = decode(response).await;
    let kinds: Vec<ReferenceKind> = bundle
        .references
        .iter()
        .map(|reference| reference.kind)
        .collect();
    assert_eq!(
        kinds,
        vec![
            ReferenceKind::StrategyPack,
            ReferenceKind::PortfolioPolicy,
            ReferenceKind::ExecutionModel,
        ]
    );
    assert!(bundle.references.iter().all(|reference| {
        reference
            .pack
            .as_ref()
            .is_some_and(|pack| pack.state == quantrule_api::PackState::Frozen)
    }));
}
