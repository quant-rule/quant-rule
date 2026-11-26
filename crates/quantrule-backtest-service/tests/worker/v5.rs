//! A v5 run through the real worker resolver and fold.
//!
//! The fixture reuses the v4 model bundle and replaces only the run config with
//! a v5 document whose three pack references are registered in the fake data
//! service. The worker must resolve them once, fold, and publish the same
//! content-addressed artifact set a failed run never publishes.

use super::support::{
    ConfigBundle, Digest, EvidenceClient, Fixture, NewRun, ObjectStore, ProcessOutcome, ProfileId,
    RunManifest, RunRecord, derive, fixture, process,
};
use quantrule_api::{ReferenceKind, RunArtifactReference, RunState};
use quantrule_app::pack::canonical_json;
use serde_json::{Value, json};

fn strategy_pack() -> Value {
    json!({
        "schema": "quantrule.pack/v1",
        "kind": "strategy",
        "version": "1.0.0",
        "name": "compression-fade-5m",
        "owner": "operator",
        "payload": {
            "mode": "fold",
            "engine": "event_composite",
            "execution_timeframe": "5m",
            "context_timeframes": ["15m"],
            "universe": ["BTCUSD"],
            "event_window_bars": 1,
            "time_stop_bars": 0,
            "giveback_atr": null,
            "giveback_arm_atr": 0.0,
            "require_contiguous": true,
            "risk": {"stop_atr_multiple": 2.0, "reward_to_risk": 1.5},
            "entry": [
                {"kind": "event", "timeframe": "5m", "event_type": 3, "subtype": 7,
                 "source": 5, "direction": 0, "contrarian": true}
            ],
            "exit": []
        }
    })
}

fn execution_pack() -> Value {
    json!({
        "schema": "quantrule.pack/v1",
        "kind": "execution",
        "version": "1.0.0",
        "name": "btcusd-conservative-fills",
        "owner": "operator",
        "payload": {
            "fill_timeframe": "5m",
            "order_type": "market_next_open",
            "tie_break": "stop_first",
            "fee_rate": 0.0,
            "slippage_bps": 2.0,
            "commission": {"rate": 0.001, "base": "equity", "per": "operation"},
            "funding": {"rate": 0.0001, "interval_hours": 8},
            "zero_volume": "reject",
            "end_of_data": "leave_open"
        }
    })
}

fn portfolio_pack() -> Value {
    json!({
        "schema": "quantrule.pack/v1",
        "kind": "portfolio",
        "version": "1.0.0",
        "name": "single-position-conservative",
        "owner": "operator",
        "payload": {
            "sizing": {"mode": "equity_fraction", "fraction": 0.05, "quantity_step": 0.0001},
            "risk": {"per_trade_risk": 0.01, "max_exposure": 1.0, "max_open_positions": 1},
            "drawdown": {"limit": 0.15, "action": "halt"},
            "rebalance": "none",
            "constraints": []
        }
    })
}

fn reference(kind: ReferenceKind, bytes: &[u8]) -> RunArtifactReference {
    let id = Digest::sha256(bytes);
    RunArtifactReference {
        kind,
        id: id.clone(),
        hash: id,
        pack: None,
    }
}

/// Build a v5 run on the v4 model bundle.
///
/// `register_strategy` is false for the fail-closed case: the run is submitted
/// with a reference the data service cannot resolve.
async fn v5_fixture(register_strategy: bool) -> (Fixture, String) {
    let mut f: Fixture = fixture().await;
    let profile = ProfileId::parse("default").expect("profile");
    let config_id = "cfg_v5_e2e";
    let config_hash = Digest::sha256(b"v5-e2e");

    let strategy = canonical_json(&strategy_pack()).into_bytes();
    let execution = canonical_json(&execution_pack()).into_bytes();
    let portfolio = canonical_json(&portfolio_pack()).into_bytes();
    let references = vec![
        reference(ReferenceKind::StrategyPack, &strategy),
        reference(ReferenceKind::ExecutionModel, &execution),
        reference(ReferenceKind::PortfolioPolicy, &portfolio),
    ];
    if register_strategy {
        f.data
            .add_pack_bytes(&profile, &references[0].hash, strategy);
    }
    f.data
        .add_pack_bytes(&profile, &references[1].hash, execution);
    f.data
        .add_pack_bytes(&profile, &references[2].hash, portfolio);

    let run = json!({
        "schema": "quantrule.run/v5",
        "name": "v5-e2e",
        "model_id": "M1.0",
        "data": "data.json",
        "model": "model.json",
        "strategy_pack": {"id": references[0].id, "hash": references[0].hash},
        "portfolio_policy": {"id": references[2].id, "hash": references[2].hash},
        "execution_model": {"id": references[1].id, "hash": references[1].hash},
        "account": {"initial_cash": 10000.0, "currency": "USD"},
        "output_root": "out"
    });
    let files = std::collections::BTreeMap::from([
        (
            "run.json".to_owned(),
            serde_json::to_string_pretty(&run).expect("run"),
        ),
        (
            "data.json".to_owned(),
            f.config.files.get("data.json").expect("data").clone(),
        ),
        (
            "model.json".to_owned(),
            f.config.files.get("model.json").expect("model").clone(),
        ),
    ]);
    let bundle = ConfigBundle {
        config_id: config_id.to_owned(),
        references: references.clone(),
        files,
        valid: true,
        message: None,
        config_hash: Some(config_hash.clone()),
    };
    f.data.add_config(&profile, bundle.clone());

    let mut manifest = f.manifest.clone();
    manifest.config_id = config_id.to_owned();
    manifest.config_hash = config_hash.clone();
    f.model.add_bundle(&profile, manifest.clone());

    let image = Digest::sha256(b"image");
    let run_id = derive(&f.data_version, &config_hash, "M1.0", &image);
    f.runs
        .submit(
            &profile,
            NewRun {
                run_id: run_id.clone(),
                lineage: references,
                config_id: config_id.to_owned(),
                data_version: f.data_version.clone(),
                config_hash: config_hash.clone(),
                model_version: "M1.0".to_owned(),
                image_digest: image,
                model_artifact_id: None,
                owner: "default".to_owned(),
                submitted_at: "2026-11-26T00:00:00Z".to_owned(),
            },
        )
        .await
        .expect("submit");

    f.config_id = config_id.to_owned();
    f.config = bundle;
    f.config_hash = config_hash;
    f.manifest = manifest;
    f.run_id = run_id.to_string();
    (f, run_id.to_string())
}

async fn record(f: &Fixture, run_id: &str) -> RunRecord {
    f.runs
        .get_run(run_id.to_owned())
        .await
        .expect("read")
        .expect("record")
}

#[tokio::test]
async fn a_v5_run_resolves_packs_fetches_artifacts_and_records_a_digest() {
    let (f, run_id) = v5_fixture(true).await;
    let outcome = process(run_id.clone(), None, &f.services)
        .await
        .expect("process");
    let record = record(&f, &run_id).await;
    assert_eq!(outcome, ProcessOutcome::Computed, "{record:?}");
    // The worker stages the digest; the evidence service finalises the record
    // from the queue message. This fake records the staged state only.
    assert_eq!(record.status, RunState::Running, "{record:?}");
    let digest = record.digest.clone().expect("the staged digest");
    assert!(!record.artifacts.is_empty());

    let manifest_key = RunManifest::key(&run_id);
    let manifest = RunManifest::from_bytes(
        &f.objects
            .get(&manifest_key)
            .await
            .expect("the run manifest"),
    )
    .expect("the manifest decodes");
    assert_eq!(manifest.digest, digest);
    for artifact in &record.artifacts {
        let key = RunManifest::blob_key(&artifact.sha256);
        let bytes = f.objects.get(&key).await.expect("the artifact bytes");
        assert_eq!(Digest::sha256(&bytes), artifact.sha256);
    }
}

#[tokio::test]
async fn a_missing_v5_pack_publishes_nothing() {
    let (f, run_id) = v5_fixture(false).await;
    let outcome = process(run_id.clone(), None, &f.services)
        .await
        .expect("process");
    let record = record(&f, &run_id).await;
    assert_eq!(outcome, ProcessOutcome::Failed, "{record:?}");
    assert_eq!(record.status, RunState::Failed, "{record:?}");
    assert!(record.digest.is_none());
    assert!(record.artifacts.is_empty());
    let manifest_key = RunManifest::key(&run_id);
    assert!(
        f.objects.get(&manifest_key).await.is_err(),
        "a failed v5 run published a manifest"
    );
}

#[tokio::test]
async fn one_worker_and_two_workers_produce_the_same_v5_digest() {
    let (sequential, sequential_id) = v5_fixture(true).await;
    assert_eq!(
        process(sequential_id.clone(), None, &sequential.services)
            .await
            .expect("sequential"),
        ProcessOutcome::Computed
    );
    let expected = record(&sequential, &sequential_id)
        .await
        .digest
        .expect("the sequential digest");

    let (concurrent, concurrent_id) = v5_fixture(true).await;
    let (left, right) = tokio::join!(
        process(concurrent_id.clone(), None, &concurrent.services),
        process(concurrent_id.clone(), None, &concurrent.services),
    );
    let outcomes = [left.expect("left"), right.expect("right")];
    assert!(outcomes.contains(&ProcessOutcome::Computed));
    assert!(
        outcomes
            .iter()
            .all(|outcome| matches!(outcome, ProcessOutcome::Computed | ProcessOutcome::Skipped))
    );
    let actual = record(&concurrent, &concurrent_id)
        .await
        .digest
        .expect("the concurrent digest");
    assert_eq!(actual, expected);
    assert_eq!(concurrent.model.calls(), 1);
}
