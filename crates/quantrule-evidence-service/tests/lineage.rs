//! The run lineage store and internal route contracts.
//!
//! The route is service-to-service only. The test records what a run cited,
//! repeats the same set idempotently, refuses a different set, and proves an
//! unknown or not-yet-recorded run is a clean 404.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{body_bytes, profile};
use quantrule_api::{
    Digest, ErrorCode, ErrorEnvelope, PROFILE_HEADER, PackKind, PackProvenance, PackState,
    ReferenceKind, RunArtifactReference, RunId, RunLineage,
};
use quantrule_evidence_service::lineage::{MemoryLineage, PgLineage, RunLineageStore};
use quantrule_evidence_service::migrations;
use quantrule_evidence_service::router;
use quantrule_evidence_service::runs::{MemoryRuns, NewRun, RunStore};
use quantrule_evidence_service::state::{EvidenceServices, EvidenceState};
use quantrule_runtime::adapters::postgres::{
    PgDatabase, apply_migrations_in as apply_postgres_migrations,
};
use quantrule_runtime::{Config, HealthRegistry};
use serde_json::json;
use std::sync::Arc;
use tower::ServiceExt;

fn run(seed: &[u8]) -> NewRun {
    NewRun {
        run_id: RunId(Digest::sha256(seed)),
        config_id: "config".to_owned(),
        data_version: Digest::sha256(b"data"),
        config_hash: Digest::sha256(b"config"),
        model_version: "M1.0".to_owned(),
        image_digest: Digest::sha256(b"image"),
        model_artifact_id: None,
        owner: "operator".to_owned(),
        submitted_at: "2026-11-22T00:00:00Z".to_owned(),
    }
}

fn strategy(seed: &[u8]) -> RunArtifactReference {
    let id = Digest::sha256(seed);
    RunArtifactReference {
        kind: ReferenceKind::StrategyPack,
        id: id.clone(),
        hash: id,
        pack: Some(PackProvenance {
            kind: PackKind::Strategy,
            name: "compression-fade-5m".to_owned(),
            version: "1.0.0".to_owned(),
            owner: "operator".to_owned(),
            state: PackState::Frozen,
        }),
    }
}

fn model(seed: &[u8]) -> RunArtifactReference {
    let id = Digest::sha256(seed);
    RunArtifactReference {
        kind: ReferenceKind::ModelArtifact,
        id: id.clone(),
        hash: id,
        pack: None,
    }
}

fn portfolio(seed: &[u8]) -> RunArtifactReference {
    let id = Digest::sha256(seed);
    RunArtifactReference {
        kind: ReferenceKind::PortfolioPolicy,
        id: id.clone(),
        hash: id,
        pack: Some(PackProvenance {
            kind: PackKind::Portfolio,
            name: "single-position".to_owned(),
            version: "1.0.0".to_owned(),
            owner: "operator".to_owned(),
            state: PackState::Frozen,
        }),
    }
}

async fn state(runs: MemoryRuns, lineage: MemoryLineage) -> Arc<EvidenceState> {
    Arc::new(EvidenceState::with_services(
        &Config::default(),
        HealthRegistry::new(),
        EvidenceServices {
            runs: Some(Arc::new(runs)),
            lineage: Some(Arc::new(lineage)),
            ..EvidenceServices::default()
        },
    ))
}

fn payload(references: &[RunArtifactReference]) -> Body {
    Body::from(serde_json::to_vec(&json!({"references": references})).expect("lineage body"))
}

fn read_error(bytes: &[u8]) -> ErrorEnvelope {
    serde_json::from_slice(bytes).expect("error envelope")
}

#[tokio::test]
async fn a_runs_lineage_is_recorded_read_and_kept_immutable() {
    let runs = MemoryRuns::default();
    let submitted = run(b"lineage");
    runs.submit(profile("default"), submitted.clone())
        .await
        .expect("the run");
    let state = state(runs, MemoryLineage::default()).await;
    let id = submitted.run_id.to_string();
    let references = vec![strategy(b"pack"), model(b"model")];

    let response = router(Arc::clone(&state))
        .oneshot(
            Request::put(format!("/internal/v1/runs/{id}/lineage"))
                .header(PROFILE_HEADER, "default")
                .header("content-type", "application/json")
                .body(payload(&references))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let recorded: RunLineage =
        serde_json::from_slice(&body_bytes(response).await).expect("lineage");
    assert_eq!(recorded.run_id, submitted.run_id);
    assert_eq!(recorded.references.len(), 2);
    assert_eq!(recorded.references[0].kind, ReferenceKind::ModelArtifact);
    assert_eq!(recorded.references[1].kind, ReferenceKind::StrategyPack);
    assert_eq!(
        recorded.references[1]
            .pack
            .as_ref()
            .expect("pack provenance")
            .state,
        PackState::Frozen
    );

    let response = router(Arc::clone(&state))
        .oneshot(
            Request::get(format!("/internal/v1/runs/{id}/lineage"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let read: RunLineage = serde_json::from_slice(&body_bytes(response).await).expect("lineage");
    assert_eq!(read, recorded);

    let response = router(Arc::clone(&state))
        .oneshot(
            Request::put(format!("/internal/v1/runs/{id}/lineage"))
                .header(PROFILE_HEADER, "default")
                .header("content-type", "application/json")
                .body(payload(&references))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let repeated: RunLineage =
        serde_json::from_slice(&body_bytes(response).await).expect("lineage");
    assert_eq!(repeated, recorded);

    let response = router(state)
        .oneshot(
            Request::put(format!("/internal/v1/runs/{id}/lineage"))
                .header(PROFILE_HEADER, "default")
                .header("content-type", "application/json")
                .body(payload(&[strategy(b"other")]))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(
        read_error(&body_bytes(response).await).error.code,
        ErrorCode::RunConflict
    );
}

#[tokio::test]
async fn an_unknown_run_or_an_unrecorded_lineage_is_not_found() {
    let runs = MemoryRuns::default();
    let submitted = run(b"known");
    runs.submit(profile("default"), submitted.clone())
        .await
        .expect("the run");
    let state = state(runs, MemoryLineage::default()).await;

    let response = router(Arc::clone(&state))
        .oneshot(
            Request::get(format!("/internal/v1/runs/{}/lineage", submitted.run_id))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        read_error(&body_bytes(response).await).error.code,
        ErrorCode::LineageNotFound
    );

    let missing = Digest::sha256(b"missing");
    let response = router(Arc::clone(&state))
        .oneshot(
            Request::get(format!("/internal/v1/runs/{missing}/lineage"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        read_error(&body_bytes(response).await).error.code,
        ErrorCode::RunNotFound
    );

    let response = router(state)
        .oneshot(
            Request::put(format!("/internal/v1/runs/{}/lineage", submitted.run_id))
                .header(PROFILE_HEADER, "default")
                .header("content-type", "application/json")
                .body(payload(&[]))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let empty: RunLineage = serde_json::from_slice(&body_bytes(response).await).expect("lineage");
    assert!(empty.references.is_empty());
}

#[tokio::test]
async fn a_malformed_reference_set_is_refused() {
    let runs = MemoryRuns::default();
    let submitted = run(b"invalid");
    runs.submit(profile("default"), submitted.clone())
        .await
        .expect("the run");
    let state = state(runs, MemoryLineage::default()).await;
    let mut wrong = strategy(b"pack");
    wrong.hash = Digest::sha256(b"other");

    let response = router(state)
        .oneshot(
            Request::put(format!("/internal/v1/runs/{}/lineage", submitted.run_id))
                .header(PROFILE_HEADER, "default")
                .header("content-type", "application/json")
                .body(payload(&[wrong]))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        read_error(&body_bytes(response).await).error.code,
        ErrorCode::RunInvalid
    );
}

#[tokio::test]
async fn the_memory_lineage_store_conforms() {
    let store = MemoryLineage::default();
    let run_id = RunId(Digest::sha256(b"conformance"));
    let references = vec![strategy(b"pack")];
    let first = store
        .record(
            profile("conformance-lineage"),
            run_id.clone(),
            references.clone(),
            "2026-11-22T00:00:00Z".to_owned(),
        )
        .await
        .expect("record");
    let again = store
        .record(
            profile("conformance-lineage"),
            run_id.clone(),
            references,
            "2026-11-23T00:00:00Z".to_owned(),
        )
        .await
        .expect("idempotent");
    assert_eq!(again, first);
    assert_eq!(
        store
            .get(profile("conformance-lineage"), run_id.clone())
            .await
            .expect("get"),
        Some(first)
    );
    assert_eq!(
        store
            .get(profile("conformance-lineage-other"), run_id)
            .await
            .expect("other"),
        None
    );
}

#[tokio::test]
#[ignore = "needs QR_TEST_DATABASE_URL pointing at an empty, disposable Postgres"]
async fn the_postgres_lineage_store_conforms() {
    let url = std::env::var("QR_TEST_DATABASE_URL").expect(
        "QR_TEST_DATABASE_URL must name an empty, disposable Postgres database; \
         this test is ignored in the fast gate",
    );
    let database = PgDatabase::connect(&url).await.expect("the Postgres pool");
    apply_postgres_migrations(database.pool(), "runs", &migrations::available())
        .await
        .expect("the evidence migrations");
    let owner = profile("conformance-lineage");
    let other = profile("conformance-lineage-other");
    for statement in [
        "DELETE FROM runs.run_lineage WHERE run_id IN
            (SELECT run_id FROM runs.submissions
              WHERE profile_id IN ('conformance-lineage', 'conformance-lineage-other'))",
        "DELETE FROM runs.outbox WHERE run_id IN
            (SELECT run_id FROM runs.submissions
              WHERE profile_id IN ('conformance-lineage', 'conformance-lineage-other'))",
        "DELETE FROM runs.submissions
          WHERE profile_id IN ('conformance-lineage', 'conformance-lineage-other')",
        "DELETE FROM runs.runs
          WHERE run_id NOT IN (SELECT run_id FROM runs.submissions)",
    ] {
        sqlx::query(statement)
            .execute(database.pool())
            .await
            .expect("the conformance cleanup");
    }
    let runs = quantrule_evidence_service::postgres::PgRunStore::new(database.pool().clone());
    let submitted = run(b"pg-lineage");
    runs.submit(owner.clone(), submitted.clone())
        .await
        .expect("the run");
    let store = PgLineage::new(database.pool().clone());
    let references = vec![strategy(b"pack"), model(b"model"), portfolio(b"portfolio")];
    let first = store
        .record(
            owner.clone(),
            submitted.run_id.clone(),
            references.clone(),
            "2026-11-22T00:00:00Z".to_owned(),
        )
        .await
        .expect("record");
    assert_eq!(first.references.len(), 3);
    assert_eq!(first.references[0].kind, ReferenceKind::ModelArtifact);
    assert_eq!(first.references[1].kind, ReferenceKind::StrategyPack);
    assert_eq!(first.references[2].kind, ReferenceKind::PortfolioPolicy);
    let again = store
        .record(
            owner.clone(),
            submitted.run_id.clone(),
            references,
            "2026-11-23T00:00:00Z".to_owned(),
        )
        .await
        .expect("idempotent");
    assert_eq!(again, first);
    assert!(matches!(
        store
            .record(
                owner.clone(),
                submitted.run_id.clone(),
                vec![strategy(b"other")],
                "2026-11-23T00:00:00Z".to_owned(),
            )
            .await,
        Err(quantrule_evidence_service::lineage::LineageError::Conflict)
    ));
    assert_eq!(
        store
            .get(owner, submitted.run_id.clone())
            .await
            .expect("get"),
        Some(first)
    );
    assert!(matches!(
        store.get(other, submitted.run_id.clone()).await,
        Err(quantrule_evidence_service::lineage::LineageError::Missing)
    ));

    // The foreign key removes the lineage with its run.
    sqlx::query("DELETE FROM runs.runs WHERE run_id = $1")
        .bind(submitted.run_id.to_string())
        .execute(database.pool())
        .await
        .expect("delete the run");
    let remaining: (i64,) =
        sqlx::query_as("SELECT count(*) FROM runs.run_lineage WHERE run_id = $1")
            .bind(submitted.run_id.to_string())
            .fetch_one(database.pool())
            .await
            .expect("the lineage count");
    assert_eq!(remaining.0, 0);
}
