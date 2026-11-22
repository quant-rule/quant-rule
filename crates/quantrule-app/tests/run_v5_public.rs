//! The public run-v5 seam.
//!
//! This test uses only the exported `quantrule_app` API: the reference resolver,
//! the loader, and the application-service use case. It proves a caller that
//! holds the bytes can validate and materialize a v5 run without a registry.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use quantrule_api::Digest;
use quantrule_app::config::{RUN_SCHEMA_V5, RunConfig};
use quantrule_app::pack::references::{MODEL_ARTIFACT_SCHEMA, SIGNAL_ARTIFACT_SCHEMA};
use quantrule_app::pack::{
    ContentReference, MemoryReferences, ReferenceKind, StrategyMode, canonical_json,
};
use quantrule_app::service::AppService;
use quantrule_data::Timeframe;
use serde_json::{Value, json};

static NEXT: AtomicU64 = AtomicU64::new(1);

const DATA_JSON: &str = include_str!("fixtures/run-v5/data.json");
const MODEL_JSON: &str = include_str!("fixtures/run-v5/model.json");

struct Fixture {
    root: PathBuf,
    references: MemoryReferences,
    run: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "quantrule-run-v5-public-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).expect("the fixture directory is created");
        fs::write(root.join("data.json"), DATA_JSON).expect("the data fixture is written");
        fs::write(root.join("model.json"), MODEL_JSON).expect("the model fixture is written");

        let mut references = MemoryReferences::new();
        let strategy = references.insert(
            ReferenceKind::StrategyPack,
            canonical_json(&strategy_pack()).into_bytes(),
        );
        let execution = references.insert(
            ReferenceKind::ExecutionModel,
            canonical_json(&execution_pack()).into_bytes(),
        );
        let portfolio = references.insert(
            ReferenceKind::PortfolioPolicy,
            canonical_json(&portfolio_pack()).into_bytes(),
        );
        let model_manifest = model_manifest();
        let model = ContentReference {
            id: model_artifact_id(&model_manifest),
            hash: model_artifact_id(&model_manifest),
        };
        references.insert_at(
            ReferenceKind::ModelArtifact,
            model.clone(),
            serde_json::to_vec(&model_manifest).expect("the model manifest is JSON"),
        );
        let signal = references.insert(
            ReferenceKind::SignalArtifact,
            signal_artifact(&strategy.hash, &model.hash),
        );

        let run = root.join("run.json");
        let document = json!({
            "schema": RUN_SCHEMA_V5,
            "name": "v5-public",
            "model_id": "M1.0",
            "data": "data.json",
            "model": "model.json",
            "model_artifact": model.to_value(),
            "strategy_pack": strategy.to_value(),
            "signal_artifact": signal.to_value(),
            "portfolio_policy": portfolio.to_value(),
            "execution_model": execution.to_value(),
            "account": {"initial_cash": 10000.0, "currency": "USD"},
            "output_root": "out"
        });
        fs::write(
            &run,
            serde_json::to_vec_pretty(&document).expect("the run document is JSON"),
        )
        .expect("the run document is written");
        Self {
            root,
            references,
            run,
        }
    }

    fn load(&self) -> RunConfig {
        RunConfig::load_with(&self.run, &self.references).expect("the v5 run loads")
    }

    fn service(&self) -> AppService {
        AppService::open(&self.root).expect("the fixture is a workspace")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn strategy_pack() -> Value {
    json!({
        "schema": "quantrule.pack/v1",
        "kind": "strategy",
        "version": "1.0.0",
        "name": "compression-fade-5m",
        "owner": "operator",
        "payload": {
            "mode": "signal",
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

fn model_manifest() -> Value {
    json!({
        "schema": MODEL_ARTIFACT_SCHEMA,
        "data_version": "btcusd-2024",
        "config_id": "cfg_test",
        "config_hash": Digest::sha256(b"config"),
        "model_version": "M1.0",
        "ladder": ["1m", "5m", "15m", "1h"],
        "manifest_key": "models/bundle/manifest.json",
        "artifacts": [
            {"name": "model/result.json", "bytes": 2, "sha256": Digest::sha256(b"a")}
        ]
    })
}

fn model_artifact_id(manifest: &Value) -> Digest {
    let artifacts = manifest["artifacts"]
        .as_array()
        .expect("the model manifest has an artifact list");
    let entries: Vec<(String, Digest)> = artifacts
        .iter()
        .map(|artifact| {
            (
                artifact["name"]
                    .as_str()
                    .expect("the artifact has a name")
                    .to_owned(),
                serde_json::from_value(artifact["sha256"].clone())
                    .expect("the artifact has a digest"),
            )
        })
        .collect();
    Digest::canonical(&entries)
}

fn signal_artifact(strategy: &Digest, model: &Digest) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "schema": SIGNAL_ARTIFACT_SCHEMA,
        "strategy_pack_id": strategy,
        "model_artifact_id": model,
        "data_version": "btcusd-2024",
        "timeframe": "5m",
        "period": {"from": "2024-01-01", "to": "2024-12-31"},
        "rows": []
    }))
    .expect("the signal artifact is JSON")
}

#[test]
fn the_public_loader_materializes_a_v5_run() {
    let fixture = Fixture::new();
    let config = fixture.load();
    assert_eq!(config.schema_version, RUN_SCHEMA_V5);
    assert_eq!(config.execution.fill_timeframe, Timeframe::M5);
    let v5 = config.v5.as_ref().expect("a v5 view");
    assert_eq!(v5.strategy.mode(), StrategyMode::Signal);
    assert!(v5.signal_artifact.is_some());
    assert_eq!(config.references(), Some(&v5.references));
    assert_eq!(
        config.references().expect("references").to_json_text(),
        v5.references.to_json_text()
    );
}

#[test]
fn the_application_service_loads_a_v5_run_with_references() {
    let fixture = Fixture::new();
    let config = fixture
        .service()
        .load_run_with(&fixture.run, &fixture.references)
        .expect("the service loads the v5 run");
    assert_eq!(config.schema_version, RUN_SCHEMA_V5);
    assert_eq!(config.portfolio.initial_cash, 10_000.0);
}

#[test]
fn a_reference_without_bytes_is_rejected_by_the_public_seam() {
    let fixture = Fixture::new();
    let empty = MemoryReferences::new();
    let error = RunConfig::load_with(&fixture.run, &empty).expect_err("the bytes are missing");
    assert!(error.to_string().contains("$.strategy_pack"), "{error}");
}

#[test]
fn the_reference_provenance_is_byte_deterministic() {
    let fixture = Fixture::new();
    let first = fixture.load();
    let second = fixture.load();
    let first = first.references().expect("references").to_json_text();
    let second = second.references().expect("references").to_json_text();
    assert_eq!(first, second);
    assert!(first.contains("\"strategy_pack\""));
    assert!(first.contains("\"signal_artifact\""));
    assert!(first.contains("\"model_artifact\""));
}
