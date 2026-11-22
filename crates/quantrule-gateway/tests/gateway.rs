//! The gateway routes that remain local: health, version, profiles, and the
//! client prefix.
//!
//! The data routes are exercised in `quantrule-data-service`, the model routes
//! in `quantrule-model-service`, and the run and hypothesis routes in
//! `quantrule-evidence-service`. The proxies are covered in `tests/proxy.rs`,
//! `tests/model_proxy.rs`, and `tests/evidence_proxy.rs`.

mod common;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, StatusCode};
use axum::routing::get;
use axum::{Json, Router};
use common::{body_bytes, state, state_with_data};
use quantrule_api::{
    Digest, HealthReport, PROFILE_HEADER, ProfileId, ProfileSummary, ServiceState,
};
use quantrule_gateway::server::router;
use quantrule_gateway::server::{AppState, Services};
use quantrule_runtime::client::{HttpServiceClient, MemoryDataClient, Service};
use quantrule_runtime::{Config, HealthRegistry};
use quantrule_service_api::DatasetObject;
use tower::ServiceExt;

#[tokio::test]
async fn health_is_served_as_json() {
    let response = router(state())
        .oneshot(
            Request::get("/health")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn ready_without_upstreams_is_unavailable() {
    let response = router(state())
        .oneshot(Request::get("/ready").body(Body::empty()).expect("request"))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("\"state\":\"down\""), "{text}");
}

#[derive(Clone)]
struct Identity(&'static str);

async fn fake_health(State(identity): State<Identity>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "v": "quantrule.api/v1",
        "service": identity.0,
        "state": "healthy",
        "services": [],
        "queue_depth": 0,
        "workers": 0,
        "image_digest": null
    }))
}

async fn fake_ready(State(identity): State<Identity>) -> (StatusCode, Json<serde_json::Value>) {
    (StatusCode::OK, fake_health(State(identity)).await)
}

#[derive(Clone)]
struct Draining(&'static str);

async fn draining_ready(State(identity): State<Draining>) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(serde_json::json!({
            "v": "quantrule.api/v1",
            "service": identity.0,
            "state": "down",
            "services": [{"name": "drain", "state": "down", "detail": "draining"}],
            "queue_depth": 0,
            "workers": 0,
            "image_digest": null
        })),
    )
}

async fn fake_service(service: &'static str) -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let app = Router::new()
        .route("/health", get(fake_health))
        .route("/ready", get(fake_ready))
        .with_state(Identity(service));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let address = listener.local_addr().expect("address");
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("health upstream");
    });
    (address, handle)
}

async fn fake_draining_service(service: &'static str) -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let app = Router::new()
        .route("/ready", get(draining_ready))
        .with_state(Draining(service));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let address = listener.local_addr().expect("address");
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("draining upstream");
    });
    (address, handle)
}

fn ready_state(addresses: [SocketAddr; 4]) -> Arc<AppState> {
    let [data, model, evidence, simulation] = addresses;
    let client = |service: Service, address: SocketAddr| {
        Arc::new(HttpServiceClient::new(service, &format!("http://{address}")).expect("client"))
    };
    Arc::new(AppState::with_services(
        &Config::default(),
        HealthRegistry::named("gateway"),
        Services {
            upstream: Some(client(Service::Data, data)),
            model_upstream: Some(client(Service::Model, model)),
            evidence_upstream: Some(client(Service::Evidence, evidence)),
            simulation_upstream: Some(client(Service::Simulation, simulation)),
            ..Services::default()
        },
    ))
}

#[tokio::test]
async fn ready_accepts_matching_service_identities() {
    let (data, data_handle) = fake_service("data").await;
    let (model, model_handle) = fake_service("model").await;
    let (evidence, evidence_handle) = fake_service("evidence").await;
    let (simulation, simulation_handle) = fake_service("simulation").await;
    let response = router(ready_state([data, model, evidence, simulation]))
        .oneshot(Request::get("/ready").body(Body::empty()).expect("request"))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let report: HealthReport =
        serde_json::from_slice(&body_bytes(response).await).expect("health report");
    assert_eq!(report.state, ServiceState::Healthy);
    assert_eq!(report.services.len(), 4);
    data_handle.abort();
    model_handle.abort();
    evidence_handle.abort();
    simulation_handle.abort();
}

#[tokio::test]
async fn a_draining_upstream_is_down_for_routing_but_the_gateway_health_stays_up() {
    let (data, data_handle) = fake_draining_service("data").await;
    let (model, model_handle) = fake_service("model").await;
    let (evidence, evidence_handle) = fake_service("evidence").await;
    let (simulation, simulation_handle) = fake_service("simulation").await;
    let state = ready_state([data, model, evidence, simulation]);

    let response = router(Arc::clone(&state))
        .oneshot(Request::get("/ready").body(Body::empty()).expect("request"))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let report: HealthReport =
        serde_json::from_slice(&body_bytes(response).await).expect("health report");
    let entry = report
        .services
        .iter()
        .find(|entry| entry.name == "data")
        .expect("data entry");
    assert_eq!(entry.state, ServiceState::Down);
    assert_eq!(entry.detail.as_deref(), Some("draining"));

    let response = router(state)
        .oneshot(
            Request::get("/health")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);

    data_handle.abort();
    model_handle.abort();
    evidence_handle.abort();
    simulation_handle.abort();
}

#[tokio::test]
async fn a_draining_gateway_answers_ready_with_503_without_probing() {
    use quantrule_runtime::request_id::RequestIds;
    use quantrule_runtime::{Drain, Shutdown};

    let (trigger, shutdown) = Shutdown::channel();
    let metrics = quantrule_runtime::metrics::Metrics::new("gateway", "1.5.2", None);
    let drain = Drain::new(
        shutdown.clone(),
        Some(metrics.clone()),
        std::time::Duration::from_secs(20),
    );
    let health = HealthRegistry::named("gateway");
    let state = Arc::new(AppState::with_observability(
        &Config::default(),
        health,
        metrics,
        RequestIds::new("gateway"),
        Services::default(),
        drain,
    ));
    trigger.trigger();

    let response = router(Arc::clone(&state))
        .oneshot(Request::get("/ready").body(Body::empty()).expect("request"))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let report: HealthReport =
        serde_json::from_slice(&body_bytes(response).await).expect("health report");
    assert_eq!(report.services[0].detail.as_deref(), Some("draining"));

    let response = router(state)
        .oneshot(
            Request::get("/health")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn ready_probes_the_strategy_service_only_when_it_is_configured() {
    let (strategy, strategy_handle) = fake_service("strategy").await;
    let fixture = ready_state([
        zero_address(),
        zero_address(),
        zero_address(),
        zero_address(),
    ]);
    let mut services = Services {
        strategy_upstream: Some(Arc::new(
            HttpServiceClient::new(Service::Strategy, &format!("http://{strategy}"))
                .expect("client"),
        )),
        ..Services::default()
    };
    services.upstream = None;
    let state = Arc::new(AppState::with_services(
        &Config::default(),
        HealthRegistry::named("gateway"),
        services,
    ));
    let response = router(state)
        .oneshot(Request::get("/ready").body(Body::empty()).expect("request"))
        .await
        .expect("response");
    // The four default probes are unconfigured and down; the strategy probe
    // must still be present, so this report has five entries.
    let report: HealthReport =
        serde_json::from_slice(&body_bytes(response).await).expect("health report");
    assert_eq!(report.services.len(), 5);
    let entry = report
        .services
        .iter()
        .find(|entry| entry.name == "strategy")
        .expect("strategy entry");
    assert_eq!(entry.state, ServiceState::Healthy);

    let response = router(fixture)
        .oneshot(Request::get("/ready").body(Body::empty()).expect("request"))
        .await
        .expect("response");
    let report: HealthReport =
        serde_json::from_slice(&body_bytes(response).await).expect("health report");
    assert_eq!(
        report.services.len(),
        4,
        "an absent strategy URL adds no probe"
    );
    strategy_handle.abort();
}

fn zero_address() -> SocketAddr {
    "127.0.0.1:9".parse().expect("address")
}

#[tokio::test]
async fn ready_rejects_a_response_from_another_service() {
    let (data, data_handle) = fake_service("gateway").await;
    let (model, model_handle) = fake_service("model").await;
    let (evidence, evidence_handle) = fake_service("evidence").await;
    let (simulation, simulation_handle) = fake_service("simulation").await;
    let response = router(ready_state([data, model, evidence, simulation]))
        .oneshot(Request::get("/ready").body(Body::empty()).expect("request"))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let report: HealthReport =
        serde_json::from_slice(&body_bytes(response).await).expect("health report");
    let entry = report
        .services
        .iter()
        .find(|entry| entry.name == "data")
        .expect("data entry");
    assert_eq!(entry.state, ServiceState::Down);
    assert!(
        entry
            .detail
            .as_deref()
            .unwrap_or_default()
            .contains("identifies itself"),
        "{:?}",
        entry.detail
    );
    data_handle.abort();
    model_handle.abort();
    evidence_handle.abort();
    simulation_handle.abort();
}

#[tokio::test]
async fn version_reports_the_contract() {
    let response = router(state())
        .oneshot(
            Request::get("/version")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("quantrule.api/v1"), "{text}");
}

#[tokio::test]
async fn an_unknown_path_is_not_found() {
    let response = router(state())
        .oneshot(Request::get("/nope").body(Body::empty()).expect("request"))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_profile_is_created_and_listed() {
    let state = state_with_data(MemoryDataClient::default()).await;
    let payload = r#"{"profile_id":"delta","name":"Delta"}"#;
    let response = router(Arc::clone(&state))
        .oneshot(
            Request::post("/profiles")
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);
    let created: ProfileSummary =
        serde_json::from_slice(&body_bytes(response).await).expect("profile");
    assert_eq!(created.profile_id.as_str(), "delta");
    assert_eq!(created.name, "Delta");
    assert_eq!(created.datasets, 0);

    let response = router(state)
        .oneshot(
            Request::get("/profiles")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let listed: Vec<ProfileSummary> =
        serde_json::from_slice(&body_bytes(response).await).expect("profiles");
    assert!(
        listed
            .iter()
            .any(|profile| profile.profile_id.as_str() == "delta")
    );
    assert!(
        listed
            .iter()
            .any(|profile| profile.profile_id.as_str() == "default")
    );
}

#[tokio::test]
async fn a_duplicate_profile_is_idempotent() {
    let state = state_with_data(MemoryDataClient::default()).await;
    let payload = r#"{"profile_id":"delta","name":"Delta"}"#;
    let first = router(Arc::clone(&state))
        .oneshot(
            Request::post("/profiles")
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(first.status(), StatusCode::CREATED);
    let first: ProfileSummary = serde_json::from_slice(&body_bytes(first).await).expect("profile");

    let second = router(state)
        .oneshot(
            Request::post("/profiles")
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(second.status(), StatusCode::OK);
    let second: ProfileSummary =
        serde_json::from_slice(&body_bytes(second).await).expect("profile");
    assert_eq!(first, second);
}

#[tokio::test]
async fn an_invalid_profile_is_rejected() {
    let state = state_with_data(MemoryDataClient::default()).await;
    let bad_id = r#"{"profile_id":"Bad!","name":"Bad"}"#;
    let response = router(Arc::clone(&state))
        .oneshot(
            Request::post("/profiles")
                .header("content-type", "application/json")
                .body(Body::from(bad_id))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("profile.invalid"), "{text}");

    let blank_name = r#"{"profile_id":"ok","name":"   "}"#;
    let response = router(state)
        .oneshot(
            Request::post("/profiles")
                .header("content-type", "application/json")
                .body(Body::from(blank_name))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("profile.invalid"), "{text}");
}

#[tokio::test]
async fn an_unknown_profile_is_not_found_before_the_upstream() {
    let response = router(state_with_data(MemoryDataClient::default()).await)
        .oneshot(
            Request::get("/catalog/datasets")
                .header(PROFILE_HEADER, "ghost")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("profile.not_found"), "{text}");
}

#[tokio::test]
async fn a_dataset_contributes_to_its_profile_count() {
    let data = MemoryDataClient::default();
    let alpha = ProfileId::parse("alpha").expect("alpha");
    let version = Digest::sha256(b"candles");
    data.add_dataset(
        &alpha,
        DatasetObject {
            data_version: version.clone(),
            object_key: format!("datasets/{}", version.as_hex()),
            sha256: version,
            candles: 2,
            first_bar: 1_704_067_200,
            last_bar: 1_704_067_260,
            bytes: 100,
        },
    );
    let response = router(state_with_data(data).await)
        .oneshot(
            Request::get("/profiles")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    let listed: Vec<ProfileSummary> =
        serde_json::from_slice(&body_bytes(response).await).expect("profiles");
    let alpha = listed
        .iter()
        .find(|profile| profile.profile_id.as_str() == "alpha")
        .expect("alpha");
    assert_eq!(alpha.datasets, 1);
    let default = listed
        .iter()
        .find(|profile| profile.profile_id.as_str() == "default")
        .expect("default");
    assert_eq!(default.datasets, 0);
}

#[tokio::test]
async fn the_client_prefix_serves_the_same_routes() {
    let response = router(state())
        .oneshot(
            Request::get("/api/v1/version")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("quantrule.api/v1"), "{text}");
}
