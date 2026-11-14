//! The M1 health and version surface.
//!
//! The router is exercised in process, so the test binds no socket and proves
//! the two routes without a live dependency.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use quantrule_api::{API_VERSION, HealthReport, ServiceState, VersionInfo};
use quantrule_model_service::router;
use quantrule_runtime::{Config, HealthRegistry};
use tower::ServiceExt;

async fn body(response: axum::response::Response) -> Vec<u8> {
    axum::body::to_bytes(response.into_body(), 65_536)
        .await
        .expect("body")
        .to_vec()
}

#[tokio::test]
async fn health_answers_with_the_contract_report() {
    let response = router(&Config::default(), HealthRegistry::new())
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
async fn version_reports_the_contract() {
    let response = router(&Config::default(), HealthRegistry::new())
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
