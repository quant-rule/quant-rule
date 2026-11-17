//! Happy-path runs, duplicate delivery, and CAS publication.

use super::support::*;

#[tokio::test]
async fn a_queued_run_computes_artifacts_and_digest() {
    let fixture = fixture().await;
    let outcome = process(fixture.run_id.clone(), false, &fixture.services)
        .await
        .expect("process");
    assert_eq!(
        outcome,
        ProcessOutcome::Computed,
        "{:?}",
        fixture
            .runs
            .get_any(fixture.run_id.clone())
            .await
            .unwrap()
            .unwrap()
            .error
    );
    let record = fixture
        .runs
        .get_any(fixture.run_id.clone())
        .await
        .expect("get")
        .expect("record");
    assert_eq!(record.status, RunState::Running);
    assert!(record.digest.is_some());
    assert!(!record.artifacts.is_empty());
    let evidence_stage = record
        .stages
        .iter()
        .find(|stage| stage.name == quantrule_api::StageName::Evidence)
        .expect("evidence stage");
    assert_eq!(evidence_stage.state, quantrule_api::StageStatus::Pending);
    for artifact in &record.artifacts {
        let key = RunManifest::blob_key(&artifact.sha256);
        let bytes = fixture.objects.get(&key).await.expect("artifact");
        assert_eq!(Digest::sha256(&bytes), artifact.sha256);
    }
    let manifest_key = RunManifest::key(&fixture.run_id);
    let manifest_bytes = fixture.objects.get(&manifest_key).await.expect("manifest");
    let manifest = RunManifest::from_bytes(&manifest_bytes).expect("manifest decode");
    assert_eq!(manifest.refs(), record.artifacts);
    assert_eq!(manifest.digest, record.digest.clone().expect("digest"));
    assert!(
        !fixture
            .objects
            .exists(&format!("runs/{}/results.json", fixture.run_id))
            .await
            .expect("per-name key")
    );
    let messages = fixture.queue.payloads("run.computed");
    assert_eq!(messages.len(), 1);
    let payload: serde_json::Value = serde_json::from_slice(&messages[0]).expect("payload");
    assert_eq!(payload, serde_json::json!({ "run_id": fixture.run_id }));
}

#[tokio::test]
async fn a_second_delivery_does_not_execute_again() {
    let fixture = fixture().await;
    assert_eq!(
        process(fixture.run_id.clone(), false, &fixture.services)
            .await
            .expect("first"),
        ProcessOutcome::Computed
    );
    assert_eq!(fixture.model.calls(), 1);
    let first = fixture
        .runs
        .get_any(fixture.run_id.clone())
        .await
        .expect("get")
        .expect("record");
    assert_eq!(
        process(fixture.run_id.clone(), false, &fixture.services)
            .await
            .expect("second"),
        ProcessOutcome::Skipped
    );
    let second = fixture
        .runs
        .get_any(fixture.run_id.clone())
        .await
        .expect("get")
        .expect("record");
    assert_eq!(first.artifacts, second.artifacts);
    assert_eq!(second.status, RunState::Running);
    assert_eq!(
        process(fixture.run_id.clone(), true, &fixture.services)
            .await
            .expect("redelivery"),
        ProcessOutcome::Skipped
    );
    assert_eq!(fixture.model.calls(), 1);
    let third = fixture
        .runs
        .get_any(fixture.run_id.clone())
        .await
        .expect("get")
        .expect("record");
    assert_eq!(third.artifacts, first.artifacts);
    assert_eq!(third.attempts, 1);
    // A delivery after compute republishes so evidence still hears about a run
    // whose first publish was lost; it never executes it again.
    assert_eq!(fixture.queue.payloads("run.computed").len(), 3);
}

#[tokio::test]
async fn a_redelivery_executes_a_run_left_running_by_a_killed_worker() {
    let fixture = fixture().await;
    fixture
        .runs
        .claim(fixture.run_id.clone(), false)
        .await
        .expect("simulated claim");
    let outcome = process(fixture.run_id.clone(), true, &fixture.services)
        .await
        .expect("process");
    assert_eq!(outcome, ProcessOutcome::Computed);
    let record = fixture
        .runs
        .get_any(fixture.run_id.clone())
        .await
        .expect("get")
        .expect("record");
    assert_eq!(record.status, RunState::Running);
    assert_eq!(record.attempts, 2);
    assert!(record.digest.is_some());
    assert!(!record.artifacts.is_empty());
}

#[tokio::test]
async fn a_stopped_write_leaves_no_manifest() {
    let fixture = fixture().await;
    let services = services_with_stop_after(&fixture, 1);
    assert_eq!(
        process(fixture.run_id.clone(), false, &services)
            .await
            .expect("process"),
        ProcessOutcome::Retry {
            delay: Duration::from_secs(5)
        }
    );
    let record = fixture
        .runs
        .get_any(fixture.run_id.clone())
        .await
        .expect("get")
        .expect("record");
    assert_eq!(record.status, RunState::Running);
    assert!(record.artifacts.is_empty());
    assert!(
        fixture
            .objects
            .keys()
            .iter()
            .any(|key| key.starts_with("artifacts/"))
    );
    let run_prefix = format!("runs/{}/", fixture.run_id);
    assert!(
        fixture
            .objects
            .keys()
            .iter()
            .all(|key| !key.starts_with(&run_prefix)),
        "a stopped write must not publish a run prefix"
    );
}

#[tokio::test]
async fn a_manifest_write_failure_leaves_no_published_set() {
    let fixture = fixture().await;
    let services = services_with_manifest_failure(&fixture);
    assert_eq!(
        process(fixture.run_id.clone(), false, &services)
            .await
            .expect("first"),
        ProcessOutcome::Retry {
            delay: Duration::from_secs(5)
        }
    );
    let record = fixture
        .runs
        .get_any(fixture.run_id.clone())
        .await
        .expect("get")
        .expect("record");
    assert_eq!(record.status, RunState::Running);
    assert!(record.artifacts.is_empty());
    assert!(
        !fixture
            .objects
            .exists(&RunManifest::key(&fixture.run_id))
            .await
            .expect("manifest")
    );
    assert!(
        fixture
            .objects
            .keys()
            .iter()
            .any(|key| key.starts_with("artifacts/"))
    );
    let run_prefix = format!("runs/{}/", fixture.run_id);
    assert!(
        fixture
            .objects
            .keys()
            .iter()
            .all(|key| !key.starts_with(&run_prefix)),
        "a killed worker must not publish a run prefix"
    );

    // Restart on the durable store: the blobs are reused and the set publishes.
    assert_eq!(
        process(fixture.run_id.clone(), true, &fixture.services)
            .await
            .expect("restart"),
        ProcessOutcome::Computed
    );
}

#[tokio::test]
async fn a_stage_failure_retries_then_dead_letters() {
    let fixture = fixture().await;
    let services = services_with_put_failure(&fixture);
    assert_eq!(
        process(fixture.run_id.clone(), false, &services)
            .await
            .expect("first"),
        ProcessOutcome::Retry {
            delay: Duration::from_secs(5)
        }
    );
    let record = fixture
        .runs
        .get_any(fixture.run_id.clone())
        .await
        .expect("get")
        .expect("record");
    assert_eq!(record.status, RunState::Running);
    assert_eq!(record.attempts, 1);
    assert!(!fixture.queue.keys().contains(&"run.dead".to_owned()));

    assert_eq!(
        process(fixture.run_id.clone(), true, &services)
            .await
            .expect("second"),
        ProcessOutcome::Retry {
            delay: Duration::from_secs(25)
        }
    );
    assert_eq!(
        process(fixture.run_id.clone(), true, &services)
            .await
            .expect("third"),
        ProcessOutcome::Exhausted
    );
    let record = fixture
        .runs
        .get_any(fixture.run_id.clone())
        .await
        .expect("get")
        .expect("record");
    assert_eq!(record.status, RunState::Failed);
    assert_eq!(record.attempts, 3);
    assert!(
        record
            .error
            .as_deref()
            .is_some_and(|error| error.contains("artifact store"))
    );
    assert!(record.artifacts.is_empty());
    assert!(
        !fixture
            .objects
            .exists(&RunManifest::key(&fixture.run_id))
            .await
            .expect("manifest")
    );
    assert!(
        !fixture
            .objects
            .exists(&format!("runs/{}/results.json", fixture.run_id))
            .await
            .expect("per-name key")
    );
    assert!(fixture.queue.keys().contains(&"run.dead".to_owned()));
    let messages = fixture.queue.payloads("run.dead");
    assert_eq!(messages.len(), 1);
    let payload: serde_json::Value = serde_json::from_slice(&messages[0]).expect("payload");
    assert_eq!(payload, serde_json::json!({ "run_id": fixture.run_id }));
}

#[tokio::test]
async fn a_run_cancelled_after_claim_writes_no_bundle() {
    let fixture = fixture().await;
    fixture
        .runs
        .claim(fixture.run_id.clone(), false)
        .await
        .expect("claim");
    fixture
        .runs
        .cancel(
            &ProfileId::parse("default").expect("profile"),
            fixture.run_id.clone(),
        )
        .await
        .expect("cancel");
    assert_eq!(
        process(fixture.run_id.clone(), false, &fixture.services)
            .await
            .expect("process"),
        ProcessOutcome::Skipped
    );
    let record = fixture
        .runs
        .get_any(fixture.run_id.clone())
        .await
        .expect("get")
        .expect("record");
    assert_eq!(record.status, RunState::Cancelled);
    assert!(record.artifacts.is_empty());
    assert!(
        !fixture
            .objects
            .exists(&RunManifest::key(&fixture.run_id))
            .await
            .expect("manifest")
    );
    assert!(
        !fixture
            .objects
            .exists(&format!("runs/{}/results.json", fixture.run_id))
            .await
            .expect("exists")
    );
}

#[tokio::test]
async fn a_cancelled_run_is_not_executed() {
    let fixture = fixture().await;
    fixture
        .runs
        .cancel(
            &ProfileId::parse("default").expect("profile"),
            fixture.run_id.clone(),
        )
        .await
        .expect("cancel");
    assert_eq!(
        process(fixture.run_id.clone(), false, &fixture.services)
            .await
            .expect("process"),
        ProcessOutcome::Skipped
    );
    let record = fixture
        .runs
        .get_any(fixture.run_id)
        .await
        .expect("get")
        .expect("record");
    assert_eq!(record.status, RunState::Cancelled);
    assert!(record.artifacts.is_empty());
}

#[tokio::test]
async fn a_missing_dataset_fails_the_run() {
    let fixture = fixture().await;
    let run_id = submit_with(
        &fixture.runs,
        Digest::sha256(b"missing-dataset"),
        fixture.config_id.clone(),
        fixture.config_hash.clone(),
        "missing",
    )
    .await;
    assert_eq!(
        process(run_id.clone(), false, &fixture.services)
            .await
            .expect("process"),
        ProcessOutcome::Failed
    );
    let record = fixture
        .runs
        .get_any(run_id.clone())
        .await
        .expect("get")
        .expect("record");
    assert_eq!(record.status, RunState::Failed);
    assert!(
        record
            .error
            .as_deref()
            .is_some_and(|error| error.contains("dataset"))
    );
    let messages = fixture.queue.payloads("run.failed");
    assert_eq!(messages.len(), 1);
    let payload: serde_json::Value = serde_json::from_slice(&messages[0]).expect("payload");
    assert_eq!(payload, serde_json::json!({ "run_id": run_id }));
}

#[tokio::test]
async fn an_invalid_config_fails_the_run() {
    let fixture = fixture().await;
    let run_id = submit_with(
        &fixture.runs,
        fixture.data_version.clone(),
        fixture.invalid_config_id.clone(),
        Digest::sha256(b"invalid"),
        "invalid",
    )
    .await;
    assert_eq!(
        process(run_id.clone(), false, &fixture.services)
            .await
            .expect("process"),
        ProcessOutcome::Failed
    );
    let record = fixture
        .runs
        .get_any(run_id)
        .await
        .expect("get")
        .expect("record");
    assert_eq!(record.status, RunState::Failed);
    assert!(record.artifacts.is_empty());
}

#[tokio::test]
async fn a_queued_run_matches_the_coupled_baseline() {
    let fixture = fixture().await;
    // The coupled `backtest run` digest for the 250-minute fixture. It is the
    // regression oracle: a stage rule, a reading, or an artifact change fails
    // this line before it can reach a run.
    assert_eq!(
        fixture.baseline_digest.to_string(),
"sha256:32dc2c72bfa54569067b0beedacd2c97e06d87532da6f32930f068732e73e26f"
    );
    assert_eq!(
        process(fixture.run_id.clone(), false, &fixture.services)
            .await
            .expect("process"),
        ProcessOutcome::Computed
    );
    let record = fixture
        .runs
        .get_any(fixture.run_id.clone())
        .await
        .expect("get")
        .expect("record");
    assert_eq!(record.digest.as_ref(), Some(&fixture.baseline_digest));
    assert_eq!(fixture.model.calls(), 1);
    assert_eq!(record.artifacts.len(), fixture.baseline.len());
    for (name, bytes) in &fixture.baseline {
        let artifact = record
            .artifacts
            .iter()
            .find(|artifact| artifact.name == *name)
            .unwrap_or_else(|| panic!("the worker has no {name}"));
        assert_eq!(artifact.bytes, bytes.len() as u64, "{name}");
        assert_eq!(artifact.sha256, Digest::sha256(bytes), "{name}");
        let stored = fixture
            .objects
            .get(&RunManifest::blob_key(&artifact.sha256))
            .await
            .expect("stored artifact");
        assert_eq!(&stored, bytes, "{name}");
    }
}
