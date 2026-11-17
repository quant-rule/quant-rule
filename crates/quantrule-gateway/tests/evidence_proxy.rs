//! The evidence proxy: passthrough, profile injection, 503, and no local store.

mod common;

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use axum::body::{Body, Bytes};
use axum::extract::{Path, Request, State};
use axum::http::{HeaderMap, StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use common::{body_bytes, memory_profiles};
use quantrule_api::PROFILE_HEADER;
use quantrule_gateway::server::router;
use quantrule_gateway::server::{AppState, Services};
use quantrule_runtime::client::{HttpServiceClient, Service};
use quantrule_runtime::{Config, HealthRegistry};
use tower::ServiceExt;

#[derive(Clone, Default)]
struct Fake {
    hits: Arc<AtomicUsize>,
}

async fn echo(State(fake): State<Fake>, headers: HeaderMap, uri: Uri, body: Bytes) -> Response {
    fake.hits.fetch_add(1, Ordering::SeqCst);
    let profiles: Vec<String> = headers
        .get_all(PROFILE_HEADER)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .map(str::to_owned)
        .collect();
    (
        StatusCode::ACCEPTED,
        [
            (header::CONTENT_TYPE, "application/json".to_owned()),
            (header::ETAG, "\"evidence-etag\"".to_owned()),
        ],
        Json(serde_json::json!({
            "profiles": profiles,
            "path": uri.path(),
            "query": uri.query(),
            "body": String::from_utf8_lossy(&body),
        })),
    )
        .into_response()
}

async fn start(fake: Fake) -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let app = Router::new().fallback(echo).with_state(fake);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let address = listener.local_addr().expect("address");
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("upstream");
    });
    (address, handle)
}

const NESTED_BODY: &[u8] = b"fill_id,side,quantity\n1,buy,0.5\n";
const NESTED_ETAG: &str = "\"sha256:nested\"";

async fn nested_artifact(Path((id, name)): Path<(String, String)>) -> Response {
    assert_eq!(id, "sha256:abc");
    assert_eq!(name, "execution/fills.csv");
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/csv".to_owned()),
            (header::ETAG, NESTED_ETAG.to_owned()),
        ],
        Body::from(NESTED_BODY),
    )
        .into_response()
}

async fn start_nested() -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let app = Router::new().route("/api/v1/runs/{id}/artifacts/{*name}", get(nested_artifact));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let address = listener.local_addr().expect("address");
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("upstream");
    });
    (address, handle)
}

async fn gateway(address: SocketAddr) -> Arc<AppState> {
    let upstream = HttpServiceClient::new(Service::Evidence, &format!("http://{address}"))
        .expect("upstream client");
    Arc::new(AppState::with_services(
        &Config::default(),
        HealthRegistry::new(),
        Services {
            profiles: Some(memory_profiles(&["default", "alpha"]).await),
            evidence_upstream: Some(Arc::new(upstream)),
            ..Services::default()
        },
    ))
}

#[tokio::test]
async fn a_run_submission_passes_through_with_the_injected_profile() {
    let (address, handle) = start(Fake::default()).await;
    let payload = serde_json::json!({
        "data_version": format!("sha256:{}", "a".repeat(64)),
        "config_id": "cfg_valid",
    })
    .to_string();
    let response = router(gateway(address).await)
        .oneshot(
            Request::post("/api/v1/runs?limit=10")
                .header(PROFILE_HEADER, "default")
                .header("content-type", "application/json")
                .body(Body::from(payload.clone()))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    assert_eq!(
        response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("application/json")
    );
    assert_eq!(
        response
            .headers()
            .get(header::ETAG)
            .and_then(|value| value.to_str().ok()),
        Some("\"evidence-etag\"")
    );
    let value: serde_json::Value =
        serde_json::from_slice(&body_bytes(response).await).expect("json");
    assert_eq!(value["path"], "/api/v1/runs");
    assert_eq!(value["query"], "limit=10");
    assert_eq!(value["body"], payload);
    assert_eq!(value["profiles"], serde_json::json!(["default"]));
    handle.abort();
}

#[tokio::test]
async fn a_run_detail_artifact_and_digest_path_is_forwarded() {
    let (address, handle) = start(Fake::default()).await;
    for path in [
        "/runs/sha256:abc",
        "/runs/sha256:abc/cancel",
        "/runs/sha256:abc/artifacts/results.json",
        "/runs/sha256:abc/digest",
        "/hypotheses/fingerprint",
        "/research/runs",
        "/research/runs/MODEL-2026-11-17-01",
        "/research/runs/MODEL-2026-11-17-01/compare",
        "/research/baselines",
        "/research/baselines/model",
        "/research/invalidations",
        "/operations",
        "/operations/status",
    ] {
        let response = router(gateway(address).await)
            .oneshot(
                Request::get(path)
                    .header(PROFILE_HEADER, "default")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::ACCEPTED, "{path}");
        let value: serde_json::Value =
            serde_json::from_slice(&body_bytes(response).await).expect("json");
        assert_eq!(value["path"], path, "{path}");
        assert_eq!(value["profiles"], serde_json::json!(["default"]), "{path}");
    }
    handle.abort();
}

#[tokio::test]
async fn research_submit_and_decision_paths_are_forwarded_to_evidence() {
    let (address, handle) = start(Fake::default()).await;
    for path in [
        "/research/runs/MODEL-2026-11-17-01/submit",
        "/research/runs/MODEL-2026-11-17-01/decision",
    ] {
        let response = router(gateway(address).await)
            .oneshot(
                Request::post(path)
                    .header(PROFILE_HEADER, "default")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::ACCEPTED, "{path}");
        let value: serde_json::Value =
            serde_json::from_slice(&body_bytes(response).await).expect("json");
        assert_eq!(value["path"], path, "{path}");
        assert_eq!(value["profiles"], serde_json::json!(["default"]), "{path}");
    }
    handle.abort();
}

#[tokio::test]
async fn a_nested_artifact_is_forwarded_byte_for_byte() {
    let (address, handle) = start_nested().await;
    let response = router(gateway(address).await)
        .oneshot(
            Request::get("/api/v1/runs/sha256:abc/artifacts/execution/fills.csv")
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("text/csv")
    );
    assert_eq!(
        response
            .headers()
            .get(header::ETAG)
            .and_then(|value| value.to_str().ok()),
        Some(NESTED_ETAG)
    );
    assert_eq!(body_bytes(response).await, NESTED_BODY);
    handle.abort();
}

#[tokio::test]
async fn an_artifact_traversal_never_reaches_the_upstream() {
    let fake = Fake::default();
    let hits = Arc::clone(&fake.hits);
    let (address, handle) = start(fake).await;
    for path in [
        "/api/v1/runs/sha256:abc/artifacts/%2e%2e/secret",
        "/api/v1/runs/sha256:abc/artifacts/execution/../../secret",
        "/api/v1/runs/sha256:abc/artifacts//etc/passwd",
        "/api/v1/runs/sha256:abc/artifacts/execution%2Ffills.csv",
        "/api/v1/runs/sha256:abc/artifacts/execution%5Cfills.csv",
        "/api/v1/runs/sha256:abc/artifacts/",
        "/api/v1/research/../internal",
        "/api/v1/research/%2e%2e/internal",
        "/api/v1/researchX",
        "/api/v1/operations/../internal",
        "/api/v1/operations/%2e%2e/internal",
        "/api/v1/operationsX",
    ] {
        let response = router(gateway(address).await)
            .oneshot(
                Request::get(path)
                    .header(PROFILE_HEADER, "default")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
        assert_eq!(hits.load(Ordering::SeqCst), 0, "{path}");
    }
    handle.abort();
}

#[tokio::test]
async fn a_client_cannot_supply_a_second_profile_value() {
    let (address, handle) = start(Fake::default()).await;
    let mut request = Request::post("/runs")
        .header("content-type", "application/json")
        .body(Body::from("{}"))
        .expect("request");
    request
        .headers_mut()
        .append(PROFILE_HEADER, "default".parse().expect("default"));
    request
        .headers_mut()
        .append(PROFILE_HEADER, "alpha".parse().expect("alpha"));

    let response = router(gateway(address).await)
        .oneshot(request)
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let value: serde_json::Value =
        serde_json::from_slice(&body_bytes(response).await).expect("json");
    assert_eq!(value["profiles"], serde_json::json!(["default"]));
    handle.abort();
}

#[tokio::test]
async fn a_client_profile_is_validated_before_the_upstream() {
    let fake = Fake::default();
    let hits = Arc::clone(&fake.hits);
    let (address, handle) = start(fake).await;
    let response = router(gateway(address).await)
        .oneshot(
            Request::get("/runs")
                .header(PROFILE_HEADER, "ghost")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("profile.not_found"), "{text}");
    assert_eq!(hits.load(Ordering::SeqCst), 0);
    handle.abort();
}

#[tokio::test]
async fn a_closed_evidence_upstream_is_service_unavailable() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let address = listener.local_addr().expect("address");
    drop(listener);

    let response = router(gateway(address).await)
        .oneshot(
            Request::get("/runs")
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("service.unavailable"), "{text}");
    assert!(text.contains("evidence"), "{text}");
}

#[tokio::test]
async fn the_internal_run_route_is_not_proxied() {
    let fake = Fake::default();
    let hits = Arc::clone(&fake.hits);
    let (address, handle) = start(fake).await;
    let response = router(gateway(address).await)
        .oneshot(
            Request::post("/internal/v1/runs/sha256:abc/claim")
                .header(PROFILE_HEADER, "default")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(hits.load(Ordering::SeqCst), 0);
    handle.abort();
}

#[tokio::test]
async fn a_stage_submission_and_its_read_paths_are_forwarded_unchanged() {
    let (address, handle) = start(Fake::default()).await;
    let payload = serde_json::json!({
        "kind": "model",
        "config_id": "cfg_valid",
        "input": {
            "kind": "data_version",
            "data_version": format!("sha256:{}", "a".repeat(64)),
        },
    })
    .to_string();
    let response = router(gateway(address).await)
        .oneshot(
            Request::post("/api/v1/stages/model/runs?limit=10")
                .header(PROFILE_HEADER, "default")
                .header("content-type", "application/json")
                .body(Body::from(payload.clone()))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let value: serde_json::Value =
        serde_json::from_slice(&body_bytes(response).await).expect("json");
    assert_eq!(value["path"], "/api/v1/stages/model/runs");
    assert_eq!(value["query"], "limit=10");
    assert_eq!(value["body"], payload);
    assert_eq!(value["profiles"], serde_json::json!(["default"]));

    for (method, path) in [
        (axum::http::Method::GET, "/stages/runs"),
        (axum::http::Method::GET, "/stages/runs/sha256:abc"),
        (axum::http::Method::GET, "/stages/runs/sha256:abc/artifacts"),
        (
            axum::http::Method::GET,
            "/stages/runs/sha256:abc/artifacts/events.csv",
        ),
        (axum::http::Method::POST, "/stages/runs/sha256:abc/cancel"),
        (axum::http::Method::DELETE, "/stages/runs/sha256:abc"),
    ] {
        let response = router(gateway(address).await)
            .oneshot(
                Request::builder()
                    .method(method.clone())
                    .uri(path)
                    .header(PROFILE_HEADER, "default")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::ACCEPTED, "{method} {path}");
        let value: serde_json::Value =
            serde_json::from_slice(&body_bytes(response).await).expect("json");
        assert_eq!(value["path"], path, "{method} {path}");
        assert_eq!(
            value["profiles"],
            serde_json::json!(["default"]),
            "{method} {path}"
        );
    }
    handle.abort();
}

#[tokio::test]
async fn a_stage_path_is_never_rewritten_into_a_run_path() {
    let (address, handle) = start(Fake::default()).await;
    let response = router(gateway(address).await)
        .oneshot(
            Request::get("/stages/runs")
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    let value: serde_json::Value =
        serde_json::from_slice(&body_bytes(response).await).expect("json");
    assert_eq!(value["path"], "/stages/runs");
    assert_ne!(value["path"], "/runs");
    handle.abort();
}

#[tokio::test]
async fn a_stage_traversal_never_reaches_the_upstream() {
    let fake = Fake::default();
    let hits = Arc::clone(&fake.hits);
    let (address, handle) = start(fake).await;
    for path in [
        "/stages/runs/%2e%2e/secret",
        "/stages/runs/../secret",
        "/stages/runs/sha256:abc/artifacts/execution/../../secret",
        "/stages/runs/sha256:abc/artifacts//etc/passwd",
        "/stages/runs/sha256:abc/artifacts/execution%2Ffills.csv",
        "/stages/%2e%2e/internal",
    ] {
        let response = router(gateway(address).await)
            .oneshot(
                Request::get(path)
                    .header(PROFILE_HEADER, "default")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
        assert_eq!(hits.load(Ordering::SeqCst), 0, "{path}");
    }
    handle.abort();
}

#[tokio::test]
async fn a_stage_route_validates_the_profile_like_every_other_proxy() {
    let fake = Fake::default();
    let hits = Arc::clone(&fake.hits);
    let (address, handle) = start(fake).await;
    let response = router(gateway(address).await)
        .oneshot(
            Request::get("/stages/runs")
                .header(PROFILE_HEADER, "ghost")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("profile.not_found"), "{text}");
    assert_eq!(hits.load(Ordering::SeqCst), 0);
    handle.abort();
}

#[tokio::test]
async fn the_internal_stage_route_is_not_proxied() {
    let fake = Fake::default();
    let hits = Arc::clone(&fake.hits);
    let (address, handle) = start(fake).await;
    for path in [
        "/internal/v1/stages/sha256:abc",
        "/internal/v1/stages/sha256:abc/claim",
    ] {
        let response = router(gateway(address).await)
            .oneshot(
                Request::get(path)
                    .header(PROFILE_HEADER, "default")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
        assert_eq!(hits.load(Ordering::SeqCst), 0, "{path}");
    }
    handle.abort();
}

#[tokio::test]
async fn the_gateway_has_no_run_or_research_store() {
    use std::path::PathBuf;

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for relative in [
        "src/server.rs",
        "src/bin/qr_gateway.rs",
        "src/adapters/postgres/mod.rs",
    ] {
        let text = std::fs::read_to_string(manifest.join(relative)).expect("source");
        for forbidden in [
            "runs.runs",
            "runs.submissions",
            "stage.runs",
            "stage.submissions",
            "research.hypotheses",
            "PgRunStore",
            "PgStageStore",
            "PgHypothesisStore",
            "MemoryRuns",
            "MemoryStages",
            "StageStore",
            "HypothesisStore",
            "submit_run",
            "submit_stage",
            "create_hypothesis",
            "ROUTING_REQUESTED",
            "ROUTING_STAGE_REQUESTED",
            "ROUTING_CANCELLED",
        ] {
            assert!(
                !text.contains(forbidden),
                "{relative} still names {forbidden}"
            );
        }
    }
}
