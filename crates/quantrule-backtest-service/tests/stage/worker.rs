//! The diagnostic stage worker over in-memory services.

use std::sync::Arc;

use quantrule_api::StageState;
use quantrule_backtest_service::stage::{StageProcessOutcome, process_stage};
use quantrule_runtime::adapters::ObjectStore;
use quantrule_runtime::client::StageClient;
use quantrule_service_api::StageManifest;

use crate::support::{
    CancelDuring, Failure, Fixture, Success, dispatch, expected_artifact, manifest_key, output,
    services_with_manifest_failure,
};

#[tokio::test]
async fn a_stage_succeeds_and_is_idempotent() {
    let fixture = Fixture::new(b"success").await;
    let outcome = process_stage(
        fixture.stage_run_id.clone(),
        false,
        &fixture.services,
        &dispatch(Arc::new(Success(output().files))),
    )
    .await
    .expect("process");
    assert_eq!(outcome, StageProcessOutcome::Computed);

    let record = fixture
        .evidence
        .get_stage(fixture.stage_run_id.clone())
        .await
        .expect("get")
        .expect("record");
    assert_eq!(record.state, StageState::Succeeded);
    assert_eq!(record.artifact, Some(expected_artifact()));
    assert_eq!(record.attempts, 1);
    assert!(
        fixture
            .objects
            .exists(&manifest_key(&record))
            .await
            .expect("manifest")
    );
    for file in &expected_artifact().files {
        assert!(
            fixture
                .objects
                .exists(&StageManifest::blob_key(&file.sha256))
                .await
                .expect("blob"),
            "{} is missing",
            file.name
        );
    }
    assert!(
        fixture
            .objects
            .keys()
            .iter()
            .all(|key| key.starts_with("stages/")),
        "the stage worker wrote a non-stage object"
    );
    assert!(
        fixture
            .objects
            .keys()
            .iter()
            .all(|key| !key.starts_with("runs/") && !key.starts_with("artifacts/")),
        "the stage worker wrote a run artifact"
    );
    let messages = fixture.queue.payloads("stage.computed");
    assert_eq!(messages.len(), 1);
    assert_eq!(fixture.queue.keys(), vec!["stage.computed".to_owned()]);
    let value: serde_json::Value = serde_json::from_slice(&messages[0]).expect("payload");
    assert_eq!(
        value["stage_run_id"].as_str(),
        Some(fixture.stage_run_id.as_str())
    );
    assert_eq!(value.as_object().map(serde_json::Map::len), Some(1));

    // A second delivery does not execute the stage again; it only republishes
    // the notification to close a lost publish.
    let outcome = process_stage(
        fixture.stage_run_id.clone(),
        true,
        &fixture.services,
        &dispatch(Arc::new(Failure("must not run"))),
    )
    .await
    .expect("second");
    assert_eq!(outcome, StageProcessOutcome::Skipped);
    assert_eq!(fixture.queue.payloads("stage.computed").len(), 2);
    let record = fixture
        .evidence
        .get_stage(fixture.stage_run_id.clone())
        .await
        .expect("get")
        .expect("record");
    assert_eq!(record.artifact, Some(expected_artifact()));
}

#[tokio::test]
async fn an_executor_failure_records_the_error_and_publishes_the_failure() {
    let fixture = Fixture::new(b"failure").await;
    let outcome = process_stage(
        fixture.stage_run_id.clone(),
        false,
        &fixture.services,
        &dispatch(Arc::new(Failure("the model refused the input"))),
    )
    .await
    .expect("process");
    assert_eq!(outcome, StageProcessOutcome::Failed);
    let record = fixture
        .evidence
        .get_stage(fixture.stage_run_id.clone())
        .await
        .expect("get")
        .expect("record");
    assert_eq!(record.state, StageState::Failed);
    assert_eq!(record.error.as_deref(), Some("the model refused the input"));
    assert!(record.artifact.is_none());
    assert!(
        !fixture
            .objects
            .exists(&manifest_key(&record))
            .await
            .expect("manifest")
    );
    let messages = fixture.queue.payloads("stage.failed");
    assert_eq!(messages.len(), 1);
    let value: serde_json::Value = serde_json::from_slice(&messages[0]).expect("payload");
    assert_eq!(
        value["stage_run_id"].as_str(),
        Some(fixture.stage_run_id.as_str())
    );
    assert_eq!(value.as_object().map(serde_json::Map::len), Some(1));
}

#[tokio::test]
async fn a_stage_cancelled_before_the_claim_is_not_executed() {
    let fixture = Fixture::new(b"cancel-before").await;
    fixture
        .evidence
        .cancel(
            &quantrule_api::ProfileId::parse("default").expect("profile"),
            fixture.stage_run_id.clone(),
        )
        .await
        .expect("cancel");
    let outcome = process_stage(
        fixture.stage_run_id.clone(),
        false,
        &fixture.services,
        &dispatch(Arc::new(Success(output().files))),
    )
    .await
    .expect("process");
    assert_eq!(outcome, StageProcessOutcome::Skipped);
    let record = fixture
        .evidence
        .get_stage(fixture.stage_run_id.clone())
        .await
        .expect("get")
        .expect("record");
    assert_eq!(record.state, StageState::Cancelled);
    assert!(record.artifact.is_none());
    assert!(
        !fixture
            .objects
            .exists(&manifest_key(&record))
            .await
            .expect("manifest")
    );
}

#[tokio::test]
async fn a_cancel_before_the_first_cas_write_leaves_no_manifest() {
    let fixture = Fixture::new(b"cancel-mid").await;
    let executor = CancelDuring {
        evidence: fixture.evidence.clone(),
        stage_run_id: fixture.stage_run_id.clone(),
    };
    let outcome = process_stage(
        fixture.stage_run_id.clone(),
        false,
        &fixture.services,
        &dispatch(Arc::new(executor)),
    )
    .await
    .expect("process");
    assert_eq!(outcome, StageProcessOutcome::Skipped);
    let record = fixture
        .evidence
        .get_stage(fixture.stage_run_id.clone())
        .await
        .expect("get")
        .expect("record");
    assert_eq!(record.state, StageState::Cancelled);
    assert!(record.artifact.is_none());
    assert!(
        !fixture
            .objects
            .exists(&manifest_key(&record))
            .await
            .expect("manifest"),
        "a cancelled stage exposed a manifest"
    );
    assert!(
        fixture
            .objects
            .keys()
            .iter()
            .all(|key| !key.ends_with("/manifest.json")),
        "a cancelled stage exposed an artifact"
    );
}

#[tokio::test]
async fn a_killed_worker_leaves_no_visible_artifact_and_may_retry() {
    let fixture = Fixture::new(b"partial").await;
    let services = services_with_manifest_failure(&fixture);
    let outcome = process_stage(
        fixture.stage_run_id.clone(),
        false,
        &services,
        &dispatch(Arc::new(Success(output().files))),
    )
    .await
    .expect("process");
    assert!(matches!(outcome, StageProcessOutcome::Retry { .. }));
    let record = fixture
        .evidence
        .get_stage(fixture.stage_run_id.clone())
        .await
        .expect("get")
        .expect("record");
    assert_eq!(record.state, StageState::Running);
    assert!(record.artifact.is_none());
    assert!(
        !fixture
            .objects
            .exists(&manifest_key(&record))
            .await
            .expect("manifest"),
        "a killed worker exposed a manifest"
    );
}

#[tokio::test]
async fn a_kind_without_an_executor_fails_instead_of_succeeding() {
    let fixture = Fixture::new(b"no-executor").await;
    let outcome = process_stage(
        fixture.stage_run_id.clone(),
        false,
        &fixture.services,
        &quantrule_backtest_service::stage::StageDispatch::new(),
    )
    .await
    .expect("process");
    assert_eq!(outcome, StageProcessOutcome::Failed);
    let record = fixture
        .evidence
        .get_stage(fixture.stage_run_id.clone())
        .await
        .expect("get")
        .expect("record");
    assert_eq!(record.state, StageState::Failed);
    assert!(
        record
            .error
            .as_deref()
            .is_some_and(|error| error.contains("no executor")),
        "{:?}",
        record.error
    );
}
