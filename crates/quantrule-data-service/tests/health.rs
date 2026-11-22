//! The M1 health and version surface.
//!
//! The router is exercised in process, so the test binds no socket and proves
//! the two routes without a live dependency.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use quantrule_api::{API_VERSION, HealthReport, ServiceState, VersionInfo};
use quantrule_data_service::{DataServices, router};
use quantrule_runtime::{Config, HealthRegistry};
use tower::ServiceExt;

fn state() -> std::sync::Arc<quantrule_data_service::DataState> {
    std::sync::Arc::new(quantrule_data_service::DataState::with_services(
        &Config::default(),
        HealthRegistry::new(),
        DataServices::default(),
    ))
}

async fn body(response: axum::response::Response) -> Vec<u8> {
    axum::body::to_bytes(response.into_body(), 65_536)
        .await
        .expect("body")
        .to_vec()
}

#[tokio::test]
async fn health_answers_with_the_contract_report() {
    let response = router(state())
        .oneshot(
            Request::get("/health")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let report: HealthReport = serde_json::from_slice(&body(response).await).expect("report");
    assert_eq!(report.v, API_VERSION);
    assert_eq!(report.state, ServiceState::Healthy);
    assert!(report.services.is_empty());
}

#[tokio::test]
async fn readiness_is_healthy_and_then_draining() {
    let (trigger, shutdown) = quantrule_runtime::Shutdown::channel();
    let state = std::sync::Arc::new(quantrule_data_service::DataState::with_services(
        &Config::default(),
        HealthRegistry::new().with_shutdown(shutdown),
        DataServices::default(),
    ));
    let response = router(std::sync::Arc::clone(&state))
        .oneshot(Request::get("/ready").body(Body::empty()).expect("request"))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);

    trigger.trigger();
    let response = router(std::sync::Arc::clone(&state))
        .oneshot(Request::get("/ready").body(Body::empty()).expect("request"))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let report: HealthReport = serde_json::from_slice(&body(response).await).expect("report");
    assert_eq!(report.services[0].detail.as_deref(), Some("draining"));

    // Liveness still answers while the process drains.
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
    let version: VersionInfo = serde_json::from_slice(&body(response).await).expect("version");
    assert_eq!(version.product, env!("CARGO_PKG_VERSION"));
    assert_eq!(version.api, API_VERSION);
}
