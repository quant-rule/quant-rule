//! Stage submission, listing, detail, artifacts, cancellation, and cleanup.

mod common;
mod stage_support;

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::body_bytes;
use quantrule_api::{
    ArtifactRef, Digest, ErrorCode, ErrorEnvelope, PROFILE_HEADER, StageArtifact, StageKind,
    StageRunDetail, StageRunSummary, StageState, StageSubmitResponse,
};
use quantrule_evidence_service::router;
use quantrule_evidence_service::stage::StageStore;
use quantrule_runtime::adapters::ObjectStore;
use quantrule_service_api::{StageComputed, StageManifest, derive_stage_run_id};
use stage_support::{
    completed_stage, completed_stage_with, completed_strategy_stage, digest, harness, json,
    model_body, submit,
};
use tower::ServiceExt;

#[tokio::test]
async fn a_model_stage_is_content_addressed_queued_and_listed() {
    let harness = harness();
    let data_version = digest(b"candles-a");
    let body = model_body(&data_version);
    let response = submit(Arc::clone(&harness.state), "default", "model", &body).await;
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let created: StageSubmitResponse = json(response).await;
    assert!(created.created);
    assert_eq!(created.state, StageState::Queued);
    assert_eq!(
        created.stage_run_id,
        derive_stage_run_id(
            StageKind::Model,
            &quantrule_api::StageInput::DataVersion {
                data_version: data_version.clone(),
            },
            &digest(b"valid"),
            "M1.3",
            &Digest::sha256(b"no-image-digest"),
        )
    );

    let response = submit(Arc::clone(&harness.state), "default", "model", &body).await;
    assert_eq!(response.status(), StatusCode::OK);
    let existing: StageSubmitResponse = json(response).await;
    assert!(!existing.created);
    assert_eq!(existing.stage_run_id, created.stage_run_id);

    let messages = harness.queue.messages();
    assert_eq!(messages.len(), 2);
    for (key, payload) in &messages {
        assert_eq!(key, "stage.requested");
        let value: serde_json::Value = serde_json::from_slice(payload).expect("payload");
        assert_eq!(
            value["stage_run_id"].as_str(),
            Some(created.stage_run_id.to_string().as_str())
        );
        assert_eq!(
            value.as_object().map(serde_json::Map::len),
            Some(1),
            "a stage message carries the stage id only"
        );
    }

    let response = router(Arc::clone(&harness.state))
        .oneshot(
            Request::get("/stages/runs?kind=model&status=queued&limit=10")
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let listed: Vec<StageRunSummary> = json(response).await;
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].stage_run_id, created.stage_run_id);
    assert_eq!(listed[0].state, StageState::Queued);
    assert!(!listed[0].evidence);

    let response = router(Arc::clone(&harness.state))
        .oneshot(
            Request::get(format!("/stages/runs/{}", created.stage_run_id))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let detail: StageRunDetail = json(response).await;
    assert_eq!(detail.state, StageState::Queued);
    assert_eq!(detail.provenance.model_version, "M1.3");
    assert_eq!(detail.provenance.data_version, data_version);
    assert_eq!(detail.owner, "default");
    assert!(!detail.evidence);
    assert!(detail.artifact.is_none());
    let text = serde_json::to_string(&detail).expect("detail");
    assert!(!text.contains("\"run_id\""));
    assert!(!text.contains("canonical"));
}

#[tokio::test]
async fn a_stage_is_profile_scoped_and_never_readable_as_a_run() {
    let harness = harness();
    let data_version = digest(b"candles-a");
    let response = submit(
        Arc::clone(&harness.state),
        "default",
        "model",
        &model_body(&data_version),
    )
    .await;
    let created: StageSubmitResponse = json(response).await;

    for (profile, expected) in [
        ("alpha", StatusCode::NOT_FOUND),
        ("default", StatusCode::OK),
    ] {
        let response = router(Arc::clone(&harness.state))
            .oneshot(
                Request::get(format!("/stages/runs/{}", created.stage_run_id))
                    .header(PROFILE_HEADER, profile)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), expected, "{profile}");
    }

    let response = router(Arc::clone(&harness.state))
        .oneshot(
            Request::get(format!("/runs/{}", created.stage_run_id))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let envelope: ErrorEnvelope = json(response).await;
    assert_eq!(envelope.error.code, ErrorCode::RunNotFound);
}

#[tokio::test]
async fn a_strategy_stage_derives_its_dataset_from_the_upstream_model_stage() {
    let harness = harness();
    let unknown = digest(b"model");
    let body = serde_json::json!({
        "kind": "strategy",
        "config_id": "cfg_valid",
        "input": {
            "kind": "model_artifact",
            "model_artifact_id": unknown,
        },
    });
    let response = submit(Arc::clone(&harness.state), "default", "strategy", &body).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let envelope: ErrorEnvelope = json(response).await;
    assert_eq!(envelope.error.code, ErrorCode::StageNotFound);

    let (_, upstream) = completed_stage(&harness, &[("readings.csv", b"a\n")]).await;
    let model_artifact = upstream.artifact_id.clone();
    let mut body = serde_json::json!({
        "kind": "strategy",
        "config_id": "cfg_valid",
        "input": {
            "kind": "model_artifact",
            "model_artifact_id": model_artifact,
        },
    });
    let response = submit(Arc::clone(&harness.state), "default", "strategy", &body).await;
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let created: StageSubmitResponse = json(response).await;
    assert_eq!(
        created.stage_run_id,
        derive_stage_run_id(
            StageKind::Strategy,
            &quantrule_api::StageInput::ModelArtifact {
                model_artifact_id: model_artifact.clone(),
            },
            &digest(b"valid"),
            "M1.3",
            &Digest::sha256(b"no-image-digest"),
        )
    );
    let response = router(Arc::clone(&harness.state))
        .oneshot(
            Request::get(format!("/stages/runs/{}", created.stage_run_id))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let detail: StageRunDetail = json(response).await;
    assert_eq!(detail.provenance.data_version, digest(b"candles-a"));
    assert_eq!(detail.provenance.model_version, "M1.3");

    body["data_version"] = serde_json::json!(digest(b"other"));
    let response = submit(Arc::clone(&harness.state), "default", "strategy", &body).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let envelope: ErrorEnvelope = json(response).await;
    assert_eq!(envelope.error.code, ErrorCode::StageInvalid);
}

#[tokio::test]
async fn a_strategy_stage_requires_the_upstream_model_version() {
    let harness = harness();
    let (_, upstream) =
        completed_stage_with(&harness, "cfg_m99", &[("readings.csv", b"a\n")]).await;
    let body = serde_json::json!({
        "kind": "strategy",
        "config_id": "cfg_valid",
        "input": {
            "kind": "model_artifact",
            "model_artifact_id": upstream.artifact_id,
        },
    });
    let response = submit(Arc::clone(&harness.state), "default", "strategy", &body).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let envelope: ErrorEnvelope = json(response).await;
    assert_eq!(envelope.error.code, ErrorCode::StageInvalid);
    assert!(
        envelope.error.message.contains("model_version"),
        "{}",
        envelope.error.message
    );
}

#[tokio::test]
async fn a_portfolio_stage_derives_its_dataset_from_the_upstream_strategy_stage() {
    let harness = harness();
    let (_, model) = completed_stage(&harness, &[("readings.csv", b"a\n")]).await;
    let (_, strategy) = completed_strategy_stage(&harness, &model.artifact_id, "cfg_valid").await;
    let decisions_artifact = strategy.artifact_id.clone();

    let unknown = digest(b"missing-decisions");
    let body = serde_json::json!({
        "kind": "portfolio",
        "config_id": "cfg_valid",
        "input": {
            "kind": "decisions_artifact",
            "decisions_artifact_id": unknown,
        },
    });
    let response = submit(Arc::clone(&harness.state), "default", "portfolio", &body).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let envelope: ErrorEnvelope = json(response).await;
    assert_eq!(envelope.error.code, ErrorCode::StageNotFound);

    let model_artifact = model.artifact_id.clone();
    let wrong_kind = serde_json::json!({
        "kind": "portfolio",
        "config_id": "cfg_valid",
        "input": {
            "kind": "decisions_artifact",
            "decisions_artifact_id": model_artifact,
        },
    });
    let response = submit(
        Arc::clone(&harness.state),
        "default",
        "portfolio",
        &wrong_kind,
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let envelope: ErrorEnvelope = json(response).await;
    assert_eq!(envelope.error.code, ErrorCode::StageNotFound);

    let mut body = serde_json::json!({
        "kind": "portfolio",
        "config_id": "cfg_valid",
        "input": {
            "kind": "decisions_artifact",
            "decisions_artifact_id": decisions_artifact.clone(),
        },
    });
    let response = submit(Arc::clone(&harness.state), "default", "portfolio", &body).await;
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let created: StageSubmitResponse = json(response).await;
    assert_eq!(
        created.stage_run_id,
        derive_stage_run_id(
            StageKind::Portfolio,
            &quantrule_api::StageInput::DecisionsArtifact {
                decisions_artifact_id: decisions_artifact.clone(),
            },
            &digest(b"valid"),
            "M1.3",
            &Digest::sha256(b"no-image-digest"),
        )
    );
    let response = router(Arc::clone(&harness.state))
        .oneshot(
            Request::get(format!("/stages/runs/{}", created.stage_run_id))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let detail: StageRunDetail = json(response).await;
    assert_eq!(detail.provenance.data_version, digest(b"candles-a"));
    assert_eq!(detail.provenance.model_version, "M1.3");

    body["data_version"] = serde_json::json!(digest(b"other"));
    let response = submit(Arc::clone(&harness.state), "default", "portfolio", &body).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let envelope: ErrorEnvelope = json(response).await;
    assert_eq!(envelope.error.code, ErrorCode::StageInvalid);

    let mut wrong_profile = body.clone();
    wrong_profile["data_version"] = serde_json::Value::Null;
    let response = submit(
        Arc::clone(&harness.state),
        "alpha",
        "portfolio",
        &wrong_profile,
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_portfolio_stage_requires_the_upstream_strategy_model_version() {
    let harness = harness();
    let (_, model) = completed_stage(&harness, &[("readings.csv", b"a\n")]).await;
    let (_, strategy) = completed_strategy_stage(&harness, &model.artifact_id, "cfg_valid").await;
    let body = serde_json::json!({
        "kind": "portfolio",
        "config_id": "cfg_m99",
        "input": {
            "kind": "decisions_artifact",
            "decisions_artifact_id": strategy.artifact_id,
        },
    });
    let response = submit(Arc::clone(&harness.state), "default", "portfolio", &body).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let envelope: ErrorEnvelope = json(response).await;
    assert_eq!(envelope.error.code, ErrorCode::StageInvalid);
    assert!(
        envelope.error.message.contains("model_version"),
        "{}",
        envelope.error.message
    );
}

#[tokio::test]
async fn the_submission_validates_the_config_dataset_kind_and_provenance() {
    let harness = harness();
    let data_version = digest(b"candles-a");

    let mut missing_dataset = model_body(&digest(b"missing"));
    missing_dataset["input"]["data_version"] = serde_json::json!(digest(b"missing"));
    let response = submit(
        Arc::clone(&harness.state),
        "default",
        "model",
        &missing_dataset,
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let envelope: ErrorEnvelope = json(response).await;
    assert_eq!(envelope.error.code, ErrorCode::DatasetNotFound);

    let mut invalid_config = model_body(&data_version);
    invalid_config["config_id"] = serde_json::json!("cfg_invalid");
    let response = submit(
        Arc::clone(&harness.state),
        "default",
        "model",
        &invalid_config,
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let envelope: ErrorEnvelope = json(response).await;
    assert_eq!(envelope.error.code, ErrorCode::ConfigInvalid);

    let response = submit(
        Arc::clone(&harness.state),
        "default",
        "strategy",
        &model_body(&data_version),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let envelope: ErrorEnvelope = json(response).await;
    assert_eq!(envelope.error.code, ErrorCode::StageInvalid);

    let mut wrong_hash = model_body(&data_version);
    wrong_hash["config_hash"] = serde_json::json!(digest(b"forged"));
    let response = submit(Arc::clone(&harness.state), "default", "model", &wrong_hash).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let mut wrong_version = model_body(&data_version);
    wrong_version["model_version"] = serde_json::json!("M9.9");
    let response = submit(
        Arc::clone(&harness.state),
        "default",
        "model",
        &wrong_version,
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let mut mismatched_input = model_body(&data_version);
    mismatched_input["data_version"] = serde_json::json!(digest(b"other"));
    let response = submit(
        Arc::clone(&harness.state),
        "default",
        "model",
        &mismatched_input,
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn the_internal_lifecycle_records_a_verified_artifact() {
    let harness = harness();
    let data_version = digest(b"candles-a");
    let response = submit(
        Arc::clone(&harness.state),
        "default",
        "model",
        &model_body(&data_version),
    )
    .await;
    let created: StageSubmitResponse = json(response).await;
    let id = created.stage_run_id.to_string();

    let response = router(Arc::clone(&harness.state))
        .oneshot(
            Request::post(format!("/internal/v1/stages/{id}/claim"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "stage_run_id": id,
                        "redelivered": false,
                        "holder": "stage-route-test",
                        "continuation": null,
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let claim: quantrule_service_api::StageClaimOutcome = json(response).await;
    let quantrule_service_api::StageClaimOutcome::Claimed(claim) = claim else {
        panic!("the stage was not claimed");
    };
    let lease_term = claim.lease_term;

    let response = router(Arc::clone(&harness.state))
        .oneshot(
            Request::get(format!("/stages/runs/{id}"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    let detail: StageRunDetail = json(response).await;
    let bytes = b"{\"coverage\":1}".to_vec();
    let file = ArtifactRef {
        name: "results.json".to_owned(),
        bytes: bytes.len() as u64,
        sha256: Digest::sha256(&bytes),
    };
    let artifact = StageArtifact::from_files(vec![file.clone()]).expect("artifact");
    let manifest = StageManifest::new(
        detail.stage_run_id.clone(),
        detail.kind,
        detail.config_id.clone(),
        &detail.provenance,
        detail.input.clone(),
        artifact.files.clone(),
    )
    .expect("manifest");
    harness
        .objects
        .put(&StageManifest::blob_key(&file.sha256), bytes)
        .await
        .expect("blob");
    harness
        .objects
        .put(
            &StageManifest::key(&detail.stage_run_id),
            manifest.to_bytes().expect("bytes"),
        )
        .await
        .expect("manifest");

    let response = router(Arc::clone(&harness.state))
        .oneshot(
            Request::post(format!("/internal/v1/stages/{id}/computed"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&StageComputed {
                        lease_term,
                        stage_run_id: detail.stage_run_id.clone(),
                        artifact: artifact.clone(),
                    })
                    .expect("body"),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let record: quantrule_service_api::StageRecord = json(response).await;
    assert_eq!(record.state, StageState::Succeeded);
    assert_eq!(record.artifact, Some(artifact.clone()));
    assert_eq!(record.attempts, 1);
    assert_eq!(record.provenance, detail.provenance);
    assert!(!record.detail().evidence);

    let response = router(Arc::clone(&harness.state))
        .oneshot(
            Request::get(format!("/stages/runs/{id}/artifacts"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let exposed: StageArtifact = json(response).await;
    assert_eq!(exposed, artifact);
}

#[tokio::test]
async fn a_stage_artifact_is_downloaded_by_its_recorded_hash() {
    let harness = harness();
    let (id, artifact) = completed_stage(
        &harness,
        &[
            ("events.csv", b"timeframe_secs,event_index\n300,0\n"),
            ("report.json", b"{\"timeframes\":[]}"),
        ],
    )
    .await;
    let response = router(Arc::clone(&harness.state))
        .oneshot(
            Request::get(format!("/stages/runs/{id}/artifacts/events.csv"))
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
            .get("content-type")
            .and_then(|value| value.to_str().ok()),
        Some("text/csv")
    );
    let expected_etag = format!("\"{}\"", artifact.files[0].sha256);
    assert_eq!(
        response
            .headers()
            .get("etag")
            .and_then(|value| value.to_str().ok()),
        Some(expected_etag.as_str())
    );
    assert_eq!(
        body_bytes(response).await,
        b"timeframe_secs,event_index\n300,0\n"
    );

    for (profile, expected) in [
        ("alpha", StatusCode::NOT_FOUND),
        ("default", StatusCode::OK),
    ] {
        let response = router(Arc::clone(&harness.state))
            .oneshot(
                Request::get(format!("/stages/runs/{id}/artifacts/report.json"))
                    .header(PROFILE_HEADER, profile)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), expected, "{profile}");
    }

    let response = router(Arc::clone(&harness.state))
        .oneshot(
            Request::get(format!("/runs/{id}/artifacts/events.csv"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let envelope: ErrorEnvelope = json(response).await;
    assert_eq!(envelope.error.code, ErrorCode::RunNotFound);
}

#[tokio::test]
async fn a_stage_artifact_download_refuses_unknown_names_and_traversal() {
    let harness = harness();
    let (id, _) = completed_stage(&harness, &[("events.csv", b"a\n")]).await;
    let response = router(Arc::clone(&harness.state))
        .oneshot(
            Request::get(format!("/stages/runs/{id}/artifacts/missing.csv"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    for name in ["../secret", "a//b", "a/./b", "/etc/passwd"] {
        let response = router(Arc::clone(&harness.state))
            .oneshot(
                Request::get(format!(
                    "/stages/runs/{id}/artifacts/{}",
                    name.replace('\\', "%5C")
                ))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(
            response.status(),
            StatusCode::BAD_REQUEST,
            "{name} was not refused"
        );
    }
    let missing_id = digest(b"missing-stage").to_string();
    let response = router(Arc::clone(&harness.state))
        .oneshot(
            Request::get(format!("/stages/runs/{missing_id}/artifacts/events.csv"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_changed_stage_artifact_is_refused() {
    let harness = harness();
    let (id, artifact) = completed_stage(&harness, &[("events.csv", b"a\n")]).await;
    harness
        .objects
        .put(
            &StageManifest::blob_key(&artifact.files[0].sha256),
            b"changed\n".to_vec(),
        )
        .await
        .expect("overwrite");
    let response = router(Arc::clone(&harness.state))
        .oneshot(
            Request::get(format!("/stages/runs/{id}/artifacts/events.csv"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let envelope: ErrorEnvelope = json(response).await;
    assert_eq!(envelope.error.code, ErrorCode::Internal);
}

#[tokio::test]
async fn the_internal_failure_route_records_the_failure() {
    let harness = harness();
    let data_version = digest(b"candles-a");
    let response = submit(
        Arc::clone(&harness.state),
        "default",
        "model",
        &model_body(&data_version),
    )
    .await;
    let created: StageSubmitResponse = json(response).await;
    let id = created.stage_run_id.to_string();
    let quantrule_service_api::StageClaimOutcome::Claimed(claimed) = harness
        .stages
        .claim(id.clone(), "stage-route-test".to_owned(), None)
        .await
        .expect("claim")
    else {
        panic!("the stage was not claimed");
    };
    let response = router(Arc::clone(&harness.state))
        .oneshot(
            Request::post(format!("/internal/v1/stages/{id}/failed"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&quantrule_service_api::StageFailed {
                        stage_run_id: created.stage_run_id.clone(),
                        lease_term: claimed.lease_term,
                        error: "the model failed".to_owned(),
                    })
                    .expect("body"),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let record: quantrule_service_api::StageRecord = json(response).await;
    assert_eq!(record.state, StageState::Failed);
    assert_eq!(record.error.as_deref(), Some("the model failed"));
    assert!(record.artifact.is_none());
}

#[tokio::test]
async fn artifacts_are_exposed_only_after_success_and_cancellation_is_idempotent() {
    let harness = harness();
    let data_version = digest(b"candles-a");
    let response = submit(
        Arc::clone(&harness.state),
        "default",
        "model",
        &model_body(&data_version),
    )
    .await;
    let created: StageSubmitResponse = json(response).await;
    let id = created.stage_run_id.to_string();

    let response = router(Arc::clone(&harness.state))
        .oneshot(
            Request::get(format!("/stages/runs/{id}/artifacts"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::CONFLICT);

    let response = router(Arc::clone(&harness.state))
        .oneshot(
            Request::post(format!("/stages/runs/{id}/cancel"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let cancelled: StageRunDetail = json(response).await;
    assert_eq!(cancelled.state, StageState::Cancelled);
    let cancel = harness
        .queue
        .messages()
        .into_iter()
        .filter(|(key, _)| key == "stage.cancelled")
        .collect::<Vec<_>>();
    assert_eq!(cancel.len(), 1);
    let value: serde_json::Value = serde_json::from_slice(&cancel[0].1).expect("payload");
    assert_eq!(value["stage_run_id"].as_str(), Some(id.as_str()));
    assert_eq!(value.as_object().map(serde_json::Map::len), Some(1));

    let response = router(Arc::clone(&harness.state))
        .oneshot(
            Request::post(format!("/stages/runs/{id}/cancel"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn an_explicit_delete_removes_only_the_stage_and_its_objects() {
    let harness = harness();
    let data_version = digest(b"candles-a");
    let response = submit(
        Arc::clone(&harness.state),
        "default",
        "model",
        &model_body(&data_version),
    )
    .await;
    let created: StageSubmitResponse = json(response).await;
    let id = created.stage_run_id.to_string();
    let manifest_key = StageManifest::key(&created.stage_run_id);
    harness
        .state
        .objects
        .as_ref()
        .expect("objects")
        .put(&manifest_key, b"manifest".to_vec())
        .await
        .expect("put");

    let response = router(Arc::clone(&harness.state))
        .oneshot(
            Request::delete(format!("/stages/runs/{id}"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert!(
        harness
            .stages
            .get_any(id.clone())
            .await
            .expect("get")
            .is_none()
    );
    assert!(
        !harness
            .state
            .objects
            .as_ref()
            .expect("objects")
            .exists(&manifest_key)
            .await
            .expect("exists")
    );

    let response = router(Arc::clone(&harness.state))
        .oneshot(
            Request::delete(format!("/stages/runs/{id}"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
