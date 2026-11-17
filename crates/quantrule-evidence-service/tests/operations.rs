//! The operations route: retention days, cleanup states, and profile gating.

mod common;

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::body_bytes;
use quantrule_api::{CleanupReport, OperationsReport, PROFILE_HEADER};
use quantrule_evidence_service::stage::MemoryStages;
use quantrule_evidence_service::{EvidenceServices, EvidenceState, router};
use quantrule_runtime::{Config, HealthRegistry};
use tower::ServiceExt;

fn state(days: u64, stages: Option<Arc<MemoryStages>>) -> Arc<EvidenceState> {
    let config = Config {
        stage_retention_days: days,
        ..Config::default()
    };
    Arc::new(EvidenceState::with_services(
        &config,
        HealthRegistry::new(),
        EvidenceServices {
            stages: stages
                .map(|stages| stages as Arc<dyn quantrule_evidence_service::stage::StageStore>),
            ..EvidenceServices::default()
        },
    ))
}

async fn get(state: Arc<EvidenceState>, profile: Option<&str>) -> axum::response::Response {
    let mut request = Request::get("/operations");
    if let Some(profile) = profile {
        request = request.header(PROFILE_HEADER, profile);
    }
    router(state)
        .oneshot(request.body(Body::empty()).expect("request"))
        .await
        .expect("response")
}

async fn report(response: axum::response::Response) -> OperationsReport {
    serde_json::from_slice(&body_bytes(response).await).expect("operations report")
}

#[tokio::test]
async fn before_the_first_cleanup_the_route_returns_null() {
    let response = get(
        state(7, Some(Arc::new(MemoryStages::default()))),
        Some("default"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let report = report(response).await;
    assert_eq!(report.stage_retention_days, 7);
    assert!(report.stage_store);
    assert!(report.last_cleanup.is_none());
}

#[tokio::test]
async fn a_successful_cleanup_is_reported_with_its_counts() {
    let state = state(30, Some(Arc::new(MemoryStages::default())));
    state.operations.record(CleanupReport {
        completed_utc: "2026-11-17T08:00:00Z".to_owned(),
        stages_removed: 3,
        objects_removed: 4,
        error: None,
    });
    let report = report(get(state, Some("alpha")).await).await;
    assert_eq!(report.stage_retention_days, 30);
    let cleanup = report.last_cleanup.expect("cleanup");
    assert_eq!(cleanup.completed_utc, "2026-11-17T08:00:00Z");
    assert_eq!(cleanup.stages_removed, 3);
    assert_eq!(cleanup.objects_removed, 4);
    assert_eq!(cleanup.error, None);
}

#[tokio::test]
async fn a_failed_cleanup_is_reported_with_its_error() {
    let state = state(30, Some(Arc::new(MemoryStages::default())));
    state.operations.record(CleanupReport {
        completed_utc: "2026-11-17T08:00:00Z".to_owned(),
        stages_removed: 0,
        objects_removed: 0,
        error: Some("the stage store refused a statement".to_owned()),
    });
    let cleanup = report(get(state, Some("default")).await)
        .await
        .last_cleanup
        .expect("cleanup");
    assert_eq!(
        cleanup.error.as_deref(),
        Some("the stage store refused a statement")
    );
    assert_eq!(cleanup.stages_removed, 0);
}

#[tokio::test]
async fn no_stage_store_is_reported_without_a_cleanup() {
    let report = report(get(state(30, None), Some("default")).await).await;
    assert!(!report.stage_store);
    assert!(report.last_cleanup.is_none());
}

#[tokio::test]
async fn the_route_requires_a_profile() {
    let response = get(state(30, Some(Arc::new(MemoryStages::default()))), None).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("profile.not_found"), "{text}");
}
