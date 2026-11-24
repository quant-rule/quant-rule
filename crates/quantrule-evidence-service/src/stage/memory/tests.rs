use super::*;
use quantrule_api::{ArtifactRef, Digest, StageInput, StageKind, StageProvenance};

fn profile(name: &str) -> ProfileId {
    ProfileId::parse(name).expect("profile")
}

fn digest(seed: &[u8]) -> Digest {
    Digest::sha256(seed)
}

fn provenance(seed: &[u8]) -> StageProvenance {
    StageProvenance {
        data_version: digest(seed),
        config_hash: digest(b"config"),
        model_version: "M1.3".to_owned(),
        image_digest: digest(b"image"),
    }
}

fn run(seed: &[u8]) -> NewStageRun {
    NewStageRun::new(
        StageKind::Model,
        "cfg-1".to_owned(),
        StageInput::DataVersion {
            data_version: digest(seed),
        },
        provenance(seed),
        "operator".to_owned(),
        "2026-11-17T00:00:00Z".to_owned(),
    )
}

fn artifact(name_a: &str, name_b: &str) -> StageArtifact {
    StageArtifact::from_files(vec![
        ArtifactRef {
            name: name_b.to_owned(),
            bytes: 2,
            sha256: digest(b"b"),
        },
        ArtifactRef {
            name: name_a.to_owned(),
            bytes: 1,
            sha256: digest(b"a"),
        },
    ])
    .expect("the artifact")
}

#[tokio::test]
async fn a_submission_is_created_once() {
    let store = MemoryStages::default();
    let first = store
        .create(profile("default"), run(b"one"))
        .await
        .expect("first");
    assert!(first.created());
    assert_eq!(first.record().state, StageState::Queued);
    let second = store
        .create(profile("default"), run(b"one"))
        .await
        .expect("second");
    assert!(!second.created());
    assert_eq!(first.record().stage_run_id, second.record().stage_run_id);
    assert_eq!(
        store
            .list(profile("default"), None, None, 50)
            .await
            .expect("list")
            .len(),
        1
    );
}

#[tokio::test]
async fn two_profiles_share_one_stage_and_a_third_cannot_see_it() {
    let store = MemoryStages::default();
    store
        .create(profile("alpha"), run(b"shared"))
        .await
        .expect("alpha");
    store
        .create(profile("beta"), run(b"shared"))
        .await
        .expect("beta");
    let id = run(b"shared").stage_run_id.to_string();
    assert!(
        store
            .get(profile("alpha"), id.clone())
            .await
            .expect("alpha get")
            .is_some()
    );
    assert!(
        store
            .get(profile("beta"), id.clone())
            .await
            .expect("beta get")
            .is_some()
    );
    assert_eq!(
        store
            .get(profile("gamma"), id.clone())
            .await
            .expect("gamma"),
        None
    );
    assert_eq!(
        store
            .list(profile("gamma"), None, None, 50)
            .await
            .expect("gamma list")
            .len(),
        0
    );
    assert!(store.get_any(id).await.expect("worker read").is_some());
}

#[tokio::test]
async fn a_claim_starts_once_and_a_live_lease_is_never_stolen() {
    let store = MemoryStages::default();
    let submitted = store
        .create(profile("default"), run(b"one"))
        .await
        .expect("submit");
    let id = submitted.record().stage_run_id.to_string();
    let StageClaimOutcome::Claimed(first) = store
        .claim(id.clone(), "worker".to_owned(), None)
        .await
        .expect("first")
    else {
        panic!("the queued stage was not claimed");
    };
    assert_eq!(first.state, StageState::Running);
    assert_eq!(first.attempts, 1);
    assert_eq!(first.lease_term, 1);
    assert!(matches!(
        store
            .claim(id.clone(), "other-worker".to_owned(), None)
            .await
            .expect("live"),
        StageClaimOutcome::Running(_)
    ));
    let StageClaimOutcome::Claimed(second) = store
        .claim(id.clone(), "worker".to_owned(), Some(1))
        .await
        .expect("second")
    else {
        panic!("the worker's own continuation was refused");
    };
    assert_eq!(second.attempts, 2);
    assert_eq!(second.lease_term, 2);
    let StageClaimOutcome::Claimed(third) = store
        .claim(id.clone(), "worker".to_owned(), Some(2))
        .await
        .expect("third")
    else {
        panic!("the second continuation was refused");
    };
    assert_eq!(third.attempts, 3);
    assert_eq!(third.lease_term, 3);
    assert!(matches!(
        store
            .claim(id.clone(), "worker".to_owned(), Some(3))
            .await
            .expect("live fourth"),
        StageClaimOutcome::Running(_)
    ));
    store.expire_lease(&id);
    let StageClaimOutcome::Exhausted(spent) = store
        .claim(id, "worker".to_owned(), Some(3))
        .await
        .expect("fourth")
    else {
        panic!("the fourth delivery was not exhausted");
    };
    assert_eq!(spent.attempts, 3);
}

#[tokio::test]
async fn only_a_running_stage_can_complete_or_fail() {
    let store = MemoryStages::default();
    let submitted = store
        .create(profile("default"), run(b"one"))
        .await
        .expect("submit");
    let id = submitted.record().stage_run_id.to_string();
    let artifact = artifact("a.csv", "b.csv");
    let record = store
        .complete(id.clone(), 1, artifact.clone())
        .await
        .expect("too early");
    assert_eq!(record.state, StageState::Queued);
    assert_eq!(record.artifact, None);
    let record = store
        .fail(id.clone(), 1, "too early".to_owned())
        .await
        .expect("too early fail");
    assert_eq!(record.state, StageState::Queued);
    assert_eq!(record.error, None);

    let StageClaimOutcome::Claimed(claimed) = store
        .claim(id.clone(), "worker".to_owned(), None)
        .await
        .expect("claim")
    else {
        panic!("the queued stage was not claimed");
    };
    let record = store
        .complete(id.clone(), claimed.lease_term, artifact.clone())
        .await
        .expect("complete");
    assert_eq!(record.state, StageState::Succeeded);
    assert_eq!(record.artifact, Some(artifact.clone()));
    assert_eq!(
        record
            .artifact
            .as_ref()
            .expect("artifact")
            .files
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["a.csv", "b.csv"]
    );
    let record = store
        .fail(id, claimed.lease_term, "after success".to_owned())
        .await
        .expect("late fail");
    assert_eq!(record.state, StageState::Succeeded);
    assert_eq!(record.error, None);
}

#[tokio::test]
async fn a_failed_stage_records_its_error() {
    let store = MemoryStages::default();
    let submitted = store
        .create(profile("default"), run(b"one"))
        .await
        .expect("submit");
    let id = submitted.record().stage_run_id.to_string();
    let StageClaimOutcome::Claimed(claimed) = store
        .claim(id.clone(), "worker".to_owned(), None)
        .await
        .expect("claim")
    else {
        panic!("the queued stage was not claimed");
    };
    let record = store
        .fail(id, claimed.lease_term, "the model failed".to_owned())
        .await
        .expect("fail");
    assert_eq!(record.state, StageState::Failed);
    assert_eq!(record.error.as_deref(), Some("the model failed"));
}

#[tokio::test]
async fn a_cancelled_stage_is_terminal_and_profile_scoped() {
    let store = MemoryStages::default();
    let submitted = store
        .create(profile("default"), run(b"one"))
        .await
        .expect("submit");
    let id = submitted.record().stage_run_id.to_string();
    assert!(matches!(
        store
            .cancel(profile("other"), id.clone())
            .await
            .expect("other cancel"),
        StageCancelOutcome::Missing
    ));
    assert!(matches!(
        store
            .cancel(profile("default"), id.clone())
            .await
            .expect("cancel"),
        StageCancelOutcome::Cancelled(_)
    ));
    assert!(matches!(
        store
            .cancel(profile("default"), id.clone())
            .await
            .expect("second cancel"),
        StageCancelOutcome::Conflict(_)
    ));
    assert!(matches!(
        store
            .claim(id, "worker".to_owned(), None)
            .await
            .expect("claim"),
        StageClaimOutcome::Terminal(_)
    ));
}

#[tokio::test]
async fn a_forged_artifact_is_refused() {
    let store = MemoryStages::default();
    let submitted = store
        .create(profile("default"), run(b"one"))
        .await
        .expect("submit");
    let id = submitted.record().stage_run_id.to_string();
    let StageClaimOutcome::Claimed(claimed) = store
        .claim(id.clone(), "worker".to_owned(), None)
        .await
        .expect("claim")
    else {
        panic!("the queued stage was not claimed");
    };
    let mut forged = artifact("a.csv", "b.csv");
    forged.artifact_id = digest(b"forged");
    assert!(
        store
            .complete(id, claimed.lease_term, forged)
            .await
            .is_err()
    );
}
