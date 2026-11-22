//! Shared harness and helpers for the research-route tests.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use crate::common::body_bytes;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use quantrule_api::{
    ArtifactRef, Digest, ImprovementPartition, ImprovementRunDetail, ImprovementTier,
    PROFILE_HEADER,
};
use quantrule_evidence_service::leader::LeaderLease;
use quantrule_evidence_service::research::{ImprovementStore, MemoryImprovementStore};
use quantrule_evidence_service::runs::{MemoryRuns, NewRun, RunStore, derive};
use quantrule_evidence_service::{EvidenceServices, EvidenceState, router};
use quantrule_runtime::adapters::memory::MemoryObjects;
use quantrule_runtime::adapters::{Backend, BoxFuture, ObjectStore, Queue, QueueError};
use quantrule_runtime::client::MemoryDataClient;
use quantrule_runtime::{Config, HealthRegistry};
use quantrule_service_api::{ConfigBundle, DatasetObject, RunManifest};
use tower::ServiceExt;

type Messages = Arc<Mutex<Vec<(String, Vec<u8>)>>>;

#[derive(Clone, Default)]
pub struct MemoryQueue {
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

impl MemoryQueue {
    pub fn messages(&self) -> Vec<(String, Vec<u8>)> {
        self.inner.lock().expect("the queue lock").clone()
    }
}

pub struct Harness {
    pub state: Arc<EvidenceState>,
    pub improvements: Arc<MemoryImprovementStore>,
    pub runs: Arc<MemoryRuns>,
    pub objects: MemoryObjects,
    pub queue: MemoryQueue,
    pub data: Arc<MemoryDataClient>,
    pub data_version: Digest,
    pub config_hash: Digest,
    pub image_digest: Digest,
}

pub fn profile(name: &str) -> quantrule_api::ProfileId {
    quantrule_api::ProfileId::parse(name).expect("profile")
}

fn dataset(version: &Digest) -> DatasetObject {
    DatasetObject {
        data_version: version.clone(),
        object_key: format!("datasets/{}", version.as_hex()),
        sha256: version.clone(),
        candles: 10,
        first_bar: 1,
        last_bar: 10,
        bytes: 100,
    }
}

fn config(id: &str, seed: &[u8], coupled: bool) -> ConfigBundle {
    let run_json = if coupled {
        serde_json::json!({"model_id": "M1.0", "strategy": "strategy.json"}).to_string()
    } else {
        serde_json::json!({"model_id": "M1.0"}).to_string()
    };
    ConfigBundle {
        config_id: id.to_owned(),
        files: BTreeMap::from([
            ("run.json".to_owned(), run_json),
            (
                "data.json".to_owned(),
                r#"{"from":"2018-01-01","to":"2020-12-31"}"#.to_owned(),
            ),
        ]),
        valid: true,
        message: None,
        config_hash: Some(Digest::sha256(seed)),
    }
}

/// A coupled config with an explicit data window, for the partition test.
pub fn config_with_window(id: &str, seed: &[u8], from: &str, to: &str) -> ConfigBundle {
    let run_json = serde_json::json!({"model_id": "M1.0", "strategy": "strategy.json"}).to_string();
    ConfigBundle {
        config_id: id.to_owned(),
        files: BTreeMap::from([
            ("run.json".to_owned(), run_json),
            (
                "data.json".to_owned(),
                serde_json::json!({"from": from, "to": to}).to_string(),
            ),
        ]),
        valid: true,
        message: None,
        config_hash: Some(Digest::sha256(seed)),
    }
}

pub fn harness() -> Harness {
    let data = Arc::new(MemoryDataClient::default());
    let data_version = Digest::sha256(b"research dataset");
    let config_hash = Digest::sha256(b"coupled config");
    for owner in ["default", "alpha"] {
        let owner = profile(owner);
        data.add_dataset(&owner, dataset(&data_version));
        data.add_config(&owner, config("cfg_full", b"coupled config", true));
        data.add_config(&owner, config("cfg_model_only", b"model only", false));
    }
    let improvements = Arc::new(MemoryImprovementStore::default());
    let runs = Arc::new(MemoryRuns::default());
    let objects = MemoryObjects::default();
    let queue = MemoryQueue::default();
    let image_digest = Digest::sha256(b"no-image-digest");
    let config = Config {
        image_digest: Some(image_digest.clone()),
        ..Config::default()
    };
    let state = Arc::new(EvidenceState::with_services(
        &config,
        HealthRegistry::new(),
        EvidenceServices {
            improvements: Some(improvements.clone()),
            runs: Some(runs.clone()),
            outbox: Some(runs.clone()),
            releases: Some(runs.clone()),
            objects: Some(Arc::new(objects.clone())),
            queue: Some(Arc::new(queue.clone())),
            data: Some(data.clone()),
            ..EvidenceServices::default()
        },
    ));
    Harness {
        state,
        improvements,
        runs,
        objects,
        queue,
        data,
        data_version,
        config_hash,
        image_digest,
    }
}

pub fn create_body(tier: ImprovementTier, partition: ImprovementPartition) -> serde_json::Value {
    let criteria = match tier {
        ImprovementTier::Model => serde_json::Value::Null,
        _ => serde_json::json!("criteria_1"),
    };
    serde_json::json!({
        "tier": tier.as_str(),
        "hypothesis": "the pre-registered research hypothesis holds",
        "partition": partition.as_str(),
        "data_version": Digest::sha256(b"research dataset").to_string(),
        "config_id": "cfg_full",
        "criteria_id": criteria,
        "green_criteria": {"return_target": 0.1},
        "baseline": null,
        "code_commit": "c236539f",
        "seed": 0,
        "analytics": true
    })
}

pub async fn send(
    state: Arc<EvidenceState>,
    owner: &str,
    method: Method,
    path: &str,
    body: Option<serde_json::Value>,
) -> axum::response::Response {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header(PROFILE_HEADER, owner);
    let body = match body {
        Some(value) => {
            request = request.header("content-type", "application/json");
            Body::from(value.to_string())
        }
        None => Body::empty(),
    };
    router(state)
        .oneshot(request.body(body).expect("request"))
        .await
        .expect("response")
}

pub async fn create(
    harness: &Harness,
    owner: &str,
    tier: ImprovementTier,
    partition: ImprovementPartition,
) -> ImprovementRunDetail {
    let response = send(
        harness.state.clone(),
        owner,
        Method::POST,
        "/research/runs",
        Some(create_body(tier, partition)),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    serde_json::from_slice(&body_bytes(response).await).expect("detail")
}

pub async fn show(harness: &Harness, owner: &str, id: &str) -> ImprovementRunDetail {
    let response = send(
        harness.state.clone(),
        owner,
        Method::GET,
        &format!("/research/runs/{id}"),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    serde_json::from_slice(&body_bytes(response).await).expect("detail")
}

fn new_run(harness: &Harness, seed: &[u8]) -> NewRun {
    let data_version = harness.data_version.clone();
    let config_hash = harness.config_hash.clone();
    let image_digest = harness.image_digest.clone();
    let run_id = derive(&data_version, &config_hash, "M1.0", &image_digest);
    let _ = seed;
    NewRun {
        run_id,
        config_id: "cfg_full".to_owned(),
        data_version,
        config_hash,
        model_version: "M1.0".to_owned(),
        image_digest,
        model_artifact_id: None,
        owner: "default".to_owned(),
        submitted_at: "2026-11-17T00:00:00Z".to_owned(),
    }
}

pub async fn artifact(harness: &Harness, name: &str, value: serde_json::Value) -> ArtifactRef {
    let bytes = serde_json::to_vec(&value).expect("json");
    let sha256 = Digest::sha256(&bytes);
    harness
        .objects
        .put(&RunManifest::blob_key(&sha256), bytes.clone())
        .await
        .expect("object");
    ArtifactRef {
        name: name.to_owned(),
        bytes: bytes.len() as u64,
        sha256,
    }
}

pub async fn succeed_run(
    harness: &Harness,
    artifacts: Vec<ArtifactRef>,
    digest: Digest,
) -> quantrule_api::RunId {
    let owner = profile("default");
    let run = new_run(harness, b"run");
    let outcome = harness
        .runs
        .submit(owner.clone(), run)
        .await
        .expect("submit");
    let run_id = outcome.record().run_id.clone();
    let term = claim_term(harness, &run_id.to_string()).await;
    let leader_term = finalizer_term(harness).await;
    harness
        .runs
        .complete(
            run_id.to_string(),
            term,
            leader_term,
            digest,
            artifacts,
            Vec::new(),
        )
        .await
        .expect("complete");
    run_id
}

async fn finalizer_term(harness: &Harness) -> u64 {
    let lease = harness.runs.leader_lease();
    if let Some(status) = lease.current().await.expect("leader status")
        && status.live
    {
        return status.term;
    }
    lease
        .acquire("research-leader".to_owned())
        .await
        .expect("leader lease")
        .expect("this process is the leader")
}

async fn claim_term(harness: &Harness, run_id: &str) -> u64 {
    let outcome = harness
        .runs
        .claim(run_id.to_owned(), "research-worker".to_owned(), None)
        .await
        .expect("claim");
    match outcome {
        quantrule_evidence_service::runs::ClaimOutcome::Claimed(record)
        | quantrule_evidence_service::runs::ClaimOutcome::Running(record)
        | quantrule_evidence_service::runs::ClaimOutcome::Terminal(record)
        | quantrule_evidence_service::runs::ClaimOutcome::Exhausted(record) => record.lease_term,
        quantrule_evidence_service::runs::ClaimOutcome::Missing => {
            panic!("the run is missing")
        }
    }
}

pub async fn succeed_seeded(
    harness: &Harness,
    seed: &[u8],
    artifacts: Vec<ArtifactRef>,
    digest: Digest,
) -> quantrule_api::RunId {
    let owner = profile("default");
    let data_version = Digest::sha256(seed);
    let config_hash = Digest::sha256(seed);
    let run_id = derive(&data_version, &config_hash, "M1.0", &harness.image_digest);
    let run = NewRun {
        run_id,
        config_id: "cfg_full".to_owned(),
        data_version,
        config_hash,
        model_version: "M1.0".to_owned(),
        image_digest: harness.image_digest.clone(),
        model_artifact_id: None,
        owner: "default".to_owned(),
        submitted_at: "2026-11-17T00:00:00Z".to_owned(),
    };
    let outcome = harness
        .runs
        .submit(owner.clone(), run)
        .await
        .expect("submit");
    let run_id = outcome.record().run_id.clone();
    let term = claim_term(harness, &run_id.to_string()).await;
    let leader_term = finalizer_term(harness).await;
    harness
        .runs
        .complete(
            run_id.to_string(),
            term,
            leader_term,
            digest,
            artifacts,
            Vec::new(),
        )
        .await
        .expect("complete");
    run_id
}

pub async fn fail_run(harness: &Harness) -> quantrule_api::RunId {
    let owner = profile("default");
    let run = new_run(harness, b"run");
    let outcome = harness
        .runs
        .submit(owner.clone(), run)
        .await
        .expect("submit");
    let run_id = outcome.record().run_id.clone();
    let term = claim_term(harness, &run_id.to_string()).await;
    harness
        .runs
        .fail(
            run_id.to_string(),
            term,
            "the run failed".to_owned(),
            Vec::new(),
        )
        .await
        .expect("fail");
    run_id
}

pub async fn link(
    harness: &Harness,
    id: &str,
    run_id: quantrule_api::RunId,
) -> ImprovementRunDetail {
    harness
        .improvements
        .link_run(
            profile("default"),
            id.to_owned(),
            run_id,
            "2026-11-17T00:05:00Z".to_owned(),
        )
        .await
        .expect("link")
}

pub async fn link_digest(harness: &Harness, id: &str, digest: Digest) {
    harness
        .improvements
        .link_digest(profile("default"), id.to_owned(), digest)
        .await
        .expect("digest");
}

pub async fn error_code(response: axum::response::Response) -> String {
    let value: serde_json::Value =
        serde_json::from_slice(&body_bytes(response).await).expect("error json");
    value["error"]["code"]
        .as_str()
        .unwrap_or("missing")
        .to_owned()
}
