//! The strategy stage executor over a real model bundle and config fixture.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use quantrule_api::{
    Digest, ProfileId, StageArtifact, StageInput, StageKind, StageProvenance, StageState,
};
use quantrule_backtest_service::stage::{StageExecutor, StrategyStageExecutor};
use quantrule_runtime::adapters::ObjectStore;
use quantrule_runtime::client::{MemoryStageClient, StageClient};
use quantrule_service_api::{ConfigBundle, NewStageRun, StageManifest, StageRecord};

use crate::support::{Fixture, fixture};

fn profile() -> ProfileId {
    ProfileId::parse("default").expect("profile")
}

fn digest(seed: &[u8]) -> Digest {
    Digest::sha256(seed)
}

async fn upstream_stage(fixture: &Fixture) -> (MemoryStageClient, StageRecord) {
    let evidence = MemoryStageClient::default();
    let provenance = StageProvenance {
        data_version: fixture.data_version.clone(),
        config_hash: fixture.config_hash.clone(),
        model_version: "M1.0".to_owned(),
        image_digest: digest(b"image"),
    };
    let run = NewStageRun::new(
        StageKind::Model,
        fixture.config_id.clone(),
        StageInput::DataVersion {
            data_version: fixture.data_version.clone(),
        },
        provenance.clone(),
        "default".to_owned(),
        "2026-11-17T00:00:00Z".to_owned(),
    );
    let stage_run_id = run.stage_run_id.clone();
    let record = evidence.submit(&profile(), run).await.expect("submit");
    let record = match evidence
        .claim_stage(record.stage_run_id.to_string(), false)
        .await
        .expect("claim")
    {
        quantrule_service_api::StageClaimOutcome::Claimed(record) => record,
        other => panic!("the model stage was not claimed: {other:?}"),
    };
    assert_eq!(record.state, StageState::Running);
    let artifact = StageArtifact::from_files(fixture.manifest.artifacts.clone())
        .expect("the upstream artifact");
    let prefix = fixture
        .manifest
        .manifest_key
        .strip_suffix("/manifest.json")
        .expect("the model bundle prefix");
    for file in &fixture.manifest.artifacts {
        let bytes = fixture
            .objects
            .get(&format!("{prefix}/{}", file.name))
            .await
            .expect("the model artifact");
        fixture
            .objects
            .put(&StageManifest::blob_key(&file.sha256), bytes)
            .await
            .expect("the stage blob");
    }
    let manifest = StageManifest::new(
        stage_run_id.clone(),
        StageKind::Model,
        fixture.config_id.clone(),
        &provenance,
        StageInput::DataVersion {
            data_version: fixture.data_version.clone(),
        },
        artifact.files.clone(),
    )
    .expect("the upstream stage manifest");
    fixture
        .objects
        .put(
            &StageManifest::key(&stage_run_id),
            manifest.to_bytes().expect("the manifest bytes"),
        )
        .await
        .expect("the stage manifest");
    let record = evidence
        .stage_computed(stage_run_id.to_string(), artifact)
        .await
        .expect("complete");
    (evidence, record)
}

fn strategy_record(upstream: &StageRecord, fixture: &Fixture) -> StageRecord {
    let run = NewStageRun::new(
        StageKind::Strategy,
        fixture.config_id.clone(),
        StageInput::ModelArtifact {
            model_artifact_id: upstream
                .artifact
                .as_ref()
                .expect("the upstream artifact")
                .artifact_id
                .clone(),
        },
        StageProvenance {
            data_version: fixture.data_version.clone(),
            config_hash: fixture.config_hash.clone(),
            model_version: "M1.0".to_owned(),
            image_digest: digest(b"image"),
        },
        "default".to_owned(),
        "2026-11-17T00:00:01Z".to_owned(),
    );
    let mut record = StageRecord::queued(&run);
    record.state = StageState::Running;
    record.attempts = 1;
    record
}

#[tokio::test]
async fn the_strategy_stage_publishes_only_decisions_and_provenance() {
    let fixture = fixture().await;
    let (evidence, upstream) = upstream_stage(&fixture).await;
    let record = strategy_record(&upstream, &fixture);
    assert_eq!(
        record.provenance.model_version, upstream.provenance.model_version,
        "the success case must match the upstream model version"
    );
    let executor = StrategyStageExecutor::new(
        Arc::new(evidence),
        Arc::clone(&fixture.services.data),
        Arc::new(fixture.objects.clone()),
    );
    let output = executor.execute(&record).await.expect("strategy stage");
    let names: Vec<&str> = output.files.iter().map(|file| file.name.as_str()).collect();
    assert_eq!(
        names,
        vec!["decisions.ndjson", "summary.txt", "provenance.json"]
    );
    for file in &output.files {
        let text = String::from_utf8_lossy(&file.bytes).to_lowercase();
        for forbidden in ["fills", "cash", "p&l", "pnl", "equity", "drawdown"] {
            assert!(
                !text.contains(forbidden),
                "{} contains {forbidden}",
                file.name
            );
        }
    }
    let first = output;
    let second = executor
        .execute(&record)
        .await
        .expect("second strategy stage");
    assert_eq!(first, second, "the strategy stage is deterministic");
}

#[tokio::test]
async fn a_missing_upstream_blob_fails_the_strategy_stage() {
    let fixture = fixture().await;
    let (evidence, upstream) = upstream_stage(&fixture).await;
    let record = strategy_record(&upstream, &fixture);
    let artifact = upstream.artifact.as_ref().expect("the upstream artifact");
    let missing = artifact.files.first().expect("the first upstream artifact");
    fixture
        .objects
        .delete(&StageManifest::blob_key(&missing.sha256))
        .await
        .expect("delete the blob");
    let executor = StrategyStageExecutor::new(
        Arc::new(evidence),
        Arc::clone(&fixture.services.data),
        Arc::new(fixture.objects.clone()),
    );
    let error = executor.execute(&record).await.expect_err("missing blob");
    assert!(error.to_string().contains("missing"), "{error}");
}

#[tokio::test]
async fn a_corrupt_upstream_blob_fails_the_strategy_stage() {
    let fixture = fixture().await;
    let (evidence, upstream) = upstream_stage(&fixture).await;
    let record = strategy_record(&upstream, &fixture);
    let artifact = upstream.artifact.as_ref().expect("the upstream artifact");
    let first = artifact.files.first().expect("the first upstream artifact");
    fixture
        .objects
        .put(&StageManifest::blob_key(&first.sha256), b"corrupt".to_vec())
        .await
        .expect("overwrite the blob");
    let executor = StrategyStageExecutor::new(
        Arc::new(evidence),
        Arc::clone(&fixture.services.data),
        Arc::new(fixture.objects.clone()),
    );
    let error = executor.execute(&record).await.expect_err("corrupt blob");
    assert!(
        error.to_string().contains("does not match its manifest"),
        "{error}"
    );
}

#[tokio::test]
async fn a_model_version_mismatch_fails_the_strategy_stage() {
    let fixture = fixture().await;
    let (evidence, upstream) = upstream_stage(&fixture).await;
    let mut record = strategy_record(&upstream, &fixture);
    record.provenance.model_version = "M9.9".to_owned();
    let executor = StrategyStageExecutor::new(
        Arc::new(evidence),
        Arc::clone(&fixture.services.data),
        Arc::new(fixture.objects.clone()),
    );
    let error = executor.execute(&record).await.expect_err("model mismatch");
    assert!(error.to_string().contains("model version"), "{error}");
}

fn nested_strategy_bundle() -> ConfigBundle {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/config/valid-v4");
    let mut files = BTreeMap::new();
    for name in [
        "run.json",
        "data.json",
        "model.json",
        "execution.json",
        "portfolio.json",
        "strategy.json",
    ] {
        files.insert(
            name.to_owned(),
            std::fs::read_to_string(source.join(name)).expect("the config file"),
        );
    }
    let mut run: serde_json::Value =
        serde_json::from_str(files.get("run.json").expect("run.json")).expect("run JSON");
    run["strategy"] = serde_json::json!("rules/entry.json");
    files.insert("run.json".to_owned(), run.to_string());
    let strategy = files.remove("strategy.json").expect("strategy file");
    files.insert("rules/entry.json".to_owned(), strategy);
    ConfigBundle {
        config_id: "cfg_nested".to_owned(),
        files,
        valid: true,
        message: None,
        config_hash: Some(Digest::sha256(b"nested-strategy")),
    }
}

#[tokio::test]
async fn the_executor_uses_the_strategy_file_named_by_the_run_config() {
    let fixture = fixture().await;
    let (evidence, upstream) = upstream_stage(&fixture).await;
    let config = nested_strategy_bundle();
    let config_hash = config.config_hash.clone().expect("the nested config hash");
    fixture.data.add_config(&profile(), config);
    let mut record = strategy_record(&upstream, &fixture);
    record.config_id = "cfg_nested".to_owned();
    record.provenance.config_hash = config_hash;
    let executor = StrategyStageExecutor::new(
        Arc::new(evidence),
        Arc::clone(&fixture.services.data),
        Arc::new(fixture.objects.clone()),
    );
    let output = executor
        .execute(&record)
        .await
        .expect("nested strategy stage");
    let names: Vec<&str> = output.files.iter().map(|file| file.name.as_str()).collect();
    assert_eq!(
        names,
        vec!["decisions.ndjson", "summary.txt", "provenance.json"]
    );
}

#[tokio::test]
async fn a_strategy_stage_with_another_kind_is_refused() {
    let fixture = fixture().await;
    let (evidence, upstream) = upstream_stage(&fixture).await;
    let mut record = strategy_record(&upstream, &fixture);
    record.kind = StageKind::Model;
    let executor = StrategyStageExecutor::new(
        Arc::new(evidence),
        Arc::clone(&fixture.services.data),
        Arc::new(fixture.objects.clone()),
    );
    let error = executor.execute(&record).await.expect_err("wrong kind");
    assert!(error.to_string().contains("strategy executor"), "{error}");
}

/// A private test directory removed on drop.
struct Scratch(std::path::PathBuf);

impl Scratch {
    fn new(label: &str) -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        Self(std::env::temp_dir().join(format!(
            "quantrule-divergence-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        )))
    }

    fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

const READINGS: &str = "\
timeframe_secs,bar,timestamp,open,high,low,close,volume,ema_stack,rsi_zone,macd_state,macd_histogram_state,adx_regime,adx_slope,di_state,rvol_regime,vol_regime,structure_state,progression_stage_code,progression_leg_direction,atr,atr_ratio,rvol_ratio,context_code
60,0,1704067200,100.0,101.0,99.0,100.5,10.0,,,,,,,,,,,,,,1.0,,
60,1,1704067260,100.5,101.5,99.5,101.0,10.0,,,,,,,,,,,,,,1.0,,
60,2,1704067380,101.0,102.0,100.0,101.5,10.0,,,,,,,,,,,,,,1.0,,
60,3,1704067440,101.5,102.5,100.5,102.0,10.0,,,,,,,,,,,,,,1.0,,
";

const EVENTS: &str = "\
timeframe_secs,event_code,event_type_code,event_subtype_code,source_code,occurred_bar_index,available_bar_index,direction_code,reference_code,primary_value,reference_value
60,1,3,7,5,1,1,-1,,,
60,2,3,7,5,3,3,-1,,,
";

const PROVENANCE: &str = "{\"schema\":\"quantrule.provenance/v1\",\"report\":\"model\",\"data_version\":\"fixture\",\"config_hash\":\"h1\",\"requested_period\":{\"from\":null,\"to\":null}}\n";

fn write_model(root: &std::path::Path) {
    std::fs::create_dir_all(root).expect("the model directory");
    std::fs::write(root.join("readings.csv"), READINGS).expect("readings");
    std::fs::write(root.join("events.csv"), EVENTS).expect("events");
    std::fs::write(root.join("provenance.json"), PROVENANCE).expect("provenance");
}

const CANDLES: &str = "\
Date,Open,High,Low,Close,Volume
2024-01-01 00:00:00+00:00,100.0,101.0,99.0,100.5,10
2024-01-01 00:01:00+00:00,100.5,101.5,99.5,101.0,10
2024-01-01 00:03:00+00:00,101.0,102.0,100.0,101.5,10
2024-01-01 00:04:00+00:00,101.5,102.5,100.5,102.0,10
";

fn write_config(root: &std::path::Path) -> (std::path::PathBuf, std::path::PathBuf) {
    let source =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/config/valid-v4");
    std::fs::create_dir_all(root).expect("the config directory");
    for name in [
        "run.json",
        "data.json",
        "model.json",
        "execution.json",
        "portfolio.json",
        "strategy.json",
    ] {
        std::fs::copy(source.join(name), root.join(name)).expect("the config file");
    }
    let data = std::fs::read_to_string(root.join("data.json")).expect("the data config");
    let data = data.replace(
        "../../../data/sources/BTCUSD_M1_2024-01-01_2024-12-31.csv",
        "../candles.csv",
    );
    std::fs::write(root.join("data.json"), data).expect("the data config");
    std::fs::write(root.join("../candles.csv"), CANDLES).expect("the candle source");
    (root.join("run.json"), root.join("strategy.json"))
}

fn decisions(root: &std::path::Path) -> Vec<serde_json::Value> {
    let text = std::fs::read_to_string(root).expect("the decisions artifact");
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("one decision record"))
        .collect()
}

fn decision_at(records: &[serde_json::Value], bar: u64) -> &serde_json::Value {
    records
        .iter()
        .find(|record| record["bar_index"].as_u64() == Some(bar))
        .expect("the decision for the bar")
}

#[test]
fn a_rejected_coupled_entry_changes_later_decisions_but_the_stage_cannot_see_it() {
    use quantrule_app::stages::backtest_from_model::{
        BacktestFromModelRequest, run as run_coupled,
    };
    use quantrule_app::stages::strategy::{StrategyRunRequest, run as run_stage};

    let scratch = Scratch::new("strategy-stage");
    let model = scratch.path().join("model");
    write_model(&model);
    let (run_config, strategy_config) = write_config(&scratch.path().join("config"));

    let stage_out = scratch.path().join("stage");
    let stage = run_stage(&StrategyRunRequest {
        model: &model,
        strategy: &strategy_config,
        out: Some(&stage_out),
    })
    .expect("the standalone strategy stage");
    assert_eq!(stage.root, stage_out.join("strategy"));
    let standalone = decisions(&stage_out.join("strategy/decisions.ndjson"));

    let coupled_out = scratch.path().join("coupled");
    let coupled = run_coupled(&BacktestFromModelRequest {
        config: &run_config,
        strategy: &strategy_config,
        model: &model,
        out: Some(&coupled_out),
    })
    .expect("the coupled backtest");
    assert_eq!(coupled.root, coupled_out.join("backtest"));
    let coupled_records = decisions(&coupled_out.join("backtest/strategy/decisions.ndjson"));

    let rejections = std::fs::read_to_string(coupled_out.join("backtest/execution/rejections.csv"))
        .expect("the execution rejections");
    assert!(
        rejections.contains("non_contiguous_bar"),
        "the coupled entry must be rejected: {rejections}"
    );

    let stage_entry = decision_at(&standalone, 1);
    assert_eq!(stage_entry["decision"], "enter");
    assert_eq!(stage_entry["position_before"], "flat");
    assert_eq!(stage_entry["position_after"], "long");

    let stage_later = decision_at(&standalone, 3);
    assert_eq!(
        stage_later["position_before"], "long",
        "the stage holds its own simulated position"
    );
    assert_eq!(stage_later["decision"], "hold");

    let coupled_later = decision_at(&coupled_records, 3);
    assert_eq!(
        coupled_later["position_before"], "flat",
        "execution rejected the entry, so the coupled strategy is still flat"
    );
    assert_eq!(coupled_later["decision"], "enter");
    assert_eq!(coupled_later["position_after"], "long");

    assert_ne!(
        standalone, coupled_records,
        "a rejected coupled entry must change the later decision sequence"
    );

    for record in standalone.iter().chain(coupled_records.iter()) {
        let text = record.to_string().to_lowercase();
        for forbidden in ["fill", "rejection", "cash", "pnl", "equity", "drawdown"] {
            assert!(
                !text.contains(forbidden),
                "a decision record must not carry execution feedback: {text}"
            );
        }
    }
    let stage_files: Vec<String> = std::fs::read_dir(stage_out.join("strategy"))
        .expect("the stage bundle")
        .map(|entry| {
            entry
                .expect("the stage artifact")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    let mut stage_files = stage_files;
    stage_files.sort();
    assert_eq!(
        stage_files,
        vec!["decisions.ndjson", "provenance.json", "summary.txt"],
        "the strategy stage publishes only the decision bundle"
    );
}
