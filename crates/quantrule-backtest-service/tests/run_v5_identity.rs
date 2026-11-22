//! A run-v5 content hash reaches the run identity.
//!
//! The app loader owns the config hash and `quantrule-service-api` owns the
//! run-id formula. This test joins them: every pack and artifact variation
//! changes the config hash and therefore the derived `RunId`, while the data,
//! model version, and image digest stay fixed.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use quantrule_api::{Digest, RunId};
use quantrule_app::config::RunConfig;
use quantrule_app::pack::references::{MODEL_ARTIFACT_SCHEMA, SIGNAL_ARTIFACT_SCHEMA};
use quantrule_app::pack::{ContentReference, MemoryReferences, ReferenceKind, canonical_json};
use quantrule_service_api::derive_run_id;
use serde_json::{Value, json};

static NEXT: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy)]
struct PackValues {
    reward: f64,
    slippage_bps: f64,
    max_exposure: f64,
}

impl PackValues {
    const DEFAULT: Self = Self {
        reward: 1.5,
        slippage_bps: 2.0,
        max_exposure: 1.0,
    };
}

struct Fixture {
    dir: PathBuf,
    references: MemoryReferences,
    run: PathBuf,
    strategy: ContentReference,
    execution: ContentReference,
    portfolio: ContentReference,
    signal: Option<ContentReference>,
}

impl Fixture {
    fn new(
        label: &str,
        mode: &str,
        signal_data_version: &str,
        model_seed: &[u8],
        values: PackValues,
    ) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "quantrule-run-v5-identity-{}-{label}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).expect("the fixture directory is created");
        let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../quantrule-app/tests/fixtures/run-v5");
        fs::copy(fixtures.join("data.json"), dir.join("data.json"))
            .expect("the data fixture is copied");
        fs::copy(fixtures.join("model.json"), dir.join("model.json"))
            .expect("the model fixture is copied");

        let mut references = MemoryReferences::new();
        let strategy = references.insert(
            ReferenceKind::StrategyPack,
            canonical_json(&strategy_pack(mode, values.reward)).into_bytes(),
        );
        let execution = references.insert(
            ReferenceKind::ExecutionModel,
            canonical_json(&execution_pack(values.slippage_bps)).into_bytes(),
        );
        let portfolio = references.insert(
            ReferenceKind::PortfolioPolicy,
            canonical_json(&portfolio_pack(values.max_exposure)).into_bytes(),
        );
        let model = if mode == "signal" {
            Some(insert_model_artifact(
                &mut references,
                model_manifest(model_seed),
            ))
        } else {
            None
        };
        let signal = if mode == "signal" {
            let model = model.as_ref().expect("a signal run names a model");
            Some(references.insert(
                ReferenceKind::SignalArtifact,
                signal_artifact(&strategy.hash, &model.hash, signal_data_version),
            ))
        } else {
            None
        };

        let mut document = json!({
            "schema": "quantrule.run/v5",
            "name": "v5-identity",
            "model_id": "M1.0",
            "data": "data.json",
            "model": "model.json",
            "strategy_pack": strategy.to_value(),
            "portfolio_policy": portfolio.to_value(),
            "execution_model": execution.to_value(),
            "account": {"initial_cash": 10000.0, "currency": "USD"},
            "output_root": "out"
        });
        let object = document
            .as_object_mut()
            .expect("the run document is an object");
        if let Some(model) = &model {
            object.insert("model_artifact".to_owned(), model.to_value());
        }
        if let Some(signal) = &signal {
            object.insert("signal_artifact".to_owned(), signal.to_value());
        }
        let run = dir.join("run.json");
        fs::write(
            &run,
            serde_json::to_vec_pretty(&document).expect("the run document is JSON"),
        )
        .expect("the run document is written");
        Self {
            dir,
            references,
            run,
            strategy,
            execution,
            portfolio,
            signal,
        }
    }

    fn load(&self) -> RunConfig {
        RunConfig::load_with(&self.run, &self.references).expect("the v5 run loads")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn run_id(config_hash: &str) -> RunId {
    let config = Digest::parse(config_hash).expect("the config hash is a digest");
    derive_run_id(
        &Digest::sha256(b"data"),
        &config,
        "M1.0",
        &Digest::sha256(b"image"),
    )
}

fn strategy_pack(mode: &str, reward: f64) -> Value {
    json!({
        "schema": "quantrule.pack/v1",
        "kind": "strategy",
        "version": "1.0.0",
        "name": "compression-fade-5m",
        "owner": "operator",
        "payload": {
            "mode": mode,
            "engine": "event_composite",
            "execution_timeframe": "5m",
            "context_timeframes": ["15m"],
            "universe": ["BTCUSD"],
            "event_window_bars": 1,
            "time_stop_bars": 0,
            "giveback_atr": null,
            "giveback_arm_atr": 0.0,
            "require_contiguous": true,
            "risk": {"stop_atr_multiple": 2.0, "reward_to_risk": reward},
            "entry": [
                {"kind": "event", "timeframe": "5m", "event_type": 3, "subtype": 7,
                 "source": 5, "direction": 0, "contrarian": true}
            ],
            "exit": []
        }
    })
}

fn execution_pack(slippage_bps: f64) -> Value {
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
            "slippage_bps": slippage_bps,
            "commission": {"rate": 0.001, "base": "equity", "per": "operation"},
            "funding": {"rate": 0.0001, "interval_hours": 8},
            "zero_volume": "reject",
            "end_of_data": "leave_open"
        }
    })
}

fn portfolio_pack(max_exposure: f64) -> Value {
    json!({
        "schema": "quantrule.pack/v1",
        "kind": "portfolio",
        "version": "1.0.0",
        "name": "single-position-conservative",
        "owner": "operator",
        "payload": {
            "sizing": {"mode": "equity_fraction", "fraction": 0.05, "quantity_step": 0.0001},
            "risk": {"per_trade_risk": 0.01, "max_exposure": max_exposure, "max_open_positions": 1},
            "drawdown": {"limit": 0.15, "action": "halt"},
            "rebalance": "none",
            "constraints": []
        }
    })
}

fn model_manifest(artifact_seed: &[u8]) -> Value {
    json!({
        "schema": MODEL_ARTIFACT_SCHEMA,
        "data_version": "btcusd-2024",
        "config_id": "cfg_test",
        "config_hash": Digest::sha256(b"config"),
        "model_version": "M1.0",
        "ladder": ["1m", "5m", "15m", "1h"],
        "manifest_key": "models/bundle/manifest.json",
        "artifacts": [
            {"name": "model/result.json", "bytes": 2, "sha256": Digest::sha256(artifact_seed)}
        ]
    })
}

fn insert_model_artifact(references: &mut MemoryReferences, manifest: Value) -> ContentReference {
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
    let id = Digest::canonical(&entries);
    let reference = ContentReference {
        id: id.clone(),
        hash: id,
    };
    references.insert_at(
        ReferenceKind::ModelArtifact,
        reference.clone(),
        serde_json::to_vec(&manifest).expect("the model manifest is JSON"),
    );
    reference
}

fn signal_artifact(strategy: &Digest, model: &Digest, data_version: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "schema": SIGNAL_ARTIFACT_SCHEMA,
        "strategy_pack_id": strategy,
        "model_artifact_id": model,
        "data_version": data_version,
        "timeframe": "5m",
        "period": {"from": "2024-01-01", "to": "2024-12-31"},
        "rows": []
    }))
    .expect("the signal artifact is JSON")
}

#[test]
fn the_model_artifact_schema_matches_the_internal_contract() {
    assert_eq!(
        MODEL_ARTIFACT_SCHEMA,
        quantrule_service_api::MODEL_BUNDLE_SCHEMA
    );
}

#[test]
fn every_pack_and_artifact_hash_changes_the_run_id() {
    let base = Fixture::new("base", "fold", "btcusd-2024", b"a", PackValues::DEFAULT);
    let base_config = base.load();
    let base_run = run_id(&base_config.config_hash);
    assert_eq!(
        base_config
            .references()
            .expect("references")
            .signal_artifact,
        None
    );
    assert_eq!(
        base_config.reference(ReferenceKind::StrategyPack),
        Some(&base.strategy)
    );
    assert_eq!(
        base_config.reference(ReferenceKind::ExecutionModel),
        Some(&base.execution)
    );
    assert_eq!(
        base_config.reference(ReferenceKind::PortfolioPolicy),
        Some(&base.portfolio)
    );

    for (label, values) in [
        (
            "strategy",
            PackValues {
                reward: 1.6,
                ..PackValues::DEFAULT
            },
        ),
        (
            "execution",
            PackValues {
                slippage_bps: 3.0,
                ..PackValues::DEFAULT
            },
        ),
        (
            "portfolio",
            PackValues {
                max_exposure: 0.5,
                ..PackValues::DEFAULT
            },
        ),
    ] {
        let fixture = Fixture::new(label, "fold", "btcusd-2024", b"a", values);
        let config = fixture.load();
        assert_ne!(
            config.config_hash, base_config.config_hash,
            "{label} must change the config hash"
        );
        assert_ne!(
            run_id(&config.config_hash),
            base_run,
            "{label} must change the run id"
        );
    }

    let signal_base = Fixture::new(
        "signal-base",
        "signal",
        "btcusd-2024",
        b"a",
        PackValues::DEFAULT,
    );
    let signal_config = signal_base.load();
    let signal_run = run_id(&signal_config.config_hash);
    assert!(
        signal_config
            .references()
            .expect("references")
            .signal_artifact
            .is_some()
    );

    let model_changed = Fixture::new(
        "signal-model",
        "signal",
        "btcusd-2024",
        b"b",
        PackValues::DEFAULT,
    );
    assert_ne!(
        model_changed.load().config_hash,
        signal_config.config_hash,
        "the model artifact must change the config hash"
    );
    assert_ne!(
        run_id(&model_changed.load().config_hash),
        signal_run,
        "the model artifact must change the run id"
    );

    let signal_changed = Fixture::new(
        "signal-artifact",
        "signal",
        "btcusd-2024-b",
        b"a",
        PackValues::DEFAULT,
    );
    assert_ne!(
        signal_changed.load().config_hash,
        signal_config.config_hash,
        "the signal artifact must change the config hash"
    );
    assert_ne!(
        run_id(&signal_changed.load().config_hash),
        signal_run,
        "the signal artifact must change the run id"
    );
    assert!(signal_base.signal.is_some());
}
