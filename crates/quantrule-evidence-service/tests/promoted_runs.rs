//! Promoted run submission and model-artifact validation.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::body_bytes;
use quantrule_api::{
    ArtifactRef, Digest, ErrorCode, ErrorEnvelope, PROFILE_HEADER, RunDetail, StageArtifact,
    StageKind, StageProvenance, StageState, SubmitRunResponse,
};
use quantrule_evidence_service::runs::MemoryRuns;
use quantrule_evidence_service::stage::{MemoryStages, NewStageRun, StageClaimOutcome, StageStore};
use quantrule_evidence_service::{EvidenceServices, EvidenceState, router};
use quantrule_runtime::adapters::{Backend, BoxFuture, Queue, QueueError};
use quantrule_runtime::client::MemoryDataClient;
use quantrule_runtime::{Config, HealthRegistry};
use quantrule_service_api::{ConfigBundle, DatasetObject};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

type Messages = Arc<Mutex<Vec<(String, Vec<u8>)>>>;

#[derive(Clone, Default)]
struct MemoryQueue {
    inner: Messages,
}

impl Queue for MemoryQueue {
    fn backend(&self) -> Backend {
        Backend::InProcess
    }

    fn publish(&self, key: &str, payload: Vec<u8>) -> BoxFuture<Result<(), QueueError>> {
        let inner = Arc::clone(&self.inner);
        let key = key.to_owned();
        Box::pin(async move {
            inner
                .lock()
                .map_err(|_| QueueError("the queue lock is poisoned".to_owned()))?
                .push((key, payload));
            Ok(())
        })
    }
}

fn digest(seed: &[u8]) -> Digest {
    Digest::sha256(seed)
}

fn dataset(version: &Digest) -> DatasetObject {
    DatasetObject {
        data_version: version.clone(),
        object_key: format!("datasets/{}", version.as_hex()),
        sha256: version.clone(),
        candles: 2,
        first_bar: 1_704_067_200,
        last_bar: 1_704_067_260,
        bytes: 100,
    }
}

fn bundle(config_id: &str, hash_seed: &[u8], model: &str) -> ConfigBundle {
    ConfigBundle {
        config_id: config_id.to_owned(),
        files: BTreeMap::from([(
            "run.json".to_owned(),
            format!(r#"{{"model_id":"{model}"}}"#),
        )]),
        valid: true,
        message: None,
        config_hash: Some(digest(hash_seed)),
    }
}

async fn harness() -> Arc<EvidenceState> {
    let queue = MemoryQueue::default();
    let data = MemoryDataClient::default();
    let stages = MemoryStages::default();
    let default = quantrule_api::ProfileId::parse("default").expect("default");
    let alpha = quantrule_api::ProfileId::parse("alpha").expect("alpha");
    for profile in [&default, &alpha] {
        for version in [digest(b"candles-a"), digest(b"candles-b")] {
            data.add_dataset(profile, dataset(&version));
        }
        data.add_config(profile, bundle("cfg_valid", b"valid", "M1.0"));
        data.add_config(profile, bundle("cfg_changed", b"changed", "M1.0"));
    }
    seed_stages(&stages).await;
    let config = Config {
        image_digest: Some(Digest::sha256(b"image")),
        ..Config::default()
    };
    Arc::new(EvidenceState::with_services(
        &config,
        HealthRegistry::new(),
        EvidenceServices {
            data: Some(Arc::new(data)),
            runs: Some(Arc::new(MemoryRuns::default())),
            stages: Some(Arc::new(stages)),
            queue: Some(Arc::new(queue)),
            ..EvidenceServices::default()
        },
    ))
}

async fn seed_stages(stages: &MemoryStages) {
    for (seed, data, config, model) in [
        (
            b"model".as_slice(),
            b"candles-a".as_slice(),
            b"valid".as_slice(),
            "M1.0",
        ),
        (
            b"model-data-b".as_slice(),
            b"candles-b".as_slice(),
            b"valid".as_slice(),
            "M1.0",
        ),
        (
            b"model-changed".as_slice(),
            b"candles-a".as_slice(),
            b"changed".as_slice(),
            "M1.0",
        ),
        (
            b"model-m99".as_slice(),
            b"candles-a".as_slice(),
            b"valid".as_slice(),
            "M9.9",
        ),
    ] {
        seed_model_stage(stages, seed, &digest(data), &digest(config), model).await;
    }
    seed_strategy_stage(stages).await;
}

fn model_artifact(seed: &[u8]) -> StageArtifact {
    StageArtifact::from_files(vec![ArtifactRef {
        name: "readings.csv".to_owned(),
        bytes: seed.len() as u64,
        sha256: digest(seed),
    }])
    .expect("the model artifact")
}

async fn seed_model_stage(
    stages: &MemoryStages,
    artifact_seed: &[u8],
    data_version: &Digest,
    config_hash: &Digest,
    model_version: &str,
) {
    let provenance = StageProvenance {
        data_version: data_version.clone(),
        config_hash: config_hash.clone(),
        model_version: model_version.to_owned(),
        image_digest: digest(b"image"),
    };
    let run = NewStageRun::new(
        StageKind::Model,
        "cfg_valid".to_owned(),
        quantrule_api::StageInput::DataVersion {
            data_version: data_version.clone(),
        },
        provenance,
        "default".to_owned(),
        "2026-11-17T00:00:00Z".to_owned(),
    );
    let run_id = run.stage_run_id.clone();
    stages
        .create(
            quantrule_api::ProfileId::parse("default").expect("default"),
            run,
        )
        .await
        .expect("the model stage");
    let StageClaimOutcome::Claimed(record) = stages
        .claim(run_id.to_string(), false)
        .await
        .expect("the model claim")
    else {
        panic!("the model stage was not claimed");
    };
    assert_eq!(record.state, StageState::Running);
    stages
        .complete(run_id.to_string(), model_artifact(artifact_seed))
        .await
        .expect("the model stage completion");
}

async fn seed_strategy_stage(stages: &MemoryStages) {
    let provenance = StageProvenance {
        data_version: digest(b"candles-a"),
        config_hash: digest(b"valid"),
        model_version: "M1.0".to_owned(),
        image_digest: digest(b"image"),
    };
    let run = NewStageRun::new(
        StageKind::Strategy,
        "cfg_valid".to_owned(),
        quantrule_api::StageInput::ModelArtifact {
            model_artifact_id: digest(b"model"),
        },
        provenance,
        "default".to_owned(),
        "2026-11-17T00:00:01Z".to_owned(),
    );
    let run_id = run.stage_run_id.clone();
    stages
        .create(
            quantrule_api::ProfileId::parse("default").expect("default"),
            run,
        )
        .await
        .expect("the strategy stage");
    let StageClaimOutcome::Claimed(_) = stages
        .claim(run_id.to_string(), false)
        .await
        .expect("the strategy claim")
    else {
        panic!("the strategy stage was not claimed");
    };
    stages
        .complete(run_id.to_string(), model_artifact(b"strategy"))
        .await
        .expect("the strategy stage completion");
}

async fn submit_promoted(
    state: Arc<EvidenceState>,
    profile: &str,
    data_version: &Digest,
    config_id: &str,
    model_artifact_id: &Digest,
) -> axum::response::Response {
    router(state)
        .oneshot(
            Request::post("/runs")
                .header(PROFILE_HEADER, profile)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "data_version": data_version,
                        "config_id": config_id,
                        "model_artifact_id": model_artifact_id,
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response")
}

async fn decode<T: serde::de::DeserializeOwned>(response: axum::response::Response) -> T {
    serde_json::from_slice(&body_bytes(response).await).expect("json")
}

#[tokio::test]
async fn a_promoted_run_reuses_a_succeeded_model_stage() {
    let state = harness().await;
    let artifact = model_artifact(b"model").artifact_id;
    let response = submit_promoted(
        Arc::clone(&state),
        "default",
        &digest(b"candles-a"),
        "cfg_valid",
        &artifact,
    )
    .await;
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let created: SubmitRunResponse = decode(response).await;
    let response = router(Arc::clone(&state))
        .oneshot(
            Request::get(format!("/runs/{}", created.run_id))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let detail: RunDetail = decode(response).await;
    assert_eq!(detail.model_artifact_id, Some(artifact.clone()));

    let response = submit_promoted(
        Arc::clone(&state),
        "default",
        &digest(b"candles-a"),
        "cfg_valid",
        &artifact,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let again: SubmitRunResponse = decode(response).await;
    assert_eq!(again.run_id, created.run_id);
    assert!(!again.created);
}

#[tokio::test]
async fn a_strategy_cell_may_submit_a_model_stage_from_another_config_hash() {
    let state = harness().await;
    let response = submit_promoted(
        Arc::clone(&state),
        "default",
        &digest(b"candles-a"),
        "cfg_valid",
        &model_artifact(b"model-changed").artifact_id,
    )
    .await;
    // The worker verifies the frozen model-input identity before reuse; the
    // submission check deliberately does not reject a strategy-only config.
    assert_eq!(response.status(), StatusCode::ACCEPTED);
}

#[tokio::test]
async fn a_promoted_run_rejects_an_invalid_model_artifact() {
    let state = harness().await;
    for (artifact, message) in [
        (digest(b"missing"), "unknown"),
        (model_artifact(b"strategy").artifact_id, "another"),
        (model_artifact(b"model-data-b").artifact_id, "dataset"),
        (model_artifact(b"model-m99").artifact_id, "model"),
    ] {
        let response = submit_promoted(
            Arc::clone(&state),
            "default",
            &digest(b"candles-a"),
            "cfg_valid",
            &artifact,
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{message}");
        let envelope: ErrorEnvelope = decode(response).await;
        assert_eq!(envelope.error.code, ErrorCode::RunModelArtifactInvalid);
    }

    let response = submit_promoted(
        Arc::clone(&state),
        "alpha",
        &digest(b"candles-a"),
        "cfg_valid",
        &model_artifact(b"model").artifact_id,
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let envelope: ErrorEnvelope = decode(response).await;
    assert_eq!(envelope.error.code, ErrorCode::RunModelArtifactInvalid);
}
