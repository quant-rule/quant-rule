//! The data proxy: passthrough, profile injection, streaming, and 503.

mod common;

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use axum::body::Body;
use axum::extract::{Path, Request, State};
use axum::http::{HeaderMap, StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use common::{body_bytes, memory_profiles};
use futures_util::{StreamExt, stream};
use quantrule_api::{ErrorCode, ErrorEnvelope, PROFILE_HEADER};
use quantrule_gateway::server::router;
use quantrule_gateway::server::{AppState, Services};
use quantrule_runtime::client::{HttpServiceClient, Service};
use quantrule_runtime::{Config, HealthRegistry};
use tokio::sync::Notify;
use tower::ServiceExt;

#[derive(Clone, Default)]
struct Fake {
    hits: Arc<AtomicUsize>,
    first_chunk: Arc<Notify>,
}

async fn candles(
    State(fake): State<Fake>,
    Path(version): Path<String>,
    headers: HeaderMap,
) -> Response {
    fake.hits.fetch_add(1, Ordering::SeqCst);
    let profile = headers
        .get(PROFILE_HEADER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("<missing>");
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/csv".to_owned()),
            (header::ETAG, "\"sha256:etag\"".to_owned()),
        ],
        format!("{version}|{profile}|/api/v1/datasets/{version}/candles"),
    )
        .into_response()
}

async fn upload(
    State(fake): State<Fake>,
    headers: HeaderMap,
    request: Request,
) -> impl IntoResponse {
    fake.hits.fetch_add(1, Ordering::SeqCst);
    let profile = headers
        .get(PROFILE_HEADER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("<missing>")
        .to_owned();
    let mut body = request.into_body().into_data_stream();
    let first = futures_util::StreamExt::next(&mut body)
        .await
        .and_then(Result::ok)
        .map(|chunk| chunk.to_vec())
        .unwrap_or_default();
    fake.first_chunk.notify_one();
    let mut rest = Vec::new();
    while let Some(chunk) = futures_util::StreamExt::next(&mut body).await {
        match chunk {
            Ok(chunk) => rest.extend_from_slice(&chunk),
            Err(_) => break,
        }
    }
    let mut all = first;
    all.extend_from_slice(&rest);
    (
        StatusCode::CREATED,
        Json(serde_json::json!({
            "profile": profile,
            "bytes": all.len(),
            "query": "forwarded",
        })),
    )
}

async fn start(fake: Fake) -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let app = Router::new()
        .route("/api/v1/datasets/{version}/candles", get(candles))
        .route("/catalog/datasets", post(upload))
        .fallback(|State(fake): State<Fake>, uri: Uri| async move {
            fake.hits.fetch_add(1, Ordering::SeqCst);
            (StatusCode::NOT_FOUND, uri.to_string())
        })
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
    let upstream = HttpServiceClient::new(Service::Data, &format!("http://{address}"))
        .expect("upstream client");
    Arc::new(AppState::with_services(
        &Config::default(),
        HealthRegistry::new(),
        Services {
            profiles: Some(memory_profiles(&["default", "alpha"]).await),
            upstream: Some(Arc::new(upstream)),
            ..Services::default()
        },
    ))
}

#[tokio::test]
async fn a_get_preserves_the_path_status_body_and_etag() {
    let (address, handle) = start(Fake::default()).await;
    let response = router(gateway(address).await)
        .oneshot(
            Request::get("/api/v1/datasets/sha256:abc/candles")
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    let status = response.status();
    if status != StatusCode::OK {
        let body = body_bytes(response).await;
        panic!("status {status}: {}", String::from_utf8_lossy(&body));
    }
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
        Some("\"sha256:etag\"")
    );
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert_eq!(
        text,
        "sha256:abc|default|/api/v1/datasets/sha256:abc/candles"
    );
    handle.abort();
}

#[tokio::test]
async fn a_client_cannot_supply_a_second_profile_value() {
    let (address, handle) = start(Fake::default()).await;
    let mut request = Request::post("/catalog/datasets?name=candles.csv")
        .body(Body::from("candles"))
        .expect("request");
    request
        .headers_mut()
        .append(PROFILE_HEADER, "alpha".parse().expect("alpha"));
    request
        .headers_mut()
        .append(PROFILE_HEADER, "beta".parse().expect("beta"));

    let response = router(gateway(address).await)
        .oneshot(request)
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);
    let value: serde_json::Value =
        serde_json::from_slice(&body_bytes(response).await).expect("json");
    assert_eq!(value["profile"], "alpha");
    assert_eq!(value["bytes"], 7);
    handle.abort();
}

#[tokio::test]
async fn a_client_profile_is_validated_before_the_upstream() {
    let fake = Fake::default();
    let hits = Arc::clone(&fake.hits);
    let (address, handle) = start(fake).await;
    let response = router(gateway(address).await)
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
    assert_eq!(hits.load(Ordering::SeqCst), 0);
    handle.abort();
}

#[tokio::test]
async fn a_streamed_upload_reaches_the_upstream_before_the_client_finishes() {
    let fake = Fake::default();
    let first_chunk = Arc::clone(&fake.first_chunk);
    let (address, handle) = start(fake).await;

    let chunks = stream::once(async { Ok::<_, std::io::Error>(b"first".to_vec()) }).chain(
        stream::once(async move {
            // The upstream notifies after it has read the first chunk. A gateway
            // that buffered the body before forwarding would never reach this.
            first_chunk.notified().await;
            Ok(b"second".to_vec())
        }),
    );
    let request = Request::post("/catalog/datasets?name=stream.csv")
        .header(PROFILE_HEADER, "default")
        .body(Body::from_stream(chunks))
        .expect("request");

    let response = tokio::time::timeout(
        Duration::from_secs(5),
        router(gateway(address).await).oneshot(request),
    )
    .await
    .expect("the proxy buffered the upload")
    .expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);
    let value: serde_json::Value =
        serde_json::from_slice(&body_bytes(response).await).expect("json");
    assert_eq!(value["bytes"], 11);
    handle.abort();
}

#[tokio::test]
async fn an_encoded_parent_path_never_reaches_the_upstream() {
    let fake = Fake::default();
    let hits = Arc::clone(&fake.hits);
    let (address, handle) = start(fake).await;
    let response = router(gateway(address).await)
        .oneshot(
            Request::get("/catalog/%2e%2e/internal/v1/profile/datasets/count")
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(hits.load(Ordering::SeqCst), 0);
    handle.abort();
}

#[tokio::test]
async fn a_closed_upstream_is_service_unavailable() {
    // Bind and drop a listener so the address is known to be closed.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let address = listener.local_addr().expect("address");
    drop(listener);

    let response = router(gateway(address).await)
        .oneshot(
            Request::get("/catalog/datasets")
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("service.unavailable"), "{text}");
    assert!(text.contains("data"), "{text}");
}

#[tokio::test]
async fn an_over_limit_upload_keeps_the_413_envelope() {
    let app = Router::new()
        .route(
            "/catalog/datasets",
            post(|| async {
                (
                    StatusCode::PAYLOAD_TOO_LARGE,
                    Json(ErrorEnvelope::of(
                        ErrorCode::UploadTooLarge,
                        "the upload exceeds the limit",
                    )),
                )
            }),
        )
        .fallback(|uri: Uri| async move { (StatusCode::NOT_FOUND, uri.to_string()) });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let address = listener.local_addr().expect("address");
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("upstream");
    });

    let response = router(gateway(address).await)
        .oneshot(
            Request::post("/catalog/datasets?name=large.csv")
                .header(PROFILE_HEADER, "default")
                .header("content-type", "text/csv")
                .body(Body::from("candles"))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("upload.too_large"), "{text}");
    assert!(text.contains("quantrule.api/v1"), "{text}");
    handle.abort();
}

#[tokio::test]
async fn the_gateway_source_has_no_data_handler() {
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
    assert!(!manifest.join("src/data").exists(), "src/data still exists");
    assert!(
        !manifest.join("src/configs").exists(),
        "src/configs still exists"
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
            "catalog.datasets",
            "catalog.configs",
            "PgDatasetCatalog",
            "PgConfigStore",
            "MemoryCatalog",
            "MemoryConfigs",
            "ingest_tolerant(",
        ] {
            assert!(
                !text.contains(forbidden),
                "{relative} still names {forbidden}"
            );
        }
    }
}
