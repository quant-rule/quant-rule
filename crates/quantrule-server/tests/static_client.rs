//! The gateway's static client, served from the fixture directory.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::body_bytes;
use quantrule_server::runtime::server::router;
use quantrule_server::runtime::{AppState, Config, HealthRegistry, Services};
use std::path::Path;
use std::sync::Arc;
use tower::ServiceExt;

fn state() -> Arc<AppState> {
    let config = Config {
        static_dir: Some(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/static")),
        ..Config::default()
    };
    Arc::new(AppState::with_services(
        &config,
        HealthRegistry::new(),
        Services::default(),
    ))
}

async fn get(path: &str) -> axum::response::Response {
    router(state())
        .oneshot(Request::get(path).body(Body::empty()).expect("request"))
        .await
        .expect("response")
}

#[tokio::test]
async fn the_root_serves_the_entry_point() {
    let response = get("/").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok()),
        Some("text/html; charset=utf-8")
    );
    let body = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(body.contains("Quant Rule"), "{body}");
}

#[tokio::test]
async fn an_asset_is_served_as_a_javascript_module() {
    let response = get("/assets/app.js").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok()),
        Some("text/javascript; charset=utf-8")
    );
    assert_eq!(
        response
            .headers()
            .get("cache-control")
            .and_then(|value| value.to_str().ok()),
        Some("public, max-age=31536000, immutable")
    );
}

#[tokio::test]
async fn an_unknown_page_falls_back_to_the_entry_point() {
    let response = get("/runs/abc").await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(body.contains("Quant Rule"), "{body}");
}

#[tokio::test]
async fn an_api_path_never_falls_back() {
    let response = get("/api/v1/nope").await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_write_method_is_not_served_the_spa() {
    let response = router(state())
        .oneshot(
            Request::post("/runs/abc")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn without_a_static_dir_nothing_is_served() {
    let state = Arc::new(AppState::new(&Config::default(), HealthRegistry::new()));
    let response = router(state)
        .oneshot(Request::get("/").body(Body::empty()).expect("request"))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
