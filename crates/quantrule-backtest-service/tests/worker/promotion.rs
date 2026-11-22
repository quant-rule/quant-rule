//! The promoted run reuses a frozen model stage instead of the model service.

use std::sync::Arc;

use quantrule_api::{Digest, ProfileId, StageArtifact, StageInput, StageKind, StageProvenance};
use quantrule_backtest_service::backtest::{ProcessOutcome, WorkerServices, process};
use quantrule_runtime::adapters::ObjectStore;
use quantrule_runtime::client::StageClient;
use quantrule_service_api::{
    ConfigBundle, NewRun, NewStageRun, RunManifest, StageClaimOutcome, StageManifest,
    derive_run_id_with_model,
};

use crate::support::{Fixture, fixture};

fn profile() -> ProfileId {
    ProfileId::parse("default").expect("profile")
}

fn digest(seed: &[u8]) -> Digest {
    Digest::sha256(seed)
}

async fn publish_model_stage(fixture: &Fixture) -> Digest {
    publish_model_stage_for(
        fixture,
        &fixture.config_id,
        &fixture.config_hash,
        &fixture.manifest,
    )
    .await
}

async fn publish_model_stage_for(
    fixture: &Fixture,
    config_id: &str,
    config_hash: &Digest,
    manifest: &quantrule_service_api::ModelBundleManifest,
) -> Digest {
    let provenance = StageProvenance {
        data_version: fixture.data_version.clone(),
        config_hash: config_hash.clone(),
        model_version: "M1.0".to_owned(),
        image_digest: digest(b"image"),
    };
    let run = NewStageRun::new(
        StageKind::Model,
        config_id.to_owned(),
        StageInput::DataVersion {
            data_version: fixture.data_version.clone(),
        },
        provenance.clone(),
        "default".to_owned(),
        "2026-11-17T03:00:00Z".to_owned(),
    );
    let stage_run_id = run.stage_run_id.clone();
    let record = fixture
        .stages
        .submit(&profile(), run)
        .await
        .expect("submit the model stage");
    let StageClaimOutcome::Claimed(_) = fixture
        .stages
        .claim_stage(record.stage_run_id.to_string(), false)
        .await
        .expect("claim the model stage")
    else {
        panic!("the model stage was not claimed");
    };
    for file in &manifest.artifacts {
        let bytes = fixture
            .objects
            .get(&format!("model/bundle/{}", file.name))
            .await
            .expect("the model bundle artifact");
        fixture
            .objects
            .put(&StageManifest::blob_key(&file.sha256), bytes)
            .await
            .expect("the stage blob");
    }
    let artifact =
        StageArtifact::from_files(manifest.artifacts.clone()).expect("the model stage artifact");
    let stage_manifest = StageManifest::new(
        stage_run_id.clone(),
        StageKind::Model,
        config_id.to_owned(),
        &provenance,
        StageInput::DataVersion {
            data_version: fixture.data_version.clone(),
        },
        artifact.files.clone(),
    )
    .expect("the model stage manifest");
    fixture
        .objects
        .put(
            &StageManifest::key(&stage_run_id),
            stage_manifest.to_bytes().expect("the manifest bytes"),
        )
        .await
        .expect("the stage manifest");
    fixture
        .stages
        .stage_computed(stage_run_id.to_string(), artifact.clone())
        .await
        .expect("complete the model stage");
    artifact.artifact_id
}

fn changed_strategy_config(fixture: &Fixture) -> ConfigBundle {
    let mut config = fixture.config.clone();
    config.config_id = "cfg_changed_strategy".to_owned();
    config.config_hash = Some(Digest::sha256(b"changed-strategy-config"));
    let mut strategy: serde_json::Value =
        serde_json::from_str(config.files.get("strategy.json").expect("strategy"))
            .expect("strategy JSON");
    strategy["id"] = serde_json::json!("SR1-FADE-CHANGED");
    strategy["context_timeframes"] = serde_json::json!(["5m"]);
    strategy["entry"] = serde_json::json!([
        {
            "kind": "state",
            "timeframe": "1m",
            "field": "ema_stack",
            "value": 1
        },
        {
            "kind": "event",
            "timeframe": "5m",
            "event_type": 3,
            "subtype": 10,
            "source": 38,
            "direction": 0,
            "contrarian": false
        }
    ]);
    strategy["exit"] = serde_json::json!([]);
    config
        .files
        .insert("strategy.json".to_owned(), strategy.to_string());
    config
}

fn changed_model_config(fixture: &Fixture) -> ConfigBundle {
    let mut config = changed_strategy_config(fixture);
    config.config_id = "cfg_changed_model".to_owned();
    config.config_hash = Some(Digest::sha256(b"changed-model-config"));
    let model = config.files.get_mut("model.json").expect("model");
    model.push('\n');
    config
}

fn promoted_run(fixture: &Fixture, artifact_id: &Digest, image_seed: &[u8]) -> NewRun {
    let image = digest(image_seed);
    NewRun {
        run_id: derive_run_id_with_model(
            &fixture.data_version,
            &fixture.config_hash,
            "M1.0",
            &image,
            Some(artifact_id),
        ),
        config_id: fixture.config_id.clone(),
        data_version: fixture.data_version.clone(),
        config_hash: fixture.config_hash.clone(),
        model_version: "M1.0".to_owned(),
        image_digest: image,
        model_artifact_id: Some(artifact_id.clone()),
        owner: "default".to_owned(),
        submitted_at: "2026-11-17T03:00:01Z".to_owned(),
    }
}

fn services(fixture: &Fixture) -> WorkerServices {
    WorkerServices {
        evidence: Arc::clone(&fixture.services.evidence),
        data: Arc::clone(&fixture.services.data),
        model: Arc::clone(&fixture.services.model),
        stages: Arc::clone(&fixture.services.stages),
        objects: Arc::clone(&fixture.services.objects),
        queue: Arc::clone(&fixture.services.queue),
        holder: "test-worker".to_owned(),
    }
}

#[tokio::test]
async fn a_promoted_run_reuses_the_model_stage_and_keeps_the_canonical_digest() {
    let fixture = fixture().await;
    let artifact_id = publish_model_stage(&fixture).await;
    let run = promoted_run(&fixture, &artifact_id, b"promoted-image");
    let submitted = fixture
        .runs
        .submit(&profile(), run.clone())
        .await
        .expect("submit");
    let run_id = submitted.run_id.to_string();
    assert_ne!(run_id, fixture.run_id);

    let outcome = process(run_id.clone(), None, &services(&fixture))
        .await
        .expect("process the promoted run");
    assert_eq!(outcome, ProcessOutcome::Computed);
    assert_eq!(
        fixture.model.calls(),
        0,
        "a promoted run must not call the model service"
    );

    let record = fixture
        .runs
        .get_any(run_id.clone())
        .await
        .expect("read the run")
        .expect("the promoted run");
    assert_eq!(record.model_artifact_id, Some(artifact_id.clone()));
    assert_eq!(record.digest, Some(fixture.baseline_digest.clone()));

    for expected in &fixture.manifest.artifacts {
        let artifact = record
            .artifacts
            .iter()
            .find(|artifact| artifact.name == format!("model/{}", expected.name))
            .expect("the materialized model artifact");
        assert_eq!(artifact.sha256, expected.sha256);
        assert_eq!(artifact.bytes, expected.bytes);
    }

    let again = fixture
        .runs
        .submit(&profile(), run)
        .await
        .expect("resubmit the promoted run");
    assert_eq!(again.run_id.to_string(), run_id);
    let rerun = process(run_id, None, &services(&fixture))
        .await
        .expect("the deterministic rerun");
    assert_eq!(rerun, ProcessOutcome::Skipped);
}

#[tokio::test]
async fn a_strategy_only_change_reuses_the_model_stage() {
    let fixture = fixture().await;
    let artifact_id = publish_model_stage(&fixture).await;
    let changed = changed_strategy_config(&fixture);
    fixture.data.add_config(&profile(), changed.clone());
    let dataset = fixture
        .objects
        .get(&format!("datasets/{}", fixture.data_version.as_hex()))
        .await
        .expect("the dataset bytes");
    let changed_hash = changed.config_hash.clone().expect("the changed hash");
    let (_, changed_digest) = crate::support::coupled_baseline(&changed, &dataset);

    let image = digest(b"strategy-only-image");
    let run = NewRun {
        run_id: derive_run_id_with_model(
            &fixture.data_version,
            &changed_hash,
            "M1.0",
            &image,
            Some(&artifact_id),
        ),
        config_id: changed.config_id.clone(),
        data_version: fixture.data_version.clone(),
        config_hash: changed_hash,
        model_version: "M1.0".to_owned(),
        image_digest: image,
        model_artifact_id: Some(artifact_id.clone()),
        owner: "default".to_owned(),
        submitted_at: "2026-11-17T03:00:03Z".to_owned(),
    };
    let run_id = fixture
        .runs
        .submit(&profile(), run)
        .await
        .expect("submit")
        .run_id
        .to_string();
    let outcome = process(run_id.clone(), None, &services(&fixture))
        .await
        .expect("process the strategy-only run");
    assert_eq!(outcome, ProcessOutcome::Computed);
    assert_eq!(
        fixture.model.calls(),
        0,
        "a strategy-only change must not call the model service"
    );

    let record = fixture
        .runs
        .get_any(run_id)
        .await
        .expect("read the run")
        .expect("the strategy-only run");
    assert_eq!(record.model_artifact_id, Some(artifact_id));
    assert_eq!(record.digest, Some(changed_digest));
    for expected in &fixture.manifest.artifacts {
        let artifact = record
            .artifacts
            .iter()
            .find(|artifact| artifact.name == format!("model/{}", expected.name))
            .expect("the reused model artifact");
        assert_eq!(artifact.sha256, expected.sha256);
        assert_eq!(artifact.bytes, expected.bytes);
    }
}

#[tokio::test]
async fn a_changed_model_config_is_refused_by_the_model_input_identity() {
    let fixture = fixture().await;
    let artifact_id = publish_model_stage(&fixture).await;
    let changed = changed_model_config(&fixture);
    fixture.data.add_config(&profile(), changed.clone());
    let changed_hash = changed.config_hash.clone().expect("the changed hash");
    let image = digest(b"changed-model-image");
    let run = NewRun {
        run_id: derive_run_id_with_model(
            &fixture.data_version,
            &changed_hash,
            "M1.0",
            &image,
            Some(&artifact_id),
        ),
        config_id: changed.config_id.clone(),
        data_version: fixture.data_version.clone(),
        config_hash: changed_hash,
        model_version: "M1.0".to_owned(),
        image_digest: image,
        model_artifact_id: Some(artifact_id),
        owner: "default".to_owned(),
        submitted_at: "2026-11-17T03:00:04Z".to_owned(),
    };
    let run_id = fixture
        .runs
        .submit(&profile(), run)
        .await
        .expect("submit")
        .run_id
        .to_string();
    let outcome = process(run_id.clone(), None, &services(&fixture))
        .await
        .expect("process the changed-model run");
    assert_eq!(outcome, ProcessOutcome::Failed);
    let record = fixture
        .runs
        .get_any(run_id)
        .await
        .expect("read the run")
        .expect("the changed-model run");
    assert!(
        record
            .error
            .as_deref()
            .is_some_and(|error| error.contains("model input")),
        "{:?}",
        record.error
    );
}

#[tokio::test]
async fn a_missing_or_corrupt_model_stage_fails_the_promoted_run() {
    let fixture = fixture().await;
    let artifact_id = publish_model_stage(&fixture).await;

    let missing = promoted_run(&fixture, &digest(b"missing-model"), b"missing-image");
    let missing_id = fixture
        .runs
        .submit(&profile(), missing)
        .await
        .expect("submit the missing model")
        .run_id
        .to_string();
    let outcome = process(missing_id.clone(), None, &services(&fixture))
        .await
        .expect("process the missing model");
    assert_eq!(outcome, ProcessOutcome::Failed);
    let record = fixture
        .runs
        .get_any(missing_id)
        .await
        .expect("read the failed run")
        .expect("the failed run");
    assert!(
        record
            .error
            .as_deref()
            .is_some_and(|error| error.contains("model")),
        "{:?}",
        record.error
    );

    let corrupt = promoted_run(&fixture, &artifact_id, b"corrupt-image");
    let corrupt_id = fixture
        .runs
        .submit(&profile(), corrupt)
        .await
        .expect("submit the corrupt model")
        .run_id
        .to_string();
    let file = fixture
        .manifest
        .artifacts
        .first()
        .expect("the model artifact");
    fixture
        .objects
        .put(&StageManifest::blob_key(&file.sha256), b"corrupt".to_vec())
        .await
        .expect("corrupt the stage blob");
    let outcome = process(corrupt_id.clone(), None, &services(&fixture))
        .await
        .expect("process the corrupt model");
    assert_eq!(outcome, ProcessOutcome::Failed);
    let record = fixture
        .runs
        .get_any(corrupt_id)
        .await
        .expect("read the corrupt run")
        .expect("the corrupt run");
    assert!(
        record
            .error
            .as_deref()
            .is_some_and(|error| error.contains("does not match")),
        "{:?}",
        record.error
    );
}

#[tokio::test]
async fn a_promoted_run_keeps_the_model_bytes_the_stage_published() {
    let fixture = fixture().await;
    let artifact_id = publish_model_stage(&fixture).await;
    let run = promoted_run(&fixture, &artifact_id, b"bytes-image");
    let run_id = fixture
        .runs
        .submit(&profile(), run)
        .await
        .expect("submit")
        .run_id
        .to_string();
    let outcome = process(run_id.clone(), None, &services(&fixture))
        .await
        .expect("process");
    assert_eq!(outcome, ProcessOutcome::Computed);
    let record = fixture
        .runs
        .get_any(run_id)
        .await
        .expect("read")
        .expect("run");
    for expected in &fixture.manifest.artifacts {
        let reference = record
            .artifacts
            .iter()
            .find(|artifact| artifact.name == format!("model/{}", expected.name))
            .expect("the materialized model artifact");
        let bytes = fixture
            .objects
            .get(&RunManifest::blob_key(&reference.sha256))
            .await
            .expect("the published bytes");
        assert_eq!(Digest::sha256(&bytes), expected.sha256);
        assert_eq!(bytes.len() as u64, expected.bytes);
    }
}

#[tokio::test]
async fn a_changed_strategy_is_recomputed_in_a_promoted_run() {
    let fixture = fixture().await;
    let changed = changed_strategy_config(&fixture);
    fixture.data.add_config(&profile(), changed.clone());
    let dataset = fixture
        .objects
        .get(&format!("datasets/{}", fixture.data_version.as_hex()))
        .await
        .expect("the dataset bytes");
    let changed_hash = changed.config_hash.clone().expect("the changed hash");
    let (_, changed_manifest) = crate::support::model_bundle(
        &changed,
        &dataset,
        &fixture.data_version,
        &profile(),
        &fixture.objects,
        &fixture.model,
    )
    .await;
    let artifact_id = publish_model_stage_for(
        &fixture,
        &changed.config_id,
        &changed_hash,
        &changed_manifest,
    )
    .await;
    let (_, changed_digest) = crate::support::coupled_baseline(&changed, &dataset);
    assert_ne!(changed_digest, fixture.baseline_digest);

    let image = digest(b"changed-strategy-image");
    let run = NewRun {
        run_id: derive_run_id_with_model(
            &fixture.data_version,
            &changed_hash,
            "M1.0",
            &image,
            Some(&artifact_id),
        ),
        config_id: changed.config_id.clone(),
        data_version: fixture.data_version.clone(),
        config_hash: changed_hash,
        model_version: "M1.0".to_owned(),
        image_digest: image,
        model_artifact_id: Some(artifact_id),
        owner: "default".to_owned(),
        submitted_at: "2026-11-17T03:00:02Z".to_owned(),
    };
    let run_id = fixture
        .runs
        .submit(&profile(), run)
        .await
        .expect("submit")
        .run_id
        .to_string();
    let outcome = process(run_id.clone(), None, &services(&fixture))
        .await
        .expect("process");
    assert_eq!(outcome, ProcessOutcome::Computed);
    let record = fixture
        .runs
        .get_any(run_id)
        .await
        .expect("read")
        .expect("run");
    assert_eq!(record.digest, Some(changed_digest));
    assert_ne!(record.digest, Some(fixture.baseline_digest));
}

#[tokio::test]
async fn a_missing_model_stage_blob_is_retried() {
    let fixture = fixture().await;
    let artifact_id = publish_model_stage(&fixture).await;
    let file = fixture
        .manifest
        .artifacts
        .first()
        .expect("the model artifact");
    fixture
        .objects
        .delete(&StageManifest::blob_key(&file.sha256))
        .await
        .expect("delete the stage blob");
    let run = promoted_run(&fixture, &artifact_id, b"missing-blob-image");
    let run_id = fixture
        .runs
        .submit(&profile(), run)
        .await
        .expect("submit")
        .run_id
        .to_string();
    let outcome = process(run_id, None, &services(&fixture))
        .await
        .expect("process");
    assert_eq!(
        outcome,
        ProcessOutcome::Retry {
            delay: std::time::Duration::from_secs(5),
            term: 1
        }
    );
}
