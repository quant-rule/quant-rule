//! The signal and lineage proxy surface.
//!
//! The gateway forwards a signal create to the strategy service's internal route,
//! forwards signal reads to the data service, and forwards a run lineage read to
//! evidence. It validates the profile, keeps the bearer gate, streams the create
//! body, preserves bytes and ETag, and refuses a traversal before any upstream.

mod common;

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use axum::body::Body;
use axum::extract::{Path, Request, State};
use axum::http::{Method, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use common::{body_bytes, memory_profiles};
use quantrule_api::PROFILE_HEADER;
use quantrule_gateway::server::router;
use quantrule_gateway::server::{AppState, Services};
use quantrule_runtime::client::{HttpServiceClient, Service};
use quantrule_runtime::{Config, HealthRegistry};
use serde_json::{Value, json};
use tower::ServiceExt;

#[derive(Clone, Default)]
struct Fake {
    hits: Arc<AtomicUsize>,
}

async fn signal_create(State(fake): State<Fake>, request: Request) -> impl IntoResponse {
    fake.hits.fetch_add(1, Ordering::SeqCst);
    let profile = request
        .headers()
        .get(PROFILE_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let path = request.uri().path().to_owned();
    let method = request.method().as_str().to_owned();
    let body = axum::body::to_bytes(request.into_body(), 65_536)
        .await
        .expect("body");
    Json(json!({
        "path": path,
        "method": method,
        "profile": profile,
        "body": String::from_utf8(body.to_vec()).expect("utf8"),
    }))
}

async fn signal_list(State(fake): State<Fake>, uri: axum::http::Uri) -> Response {
    fake.hits.fetch_add(1, Ordering::SeqCst);
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/json".to_owned()),
            (header::ETAG, "\"signal-list\"".to_owned()),
        ],
        Json(json!({ "path": uri.path() })),
    )
        .into_response()
}

async fn signal_bytes(State(fake): State<Fake>, Path(id): Path<String>) -> impl IntoResponse {
    fake.hits.fetch_add(1, Ordering::SeqCst);
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/octet-stream".to_owned()),
            (header::ETAG, format!("\"{id}\"")),
        ],
        b"signal-document\n".to_vec(),
    )
}

async fn lineage(State(fake): State<Fake>, Path(id): Path<String>) -> impl IntoResponse {
    fake.hits.fetch_add(1, Ordering::SeqCst);
    Json(json!({
        "run_id": id,
        "references": [],
        "recorded_at": "2026-11-22T00:00:00Z",
    }))
}

async fn fallback(State(fake): State<Fake>, uri: axum::http::Uri) -> Response {
    fake.hits.fetch_add(1, Ordering::SeqCst);
    (StatusCode::NOT_FOUND, uri.to_string()).into_response()
}

async fn start(routes: Router<Fake>, fake: Fake) -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let app = routes.fallback(fallback).with_state(fake);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let address = listener.local_addr().expect("address");
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("upstream");
    });
    (address, handle)
}

fn upstream(service: Service, address: SocketAddr) -> Arc<HttpServiceClient> {
    Arc::new(
        HttpServiceClient::new(service, &format!("http://{address}")).expect("upstream client"),
    )
}

async fn gateway(services: Services, token: Option<&str>) -> Arc<AppState> {
    let config = Config {
        bearer_token: token.map(str::to_owned),
        ..Config::default()
    };
    Arc::new(AppState::with_services(
        &config,
        HealthRegistry::new(),
        Services {
            profiles: Some(memory_profiles(&["default", "alpha"]).await),
            ..services
        },
    ))
}

async fn send(
    state: &Arc<AppState>,
    method: Method,
    path: &str,
    profile: Option<&str>,
    token: Option<&str>,
    body: Body,
) -> Response {
    let mut request = Request::builder().method(method).uri(path);
    if let Some(profile) = profile {
        request = request.header(PROFILE_HEADER, profile);
    }
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    request = request.header("content-type", "application/json");
    router(Arc::clone(state))
        .oneshot(request.body(body).expect("request"))
        .await
        .expect("response")
}

#[tokio::test]
async fn a_signal_create_is_rewritten_to_the_internal_strategy_route() {
    let fake = Fake::default();
    let (address, handle) = start(
        Router::new().route("/internal/v1/signals", post(signal_create)),
        fake,
    )
    .await;
    let state = gateway(
        Services {
            strategy_upstream: Some(upstream(Service::Strategy, address)),
            ..Services::default()
        },
        None,
    )
    .await;

    let body = "{\"model_artifact_id\":\"sha256:abc\"}";
    let response = send(
        &state,
        Method::POST,
        "/api/v1/signals",
        Some("alpha"),
        None,
        Body::from(body),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let value: Value = serde_json::from_slice(&body_bytes(response).await).expect("json");
    assert_eq!(value["path"], "/internal/v1/signals");
    assert_eq!(value["method"], "POST");
    assert_eq!(value["profile"], "alpha");
    assert_eq!(value["body"], body);
    handle.abort();
}

#[tokio::test]
async fn a_missing_strategy_service_is_unavailable_for_create_but_reads_still_work() {
    let data_fake = Fake::default();
    let (data_address, data_handle) =
        start(Router::new().route("/signals", get(signal_list)), data_fake).await;
    let state = gateway(
        Services {
            upstream: Some(upstream(Service::Data, data_address)),
            ..Services::default()
        },
        None,
    )
    .await;

    let response = send(
        &state,
        Method::POST,
        "/signals",
        Some("default"),
        None,
        Body::from("{}"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(body.contains("service.unavailable"), "{body}");
    assert!(body.contains("strategy"), "{body}");

    let response = send(
        &state,
        Method::GET,
        "/signals",
        Some("default"),
        None,
        Body::empty(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let value: Value = serde_json::from_slice(&body_bytes(response).await).expect("json");
    assert_eq!(value["path"], "/signals");
    data_handle.abort();
}

#[tokio::test]
async fn signal_bytes_keep_the_data_etag() {
    let fake = Fake::default();
    let hits = Arc::clone(&fake.hits);
    let (address, handle) = start(
        Router::new().route("/signals/{id}/bytes", get(signal_bytes)),
        fake,
    )
    .await;
    let state = gateway(
        Services {
            upstream: Some(upstream(Service::Data, address)),
            ..Services::default()
        },
        None,
    )
    .await;

    let id = format!("sha256:{}", "a".repeat(64));
    let response = send(
        &state,
        Method::GET,
        &format!("/signals/{id}/bytes"),
        Some("default"),
        None,
        Body::empty(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let etag = format!("\"{id}\"");
    assert_eq!(
        response
            .headers()
            .get(header::ETAG)
            .and_then(|value| value.to_str().ok()),
        Some(etag.as_str())
    );
    assert_eq!(body_bytes(response).await, b"signal-document\n");
    assert_eq!(hits.load(Ordering::SeqCst), 1);
    handle.abort();
}

#[tokio::test]
async fn a_signal_traversal_never_reaches_an_upstream() {
    let fake = Fake::default();
    let hits = Arc::clone(&fake.hits);
    let (address, handle) = start(Router::new(), fake).await;
    let state = gateway(
        Services {
            upstream: Some(upstream(Service::Data, address)),
            ..Services::default()
        },
        None,
    )
    .await;

    let response = send(
        &state,
        Method::GET,
        "/signals/%2e%2e/internal/v1/signals",
        Some("default"),
        None,
        Body::empty(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(hits.load(Ordering::SeqCst), 0);
    handle.abort();
}

#[tokio::test]
async fn a_signal_create_profile_is_validated_before_the_upstream() {
    let fake = Fake::default();
    let hits = Arc::clone(&fake.hits);
    let (address, handle) = start(
        Router::new().route("/internal/v1/signals", post(signal_create)),
        fake,
    )
    .await;
    let state = gateway(
        Services {
            strategy_upstream: Some(upstream(Service::Strategy, address)),
            ..Services::default()
        },
        None,
    )
    .await;

    let response = send(
        &state,
        Method::POST,
        "/signals",
        Some("ghost"),
        None,
        Body::from("{}"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(body.contains("profile.not_found"), "{body}");
    assert_eq!(hits.load(Ordering::SeqCst), 0);
    handle.abort();
}

#[tokio::test]
async fn the_bearer_gate_covers_signal_create_before_the_upstream() {
    let fake = Fake::default();
    let hits = Arc::clone(&fake.hits);
    let (address, handle) = start(
        Router::new().route("/internal/v1/signals", post(signal_create)),
        fake,
    )
    .await;
    let state = gateway(
        Services {
            strategy_upstream: Some(upstream(Service::Strategy, address)),
            ..Services::default()
        },
        Some("s3cret"),
    )
    .await;

    let response = send(
        &state,
        Method::POST,
        "/signals",
        Some("default"),
        None,
        Body::from("{}"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(hits.load(Ordering::SeqCst), 0);

    let response = send(
        &state,
        Method::POST,
        "/signals",
        Some("default"),
        Some("s3cret"),
        Body::from("{}"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(hits.load(Ordering::SeqCst), 1);
    handle.abort();
}

#[tokio::test]
async fn a_run_lineage_read_is_forwarded_to_evidence() {
    let fake = Fake::default();
    let (address, handle) = start(
        Router::new().route("/runs/{id}/lineage", get(lineage)),
        fake,
    )
    .await;
    let state = gateway(
        Services {
            evidence_upstream: Some(upstream(Service::Evidence, address)),
            ..Services::default()
        },
        None,
    )
    .await;

    let id = format!("sha256:{}", "b".repeat(64));
    let response = send(
        &state,
        Method::GET,
        &format!("/runs/{id}/lineage"),
        Some("default"),
        None,
        Body::empty(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let value: Value = serde_json::from_slice(&body_bytes(response).await).expect("json");
    assert_eq!(value["run_id"], id);
    assert_eq!(value["references"], json!([]));
    handle.abort();
}
