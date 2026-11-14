//! The simulation proxy: passthrough, profile injection, 503, and no local
//! simulation handler or store.

mod common;

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use axum::body::{Body, Bytes};
use axum::extract::{Request, State};
use axum::http::{HeaderMap, StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
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

async fn read(State(fake): State<Fake>, headers: HeaderMap, uri: Uri) -> Response {
    fake.hits.fetch_add(1, Ordering::SeqCst);
    let profiles: Vec<String> = headers
        .get_all(PROFILE_HEADER)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .map(str::to_owned)
        .collect();
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/json".to_owned()),
            (header::ETAG, "\"simulation-etag\"".to_owned()),
        ],
        Json(serde_json::json!({
            "profiles": profiles,
            "path": uri.path(),
            "query": uri.query(),
        })),
    )
        .into_response()
}

async fn write(State(fake): State<Fake>, headers: HeaderMap, uri: Uri, body: Bytes) -> Response {
    fake.hits.fetch_add(1, Ordering::SeqCst);
    let profiles: Vec<String> = headers
        .get_all(PROFILE_HEADER)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .map(str::to_owned)
        .collect();
    (
        StatusCode::CREATED,
        [(header::CONTENT_TYPE, "application/json".to_owned())],
        Json(serde_json::json!({
            "profiles": profiles,
            "path": uri.path(),
            "body": String::from_utf8_lossy(&body),
        })),
    )
        .into_response()
}

async fn start(fake: Fake) -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let app = Router::new()
        .route("/simulation/runs", get(read).post(write))
        .route("/api/v1/simulation/runs", get(read).post(write))
        .route("/simulation/runs/{*rest}", get(read).post(write))
        .route("/api/v1/simulation/runs/{*rest}", get(read).post(write))
        .route("/internal/v1/simulation/runs", post(write))
        .fallback(|uri: Uri| async move { (StatusCode::NOT_FOUND, uri.to_string()) })
        .with_state(fake);
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
    let upstream = HttpServiceClient::new(Service::Simulation, &format!("http://{address}"))
        .expect("upstream client");
    Arc::new(AppState::with_services(
        &Config::default(),
        HealthRegistry::new(),
        Services {
            profiles: Some(memory_profiles(&["default", "alpha"]).await),
            simulation_upstream: Some(Arc::new(upstream)),
            ..Services::default()
        },
    ))
}

#[tokio::test]
async fn a_simulation_read_and_write_pass_through_unchanged() {
    let (address, handle) = start(Fake::default()).await;
    let response = router(gateway(address).await)
        .oneshot(
            Request::get("/api/v1/simulation/runs?status=marking&limit=5")
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
            .get(header::ETAG)
            .and_then(|value| value.to_str().ok()),
        Some("\"simulation-etag\"")
    );
    let value: serde_json::Value =
        serde_json::from_slice(&body_bytes(response).await).expect("json");
    assert_eq!(value["path"], "/api/v1/simulation/runs");
    assert_eq!(value["query"], "status=marking&limit=5");
    assert_eq!(value["profiles"], serde_json::json!(["default"]));

    let payload = serde_json::json!({
        "premise": "higher lows",
        "bundle": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "inputs": "EMA stack",
        "seed": 0,
        "criteria": "min_events=30"
    })
    .to_string();
    let response = router(gateway(address).await)
        .oneshot(
            Request::post("/simulation/runs")
                .header(PROFILE_HEADER, "default")
                .header("content-type", "application/json")
                .body(Body::from(payload.clone()))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);
    let value: serde_json::Value =
        serde_json::from_slice(&body_bytes(response).await).expect("json");
    assert_eq!(value["path"], "/simulation/runs");
    assert_eq!(value["body"], payload);
    assert_eq!(value["profiles"], serde_json::json!(["default"]));
    handle.abort();
}

#[tokio::test]
async fn the_client_profile_is_validated_and_replaced() {
    let (address, handle) = start(Fake::default()).await;
    let mut request = Request::get("/simulation/runs")
        .header(PROFILE_HEADER, "default")
        .body(Body::empty())
        .expect("request");
    request
        .headers_mut()
        .append(PROFILE_HEADER, "alpha".parse().expect("alpha"));

    let response = router(gateway(address).await)
        .oneshot(request)
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let value: serde_json::Value =
        serde_json::from_slice(&body_bytes(response).await).expect("json");
    assert_eq!(value["profiles"], serde_json::json!(["default"]));
    handle.abort();
}

#[tokio::test]
async fn an_unknown_profile_is_refused_before_the_upstream() {
    let fake = Fake::default();
    let hits = Arc::clone(&fake.hits);
    let (address, handle) = start(fake).await;
    let response = router(gateway(address).await)
        .oneshot(
            Request::get("/simulation/runs")
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
async fn a_closed_upstream_is_service_unavailable() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let address = listener.local_addr().expect("address");
    drop(listener);

    let response = router(gateway(address).await)
        .oneshot(
            Request::get("/simulation/runs")
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("service.unavailable"), "{text}");
    assert!(text.contains("simulation"), "{text}");
}

#[tokio::test]
async fn the_internal_simulation_route_is_not_proxied() {
    let fake = Fake::default();
    let hits = Arc::clone(&fake.hits);
    let (address, handle) = start(fake).await;
    let response = router(gateway(address).await)
        .oneshot(
            Request::post("/internal/v1/simulation/runs")
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

#[test]
fn the_gateway_source_has_no_simulation_handler_or_store() {
    use std::path::{Path, PathBuf};

    fn sources(root: &Path, found: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(root).expect("read dir") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                sources(&path, found);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                found.push(path);
            }
        }
    }

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let cargo = std::fs::read_to_string(manifest.join("Cargo.toml")).expect("manifest");
    assert!(
        !cargo.contains("quantrule-simulation"),
        "the gateway depends on the simulation domain or service"
    );

    let mut files = Vec::new();
    sources(&manifest.join("src"), &mut files);
    assert!(!files.is_empty(), "no sources found");
    for file in files {
        let relative = file
            .strip_prefix(&manifest)
            .expect("relative")
            .to_string_lossy()
            .replace('\\', "/");
        let text = std::fs::read_to_string(&file).expect("source");
        for forbidden in [
            "quantrule_simulation::",
            "SimulationStore",
            "simulation.runs",
            "simulation.marks",
            "INSERT INTO simulation",
            "UPDATE simulation",
            "DELETE FROM simulation",
        ] {
            assert!(
                !text.contains(forbidden),
                "{relative} still names {forbidden}"
            );
        }
    }
}
