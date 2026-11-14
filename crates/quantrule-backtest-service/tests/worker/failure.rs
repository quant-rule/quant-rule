//! Failure and retry behaviour at each hop.

use super::support::*;

#[tokio::test]
async fn a_data_outage_retries_then_dead_letters() {
    let fixture = fixture().await;
    let data = Arc::new(FailingData {
        inner: MemoryDataClient::default(),
        fail_config: true,
        fail_dataset: false,
    });
    let services = services_with(
        &fixture,
        Arc::clone(&fixture.services.evidence),
        data,
        Arc::clone(&fixture.services.model),
    );
    assert_eq!(
        process(fixture.run_id.clone(), false, &services)
            .await
            .expect("first"),
        ProcessOutcome::Retry {
            delay: Duration::from_secs(5)
        }
    );
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
    assert!(record.artifacts.is_empty());
    assert!(
        record
            .error
            .as_deref()
            .is_some_and(|error| error.contains("data is unreachable"))
    );
    assert!(fixture.queue.keys().contains(&"run.dead".to_owned()));
}

#[tokio::test]
async fn a_model_outage_retries_then_dead_letters() {
    let fixture = fixture().await;
    let model = Arc::new(FailingModel {
        error: ClientError::Timeout {
            service: Service::Model,
        },
    });
    let services = services_with(
        &fixture,
        Arc::clone(&fixture.services.evidence),
        Arc::clone(&fixture.services.data),
        model,
    );
    assert_eq!(
        process(fixture.run_id.clone(), false, &services)
            .await
            .expect("first"),
        ProcessOutcome::Retry {
            delay: Duration::from_secs(5)
        }
    );
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
    assert!(record.artifacts.is_empty());
    assert!(
        record
            .error
            .as_deref()
            .is_some_and(|error| error.contains("did not answer in time"))
    );
    assert!(fixture.queue.keys().contains(&"run.dead".to_owned()));
}

#[tokio::test]
async fn a_model_rejection_fails_the_run_immediately() {
    let fixture = fixture().await;
    let model = Arc::new(FailingModel {
        error: ClientError::Status {
            service: Service::Model,
            status: 400,
            message: "the config is invalid".to_owned(),
        },
    });
    let services = services_with(
        &fixture,
        Arc::clone(&fixture.services.evidence),
        Arc::clone(&fixture.services.data),
        model,
    );
    assert_eq!(
        process(fixture.run_id.clone(), false, &services)
            .await
            .expect("process"),
        ProcessOutcome::Failed
    );
    let record = fixture
        .runs
        .get_any(fixture.run_id.clone())
        .await
        .expect("get")
        .expect("record");
    assert_eq!(record.status, RunState::Failed);
    assert!(record.artifacts.is_empty());
    assert!(
        record
            .error
            .as_deref()
            .is_some_and(|error| error.contains("the config is invalid"))
    );
    assert_eq!(fixture.queue.payloads("run.failed").len(), 1);
    assert!(!fixture.queue.keys().contains(&"run.dead".to_owned()));
}

#[tokio::test]
async fn a_model_bundle_for_another_dataset_fails_the_run() {
    let fixture = fixture().await;
    let mut manifest = fixture.manifest.clone();
    manifest.data_version = Digest::sha256(b"another dataset");
    let services = services_with(
        &fixture,
        Arc::clone(&fixture.services.evidence),
        Arc::clone(&fixture.services.data),
        Arc::new(FixedModel { manifest }),
    );
    assert_eq!(
        process(fixture.run_id.clone(), false, &services)
            .await
            .expect("process"),
        ProcessOutcome::Failed
    );
    let record = fixture
        .runs
        .get_any(fixture.run_id.clone())
        .await
        .expect("get")
        .expect("record");
    assert_eq!(record.status, RunState::Failed);
    assert!(record.artifacts.is_empty());
    assert!(
        record
            .error
            .as_deref()
            .is_some_and(|error| error.contains("another dataset"))
    );
}

#[tokio::test]
async fn a_missing_model_bundle_fails_the_run() {
    let fixture = fixture().await;
    let services = services_with(
        &fixture,
        Arc::clone(&fixture.services.evidence),
        Arc::clone(&fixture.services.data),
        Arc::new(MemoryModel::new()),
    );
    assert_eq!(
        process(fixture.run_id.clone(), false, &services)
            .await
            .expect("process"),
        ProcessOutcome::Failed
    );
    let record = fixture
        .runs
        .get_any(fixture.run_id.clone())
        .await
        .expect("get")
        .expect("record");
    assert_eq!(record.status, RunState::Failed);
    assert!(record.artifacts.is_empty());
    assert!(
        record
            .error
            .as_deref()
            .is_some_and(|error| error.contains("the model bundle is unknown"))
    );
}

#[tokio::test]
async fn a_staged_backtest_failure_retries_then_dead_letters() {
    let fixture = fixture().await;
    let profile = ProfileId::parse("default").expect("profile");
    let provenance = fixture
        .objects
        .get("model/bundle/provenance.json")
        .await
        .expect("provenance");
    let events = fixture
        .objects
        .get("model/bundle/events.csv")
        .await
        .expect("events");
    let readings = b"timeframe_secs,bar\n".to_vec();
    let mut artifacts = Vec::new();
    for (name, bytes) in [
        ("provenance.json", provenance),
        ("events.csv", events),
        ("readings.csv", readings),
    ] {
        fixture
            .objects
            .put(&format!("model/bad/{name}"), bytes.clone())
            .await
            .expect("bad artifact");
        artifacts.push(ArtifactRef {
            name: name.to_owned(),
            bytes: bytes.len() as u64,
            sha256: Digest::sha256(&bytes),
        });
    }
    artifacts.sort_by(|left, right| left.name.cmp(&right.name));
    let manifest = ModelBundleManifest {
        schema: quantrule_service_api::MODEL_BUNDLE_SCHEMA.to_owned(),
        data_version: fixture.data_version.clone(),
        config_id: fixture.config_id.clone(),
        config_hash: fixture.config_hash.clone(),
        model_version: "M1.0".to_owned(),
        ladder: vec!["1m".to_owned()],
        manifest_key: "model/bad/manifest.json".to_owned(),
        artifacts,
    };
    let model = MemoryModel::new();
    model.add_bundle(&profile, manifest);
    let services = services_with(
        &fixture,
        Arc::clone(&fixture.services.evidence),
        Arc::clone(&fixture.services.data),
        Arc::new(model),
    );
    assert_eq!(
        process(fixture.run_id.clone(), false, &services)
            .await
            .expect("first"),
        ProcessOutcome::Retry {
            delay: Duration::from_secs(5)
        }
    );
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
    assert!(record.artifacts.is_empty());
    assert!(
        record
            .error
            .as_deref()
            .is_some_and(|error| error.contains("model bundle"))
    );
    assert!(fixture.queue.keys().contains(&"run.dead".to_owned()));
}

#[tokio::test]
async fn an_evidence_outage_holds_the_delivery() {
    let fixture = fixture().await;
    let evidence = Arc::new(EvidenceDown {
        inner: fixture.runs.clone(),
        fail_claim: true,
        fail_computed: false,
    });
    let services = services_with(
        &fixture,
        evidence,
        Arc::clone(&fixture.services.data),
        Arc::clone(&fixture.services.model),
    );
    assert!(
        process(fixture.run_id.clone(), false, &services)
            .await
            .is_err()
    );
    let record = fixture
        .runs
        .get_any(fixture.run_id.clone())
        .await
        .expect("get")
        .expect("record");
    assert_eq!(record.status, RunState::Queued);
    assert!(fixture.queue.keys().is_empty());
}

#[tokio::test]
async fn an_evidence_finalize_outage_leaves_the_run_running() {
    let fixture = fixture().await;
    let evidence = Arc::new(EvidenceDown {
        inner: fixture.runs.clone(),
        fail_claim: false,
        fail_computed: true,
    });
    let services = services_with(
        &fixture,
        evidence,
        Arc::clone(&fixture.services.data),
        Arc::clone(&fixture.services.model),
    );
    assert!(
        process(fixture.run_id.clone(), false, &services)
            .await
            .is_err()
    );
    let record = fixture
        .runs
        .get_any(fixture.run_id.clone())
        .await
        .expect("get")
        .expect("record");
    assert_eq!(record.status, RunState::Running);
    assert!(record.digest.is_none());
    assert!(record.artifacts.is_empty());
    assert!(!fixture.queue.keys().contains(&"run.computed".to_owned()));
}
