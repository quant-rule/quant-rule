//! Runtime-history tests for evidence runs.
//!
//! The store tests exercise archive, protected states, shared submissions, and
//! artifact reference counting. The route test proves the archive/erase
//! confirmation, object cleanup, and dataset-reference release.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::body_bytes;
use quantrule_api::{Digest, PROFILE_HEADER, RunState};
use quantrule_evidence_service::runs::{MemoryRuns, RunStore, derive};
use quantrule_evidence_service::{EvidenceServices, EvidenceState, router};
use quantrule_runtime::adapters::ObjectStore;
use quantrule_runtime::client::MemoryDataClient;
use quantrule_runtime::{Config, HealthRegistry};
use quantrule_service_api::{DatasetObject, NewRun};
use std::sync::Arc;
use tower::ServiceExt;

fn new_run(seed: &[u8]) -> NewRun {
    let data_version = Digest::sha256(seed);
    let config_hash = Digest::sha256(b"config");
    let image = Digest::sha256(b"image");
    NewRun {
        run_id: derive(&data_version, &config_hash, "M1.0", &image),
        config_id: "cfg_abc".to_owned(),
        data_version,
        config_hash,
        model_version: "M1.0".to_owned(),
        image_digest: image,
        model_artifact_id: None,
        owner: "default".to_owned(),
        submitted_at: "2026-11-14T00:00:00Z".to_owned(),
    }
}

#[tokio::test]
async fn a_failed_run_is_archived_and_erased_with_its_unshared_blob() {
    use quantrule_api::ArtifactRef;
    use quantrule_evidence_service::runs::{RunArchiveOutcome, RunEraseOutcome};

    let runs = MemoryRuns::default();
    let owner = common::profile("retention-run");
    let submitted = runs
        .submit(owner.clone(), new_run(b"retention-run"))
        .await
        .expect("submit");
    let id = submitted.record().run_id.to_string();
    runs.claim(id.clone(), false).await.expect("claim");
    let bytes = b"{\"retention\":true}".to_vec();
    let artifact = ArtifactRef {
        name: "results.json".to_owned(),
        bytes: bytes.len() as u64,
        sha256: Digest::sha256(&bytes),
    };
    runs.computed(
        id.clone(),
        Digest::sha256(b"retention-digest"),
        vec![artifact.clone()],
        Vec::new(),
    )
    .await
    .expect("computed");
    runs.fail(id.clone(), "the run failed".to_owned(), Vec::new())
        .await
        .expect("fail");

    assert_eq!(
        runs.erase(owner.clone(), id.clone()).await.expect("erase"),
        RunEraseOutcome::NotArchived
    );
    let RunArchiveOutcome::Archived(summary) = runs
        .archive(owner.clone(), id.clone(), "2026-11-20T00:00:00Z".to_owned())
        .await
        .expect("archive")
    else {
        panic!("the run was not archived");
    };
    assert_eq!(summary.archived_at.as_deref(), Some("2026-11-20T00:00:00Z"));
    assert!(
        runs.list(owner.clone(), None, 50)
            .await
            .expect("active list")
            .is_empty()
    );
    assert_eq!(
        runs.list_including_archived(owner.clone(), None, 50)
            .await
            .expect("history list")
            .len(),
        1
    );
    let RunEraseOutcome::Erased(receipt) =
        runs.erase(owner.clone(), id.clone()).await.expect("erase")
    else {
        panic!("the archived run was not erased");
    };
    assert!(receipt.removed_run);
    assert_eq!(receipt.blobs, vec![artifact.sha256]);
    assert_eq!(receipt.bytes, artifact.bytes);
    assert!(
        runs.get(owner, id).await.expect("get").is_none(),
        "the erased run is still visible"
    );
}

#[tokio::test]
async fn a_succeeded_run_refuses_erase() {
    use quantrule_evidence_service::runs::{RunArchiveOutcome, RunEraseOutcome};

    let runs = MemoryRuns::default();
    let owner = common::profile("retention-succeeded");
    let submitted = runs
        .submit(owner.clone(), new_run(b"retention-succeeded"))
        .await
        .expect("submit");
    let id = submitted.record().run_id.to_string();
    runs.claim(id.clone(), false).await.expect("claim");
    runs.complete(
        id.clone(),
        Digest::sha256(b"canonical"),
        Vec::new(),
        Vec::new(),
    )
    .await
    .expect("complete");
    assert!(matches!(
        runs.archive(owner.clone(), id.clone(), "2026-11-20T00:00:00Z".to_owned())
            .await
            .expect("archive"),
        RunArchiveOutcome::Archived(_)
    ));
    assert_eq!(
        runs.erase(owner, id).await.expect("erase"),
        RunEraseOutcome::Protected(RunState::Succeeded)
    );
}

#[tokio::test]
async fn a_shared_run_survives_the_first_profile_erase() {
    use quantrule_evidence_service::runs::{RunArchiveOutcome, RunEraseOutcome};

    let runs = MemoryRuns::default();
    let alpha = common::profile("retention-alpha");
    let beta = common::profile("retention-beta");
    for owner in [alpha.clone(), beta.clone()] {
        runs.submit(owner, new_run(b"retention-shared"))
            .await
            .expect("submit");
    }
    let id = runs.list(alpha.clone(), None, 10).await.expect("list")[0]
        .run_id
        .to_string();
    runs.claim(id.clone(), false).await.expect("claim");
    runs.fail(id.clone(), "the run failed".to_owned(), Vec::new())
        .await
        .expect("fail");
    for owner in [alpha.clone(), beta.clone()] {
        assert!(matches!(
            runs.archive(owner, id.clone(), "2026-11-20T00:00:00Z".to_owned())
                .await
                .expect("archive"),
            RunArchiveOutcome::Archived(_)
        ));
    }
    let RunEraseOutcome::Erased(receipt) =
        runs.erase(alpha.clone(), id.clone()).await.expect("erase")
    else {
        panic!("the first submission was not erased");
    };
    assert!(!receipt.removed_run);
    assert!(
        runs.get(beta.clone(), id.clone())
            .await
            .expect("beta read")
            .is_some(),
        "the shared run was removed for the other profile"
    );
    let RunEraseOutcome::Erased(receipt) = runs.erase(beta, id.clone()).await.expect("erase")
    else {
        panic!("the last submission was not erased");
    };
    assert!(receipt.removed_run);
}

#[tokio::test]
async fn the_archive_and_erase_routes_enforce_state_and_confirmation() {
    use quantrule_api::{DatasetReferenceKind, EraseReceipt};
    use quantrule_runtime::client::DataClient;

    let runs = MemoryRuns::default();
    let owner = common::profile("default");
    let submitted = runs
        .submit(owner.clone(), new_run(b"retention-route"))
        .await
        .expect("submit");
    let id = submitted.record().run_id.to_string();
    runs.claim(id.clone(), false).await.expect("claim");
    let bytes = b"{\"route\":true}".to_vec();
    let artifact = quantrule_api::ArtifactRef {
        name: "results.json".to_owned(),
        bytes: bytes.len() as u64,
        sha256: Digest::sha256(&bytes),
    };
    runs.computed(
        id.clone(),
        Digest::sha256(b"route-digest"),
        vec![artifact.clone()],
        Vec::new(),
    )
    .await
    .expect("computed");
    runs.fail(id.clone(), "the run failed".to_owned(), Vec::new())
        .await
        .expect("fail");

    let objects = common::MemoryObjects::default();
    objects
        .put(
            &quantrule_service_api::RunManifest::blob_key(&artifact.sha256),
            bytes,
        )
        .await
        .expect("blob");
    let data = MemoryDataClient::default();
    data.add_dataset(
        &owner,
        DatasetObject {
            data_version: Digest::sha256(b"retention-route"),
            object_key: "datasets/route".to_owned(),
            sha256: Digest::sha256(b"retention-route"),
            candles: 1,
            first_bar: 1,
            last_bar: 1,
            bytes: 1,
        },
    );
    data.register_dataset_reference(
        owner.clone(),
        Digest::sha256(b"retention-route"),
        format!("run:{id}"),
        DatasetReferenceKind::Run,
    )
    .await
    .expect("reference");
    let state = Arc::new(EvidenceState::with_services(
        &Config::default(),
        HealthRegistry::new(),
        EvidenceServices {
            runs: Some(Arc::new(runs) as Arc<dyn RunStore>),
            objects: Some(Arc::new(objects.clone())),
            data: Some(Arc::new(data.clone()) as Arc<dyn DataClient>),
            ..EvidenceServices::default()
        },
    ));

    let response = router(Arc::clone(&state))
        .oneshot(
            Request::post(format!("/runs/{id}/archive"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("archive request"),
        )
        .await
        .expect("archive response");
    assert_eq!(response.status(), StatusCode::OK);

    let response = router(Arc::clone(&state))
        .oneshot(
            Request::delete(format!("/runs/{id}?confirm=sha256:wrong"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("erase request"),
        )
        .await
        .expect("erase response");
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("history.state"), "{text}");

    let response = router(state)
        .oneshot(
            Request::delete(format!("/runs/{id}?confirm={id}"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("erase request"),
        )
        .await
        .expect("erase response");
    assert_eq!(response.status(), StatusCode::OK);
    let receipt: EraseReceipt =
        serde_json::from_slice(&body_bytes(response).await).expect("receipt");
    assert_eq!(receipt.record_id, id);
    assert_eq!(receipt.bytes, artifact.bytes);
    assert!(
        !objects
            .exists(&quantrule_service_api::RunManifest::blob_key(
                &artifact.sha256
            ))
            .await
            .expect("blob exists")
    );
    assert!(
        data.reference_ids(&owner, &Digest::sha256(b"retention-route").to_string())
            .is_empty(),
        "the dataset reference survived the erase"
    );
}
