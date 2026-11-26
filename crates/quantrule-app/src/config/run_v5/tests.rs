//! Tests for the `quantrule.run/v5` loader.
//!
//! The fixture writes the data and model files from the repository fixture and
//! builds packs in memory, so a pack mutation changes exactly one content hash.
//! Every test uses the public loader and the public reference seam.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use quantrule_api::Digest;
use quantrule_backtest::portfolio::{Currency, SizingMode};
use quantrule_data::Timeframe;
use serde_json::{Value, json};

use super::super::run::{RUN_SCHEMA_V3, RUN_SCHEMA_V5, RunConfig};
use crate::config::ConfigErrorKind;
use crate::pack::references::{MODEL_ARTIFACT_SCHEMA, SIGNAL_ARTIFACT_SCHEMA};
use crate::pack::{
    ContentReference, MemoryReferences, ReferenceKind, StrategyMode, canonical_json,
};

const DATA_JSON: &str = include_str!("../../../tests/fixtures/run-v5/data.json");
const MODEL_JSON: &str = include_str!("../../../tests/fixtures/run-v5/model.json");

static NEXT: AtomicU64 = AtomicU64::new(1);

#[derive(Clone)]
struct Refs {
    model: Option<ContentReference>,
    strategy: ContentReference,
    signal: Option<ContentReference>,
    portfolio: ContentReference,
    execution: ContentReference,
}

struct PackSet {
    strategy: Value,
    execution: Value,
    portfolio: Value,
    model: Option<Value>,
    signal: bool,
    signal_data_version: &'static str,
    account_cash: f64,
}

impl PackSet {
    fn fold() -> Self {
        Self {
            strategy: strategy_pack("fold", 1.5),
            execution: execution_pack(2.0),
            portfolio: portfolio_pack(1.0),
            model: None,
            signal: false,
            signal_data_version: "btcusd-2024",
            account_cash: 10_000.0,
        }
    }

    fn signal() -> Self {
        Self {
            strategy: strategy_pack("signal", 1.5),
            model: Some(model_manifest("M1.0", &["1m", "5m", "15m", "1h"], b"a")),
            signal: true,
            ..Self::fold()
        }
    }
}

struct Fixture {
    dir: PathBuf,
    references: MemoryReferences,
    refs: Refs,
    account_cash: f64,
}

impl Fixture {
    fn new(label: &str, packs: PackSet) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "quantrule-run-v5-{}-{label}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).expect("the fixture directory is created");
        fs::write(dir.join("data.json"), DATA_JSON).expect("the data fixture is written");
        fs::write(dir.join("model.json"), MODEL_JSON).expect("the model fixture is written");

        let mut references = MemoryReferences::new();
        let strategy = references.insert(
            ReferenceKind::StrategyPack,
            canonical_json(&packs.strategy).into_bytes(),
        );
        let execution = references.insert(
            ReferenceKind::ExecutionModel,
            canonical_json(&packs.execution).into_bytes(),
        );
        let portfolio = references.insert(
            ReferenceKind::PortfolioPolicy,
            canonical_json(&packs.portfolio).into_bytes(),
        );
        let model = packs
            .model
            .map(|value| insert_model_artifact(&mut references, value));
        let signal = if packs.signal {
            let model = model
                .as_ref()
                .expect("a signal fixture names a model artifact");
            Some(references.insert(
                ReferenceKind::SignalArtifact,
                signal_artifact(&strategy.hash, &model.hash, packs.signal_data_version),
            ))
        } else {
            None
        };
        let fixture = Self {
            dir,
            references,
            refs: Refs {
                model,
                strategy,
                signal,
                portfolio,
                execution,
            },
            account_cash: packs.account_cash,
        };
        fixture.write_run(if packs.signal {
            "run-signal.json"
        } else {
            "run.json"
        });
        fixture
    }

    fn write_run(&self, name: &str) {
        let mode = if self.refs.signal.is_some() {
            "signal"
        } else {
            "fold"
        };
        let mut document = json!({
            "schema": RUN_SCHEMA_V5,
            "name": format!("v5-{mode}"),
            "model_id": "M1.0",
            "data": "data.json",
            "model": "model.json",
            "strategy_pack": self.refs.strategy.to_value(),
            "portfolio_policy": self.refs.portfolio.to_value(),
            "execution_model": self.refs.execution.to_value(),
            "account": {"initial_cash": self.account_cash, "currency": "USD"},
            "output_root": "out"
        });
        let object = document
            .as_object_mut()
            .expect("the run document is an object");
        if let Some(model) = &self.refs.model {
            object.insert("model_artifact".to_owned(), model.to_value());
        }
        if let Some(signal) = &self.refs.signal {
            object.insert("signal_artifact".to_owned(), signal.to_value());
        }
        self.write_document(name, &document);
    }

    fn write_document(&self, name: &str, document: &Value) {
        fs::write(
            self.dir.join(name),
            serde_json::to_vec_pretty(document).expect("the run document is JSON"),
        )
        .expect("the run document is written");
    }

    fn load(&self, name: &str) -> Result<RunConfig, crate::config::ConfigError> {
        RunConfig::load_with(&self.dir.join(name), &self.references)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
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

fn model_manifest(version: &str, ladder: &[&str], artifact_seed: &[u8]) -> Value {
    json!({
        "schema": MODEL_ARTIFACT_SCHEMA,
        "data_version": "btcusd-2024",
        "config_id": "cfg_test",
        "config_hash": Digest::sha256(b"config"),
        "model_version": version,
        "ladder": ladder,
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

fn error<T: std::fmt::Debug>(
    result: Result<T, crate::config::ConfigError>,
) -> crate::config::ConfigError {
    result.expect_err("the run must be rejected")
}

#[test]
fn a_fold_run_materializes_every_pack() {
    let fixture = Fixture::new("fold", PackSet::fold());
    let config = fixture.load("run.json").expect("the fold run loads");
    assert_eq!(config.schema_version, RUN_SCHEMA_V5);
    assert!(config.paths.strategy.is_none());
    assert!(config.paths.execution.is_none());
    assert!(config.paths.portfolio.is_none());
    assert_eq!(config.execution.fill_timeframe, Timeframe::M5);
    match config.portfolio.sizing {
        SizingMode::EquityFraction { fraction, .. } => assert_eq!(fraction, 0.05),
        other => panic!("unexpected sizing: {other:?}"),
    }
    assert_eq!(config.portfolio.initial_cash, 10_000.0);
    assert_eq!(config.portfolio.currency, Currency::Usd);
    assert_eq!(config.portfolio.max_open_positions, 1);
    let v5 = config.v5.as_ref().expect("a v5 run has a v5 view");
    assert_eq!(v5.strategy.mode(), StrategyMode::Fold);
    assert!(v5.signal_artifact.is_none());
    assert_eq!(v5.account.initial_cash, 10_000.0);
    assert_eq!(config.references(), Some(&v5.references));
}

#[test]
fn a_signal_run_carries_the_signal_artifact() {
    let fixture = Fixture::new("signal", PackSet::signal());
    let config = fixture
        .load("run-signal.json")
        .expect("the signal run loads");
    let v5 = config.v5.as_ref().expect("a v5 run has a v5 view");
    assert_eq!(v5.strategy.mode(), StrategyMode::Signal);
    let signal = v5
        .signal_artifact
        .as_ref()
        .expect("a signal run carries a signal artifact");
    assert_eq!(signal.timeframe, Timeframe::M5);
    assert_eq!(signal.strategy_pack_id, fixture.refs.strategy.hash);
    assert_eq!(
        signal.model_artifact_id,
        fixture.refs.model.as_ref().expect("a model reference").hash
    );
}

#[test]
fn a_v5_model_bundle_lists_only_the_model_inputs() {
    let fixture = Fixture::new("model-files", PackSet::fold());
    let config = fixture.load("run.json").expect("the v5 run loads");
    assert_eq!(
        crate::artifact::model_bundle::config_files(&config),
        vec![
            "run.json".to_owned(),
            "data.json".to_owned(),
            "model.json".to_owned(),
        ]
    );
}

#[test]
fn a_v5_run_needs_supplied_references() {
    let fixture = Fixture::new("needs-resolver", PackSet::fold());
    let error = error(RunConfig::load(&fixture.dir.join("run.json")));
    assert_eq!(error.kind(), ConfigErrorKind::MissingFile);
    assert_eq!(error.path(), "$.strategy_pack");
}

#[test]
fn a_structural_load_returns_the_reference_set_and_hash() {
    let fixture = Fixture::new("structural", PackSet::fold());
    let (hash, references) =
        RunConfig::load_v5_references(&fixture.dir.join("run.json")).expect("the structural load");
    let full = fixture.load("run.json").expect("the full load");
    assert_eq!(hash, full.config_hash);
    assert_eq!(references.len(), 3);
    assert_eq!(references[0].kind, ReferenceKind::StrategyPack);
    assert_eq!(references[1].kind, ReferenceKind::PortfolioPolicy);
    assert_eq!(references[2].kind, ReferenceKind::ExecutionModel);
    assert!(references.iter().all(|reference| reference.is_consistent()));
    assert!(references.iter().all(|reference| reference.pack.is_none()));
}

#[test]
fn a_missing_reference_names_the_field() {
    let fixture = Fixture::new("missing", PackSet::fold());
    let empty = MemoryReferences::new();
    let error = error(RunConfig::load_with(&fixture.dir.join("run.json"), &empty));
    assert_eq!(error.kind(), ConfigErrorKind::MissingFile);
    assert_eq!(error.path(), "$.strategy_pack");
}

#[test]
fn a_reference_id_must_equal_its_hash() {
    let fixture = Fixture::new("id-hash", PackSet::fold());
    let mut document = run_document(&fixture);
    document["strategy_pack"]["hash"] = json!(Digest::sha256(b"other"));
    fixture.write_document("run.json", &document);
    let error = error(fixture.load("run.json"));
    assert_eq!(error.kind(), ConfigErrorKind::Inconsistent);
    assert_eq!(error.path(), "$.strategy_pack");
}

#[test]
fn stored_bytes_must_match_the_reference_hash() {
    let fixture = Fixture::new("hash", PackSet::fold());
    let wrong = ContentReference {
        id: Digest::sha256(b"other"),
        hash: Digest::sha256(b"other"),
    };
    let mut references = MemoryReferences::new();
    references.insert_at(
        ReferenceKind::StrategyPack,
        wrong.clone(),
        canonical_json(&strategy_pack("fold", 1.5)).into_bytes(),
    );
    let mut document = run_document(&fixture);
    document["strategy_pack"] = wrong.to_value();
    fixture.write_document("run.json", &document);
    let error = error(RunConfig::load_with(
        &fixture.dir.join("run.json"),
        &references,
    ));
    assert_eq!(error.kind(), ConfigErrorKind::Inconsistent);
    assert_eq!(error.path(), "$.strategy_pack");
}

#[test]
fn a_pack_of_the_wrong_kind_is_rejected() {
    let fixture = Fixture::new("wrong-kind", PackSet::fold());
    let mut references = MemoryReferences::new();
    let strategy = references.insert(
        ReferenceKind::StrategyPack,
        canonical_json(&portfolio_pack(1.0)).into_bytes(),
    );
    let mut document = run_document(&fixture);
    document["strategy_pack"] = strategy.to_value();
    fixture.write_document("run.json", &document);
    let message = error(RunConfig::load_with(
        &fixture.dir.join("run.json"),
        &references,
    ))
    .to_string();
    assert!(message.contains("$.strategy_pack"), "{message}");
    assert!(message.contains("strategy pack"), "{message}");
    assert!(message.contains("portfolio pack"), "{message}");
}

#[test]
fn signal_mode_requires_a_signal_artifact() {
    let mut fixture = Fixture::new("signal-missing", PackSet::fold());
    fixture.refs.strategy = fixture.references.insert(
        ReferenceKind::StrategyPack,
        canonical_json(&strategy_pack("signal", 1.5)).into_bytes(),
    );
    fixture.write_run("run.json");
    let error = error(fixture.load("run.json"));
    assert_eq!(error.kind(), ConfigErrorKind::MissingField);
    assert_eq!(error.path(), "$.signal_artifact");
}

#[test]
fn fold_mode_rejects_a_signal_artifact() {
    let fixture = Fixture::new("fold-signal", PackSet::fold());
    let mut document = run_document(&fixture);
    document["signal_artifact"] = fixture.refs.strategy.to_value();
    fixture.write_document("run.json", &document);
    let error = error(fixture.load("run.json"));
    assert_eq!(error.kind(), ConfigErrorKind::Inconsistent);
    assert_eq!(error.path(), "$.signal_artifact");
}

#[test]
fn a_signal_artifact_must_name_the_strategy_pack() {
    let mut fixture = Fixture::new("signal-strategy", PackSet::signal());
    let bytes = signal_artifact(
        &Digest::sha256(b"another-strategy"),
        &fixture.refs.model.as_ref().expect("a model").hash,
        "btcusd-2024",
    );
    fixture.refs.signal = Some(
        fixture
            .references
            .insert(ReferenceKind::SignalArtifact, bytes),
    );
    fixture.write_run("run-signal.json");
    let error = error(fixture.load("run-signal.json"));
    assert_eq!(error.kind(), ConfigErrorKind::Inconsistent);
    assert_eq!(error.path(), "$.signal_artifact");
}

#[test]
fn a_signal_artifact_must_name_the_model_artifact() {
    let mut fixture = Fixture::new("signal-model", PackSet::signal());
    let bytes = signal_artifact(
        &fixture.refs.strategy.hash,
        &Digest::sha256(b"another-model"),
        "btcusd-2024",
    );
    fixture.refs.signal = Some(
        fixture
            .references
            .insert(ReferenceKind::SignalArtifact, bytes),
    );
    fixture.write_run("run-signal.json");
    let error = error(fixture.load("run-signal.json"));
    assert_eq!(error.kind(), ConfigErrorKind::Inconsistent);
    assert_eq!(error.path(), "$.signal_artifact");
}

#[test]
fn a_signal_artifact_must_match_the_execution_timeframe() {
    let mut fixture = Fixture::new("signal-timeframe", PackSet::signal());
    let mut value: Value = serde_json::from_slice(&signal_artifact(
        &fixture.refs.strategy.hash,
        &fixture.refs.model.as_ref().expect("a model").hash,
        "btcusd-2024",
    ))
    .expect("the signal artifact is JSON");
    value["timeframe"] = json!("15m");
    let bytes = serde_json::to_vec(&value).expect("the changed artifact is JSON");
    fixture.refs.signal = Some(
        fixture
            .references
            .insert(ReferenceKind::SignalArtifact, bytes),
    );
    fixture.write_run("run-signal.json");
    let error = error(fixture.load("run-signal.json"));
    assert_eq!(error.kind(), ConfigErrorKind::Inconsistent);
    assert_eq!(error.path(), "$.signal_artifact");
}

#[test]
fn a_signal_run_requires_a_model_artifact() {
    let mut fixture = Fixture::new("signal-no-model", PackSet::signal());
    fixture.refs.model = None;
    fixture.write_run("run-signal.json");
    let error = error(fixture.load("run-signal.json"));
    assert_eq!(error.kind(), ConfigErrorKind::MissingField);
    assert_eq!(error.path(), "$.model_artifact");
}

#[test]
fn a_model_artifact_must_match_the_model_version() {
    let mut packs = PackSet::fold();
    packs.model = Some(model_manifest("M2.0", &["1m", "5m", "15m", "1h"], b"a"));
    let fixture = Fixture::new("model-version", packs);
    let error = error(fixture.load("run.json"));
    assert_eq!(error.kind(), ConfigErrorKind::Inconsistent);
    assert_eq!(error.path(), "$.model_artifact");
}

#[test]
fn a_model_artifact_must_match_the_run_ladder() {
    let mut packs = PackSet::fold();
    packs.model = Some(model_manifest("M1.0", &["5m"], b"a"));
    let fixture = Fixture::new("model-ladder", packs);
    let error = error(fixture.load("run.json"));
    assert_eq!(error.kind(), ConfigErrorKind::Inconsistent);
    assert_eq!(error.path(), "$.model_artifact");
}

#[test]
fn the_execution_model_timeframe_must_match_the_strategy() {
    let mut packs = PackSet::fold();
    packs.execution = execution_pack(2.0);
    packs.execution["payload"]["fill_timeframe"] = json!("15m");
    let fixture = Fixture::new("execution-timeframe", packs);
    let error = error(fixture.load("run.json"));
    assert_eq!(error.kind(), ConfigErrorKind::Inconsistent);
    assert_eq!(error.path(), "$.execution_model");
}

#[test]
fn a_strategy_universe_must_contain_the_data_instrument() {
    let mut packs = PackSet::fold();
    packs.strategy["payload"]["universe"] = json!(["EURUSD"]);
    let fixture = Fixture::new("universe", packs);
    let error = error(fixture.load("run.json"));
    assert_eq!(error.kind(), ConfigErrorKind::Inconsistent);
    assert_eq!(error.path(), "$.strategy_pack");
}

#[test]
fn the_account_cash_stays_in_the_run() {
    let mut packs = PackSet::fold();
    packs.account_cash = 12_345.0;
    let fixture = Fixture::new("account", packs);
    let config = fixture.load("run.json").expect("the run loads");
    assert_eq!(config.portfolio.initial_cash, 12_345.0);
    assert_eq!(
        config.v5.as_ref().expect("v5").account.initial_cash,
        12_345.0
    );
    assert!(
        !canonical_json(&portfolio_pack(1.0)).contains("initial_cash"),
        "the pack must not duplicate the account cash"
    );
}

#[test]
fn a_v5_run_rejects_the_legacy_file_fields() {
    let fixture = Fixture::new("legacy-field", PackSet::fold());
    let mut document = run_document(&fixture);
    document["execution"] = json!("execution.json");
    fixture.write_document("run.json", &document);
    let error = error(fixture.load("run.json"));
    assert_eq!(error.kind(), ConfigErrorKind::Unsupported);
    assert_eq!(error.path(), "$.execution");
}

#[test]
fn a_v5_run_rejects_an_unknown_field() {
    let fixture = Fixture::new("unknown-field", PackSet::fold());
    let mut document = run_document(&fixture);
    document["analytics"] = json!("old.json");
    fixture.write_document("run.json", &document);
    let error = error(fixture.load("run.json"));
    assert_eq!(error.kind(), ConfigErrorKind::UnknownField);
    assert_eq!(error.path(), "$.analytics");
}

#[test]
fn the_legacy_configs_keep_their_golden_hashes() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let v3 = RunConfig::load(
        &root.join("../quantrule-data-service/tests/fixtures/config/valid/run.json"),
    )
    .expect("the v3 fixture loads");
    assert_eq!(v3.schema_version, RUN_SCHEMA_V3);
    assert_eq!(
        v3.config_hash,
        "sha256:a84c64441e071bf31b9b2f9fc7394e69ee915a1502fb3e675ecb6cf23efc70d1"
    );
    assert!(v3.v5.is_none());

    let v4 = RunConfig::load(
        &root.join("../quantrule-data-service/tests/fixtures/config/valid-v4/run.json"),
    )
    .expect("the v4 fixture loads");
    assert_eq!(v4.schema_version, "quantrule.run/v4");
    assert_eq!(
        v4.config_hash,
        "sha256:256956eee01225e00c3205c22d5af104be2885558484ed2481f40209aca8efd9"
    );
    assert!(v4.strategy.is_some());
    assert!(v4.v5.is_none());
}

#[test]
fn every_pack_and_signal_hash_changes_the_config_hash() {
    let fold = Fixture::new("identity-fold", PackSet::fold());
    let base = fold
        .load("run.json")
        .expect("the base run loads")
        .config_hash;

    let mut packs = PackSet::fold();
    packs.strategy = strategy_pack("fold", 1.6);
    let changed_strategy = Fixture::new("identity-strategy", packs);
    let strategy = changed_strategy
        .load("run.json")
        .expect("the run loads")
        .config_hash;
    assert_ne!(
        strategy, base,
        "the strategy pack must change the config hash"
    );

    let mut packs = PackSet::fold();
    packs.execution = execution_pack(3.0);
    let changed_execution = Fixture::new("identity-execution", packs);
    let execution = changed_execution
        .load("run.json")
        .expect("the run loads")
        .config_hash;
    assert_ne!(
        execution, base,
        "the execution pack must change the config hash"
    );

    let mut packs = PackSet::fold();
    packs.portfolio = portfolio_pack(0.5);
    let changed_portfolio = Fixture::new("identity-portfolio", packs);
    let portfolio = changed_portfolio
        .load("run.json")
        .expect("the run loads")
        .config_hash;
    assert_ne!(
        portfolio, base,
        "the portfolio pack must change the config hash"
    );

    let mut packs = PackSet::fold();
    packs.model = Some(model_manifest("M1.0", &["1m", "5m", "15m", "1h"], b"b"));
    let changed_model = Fixture::new("identity-model", packs);
    let model = changed_model
        .load("run.json")
        .expect("the run loads")
        .config_hash;
    assert_ne!(
        model, base,
        "the model artifact must change the config hash"
    );

    let signal = Fixture::new("identity-signal-base", PackSet::signal());
    let signal_base = signal
        .load("run-signal.json")
        .expect("the signal run loads")
        .config_hash;
    let mut packs = PackSet::signal();
    packs.signal_data_version = "btcusd-2024-b";
    let changed_signal = Fixture::new("identity-signal", packs);
    let changed = changed_signal
        .load("run-signal.json")
        .expect("the run loads")
        .config_hash;
    assert_ne!(
        changed, signal_base,
        "the signal artifact must change the config hash"
    );

    for (name, value) in [
        ("strategy", &strategy),
        ("execution", &execution),
        ("portfolio", &portfolio),
        ("model", &model),
    ] {
        assert_ne!(value, &base, "{name} must change the config hash");
    }
}

#[test]
fn the_provenance_value_names_every_reference() {
    let fixture = Fixture::new("provenance", PackSet::signal());
    let config = fixture.load("run-signal.json").expect("the run loads");
    let value = config
        .references_provenance()
        .expect("a v5 run has references");
    assert_eq!(
        value["strategy_pack"]["hash"],
        json!(fixture.refs.strategy.hash)
    );
    assert_eq!(
        value["execution_model"]["hash"],
        json!(fixture.refs.execution.hash)
    );
    assert_eq!(
        value["portfolio_policy"]["hash"],
        json!(fixture.refs.portfolio.hash)
    );
    assert_eq!(
        value["model_artifact"]["hash"],
        json!(fixture.refs.model.as_ref().expect("a model").hash)
    );
    assert_eq!(
        value["signal_artifact"]["hash"],
        json!(fixture.refs.signal.as_ref().expect("a signal").hash)
    );
    let text = config
        .references_provenance()
        .expect("a v5 run has references")
        .to_string();
    assert_eq!(
        text,
        config
            .references_provenance()
            .expect("a v5 run has references")
            .to_string()
    );
}

#[test]
fn the_config_hash_is_deterministic() {
    let first = Fixture::new("deterministic-a", PackSet::fold());
    let second = Fixture::new("deterministic-b", PackSet::fold());
    let first = first.load("run.json").expect("the first run loads");
    let second = second.load("run.json").expect("the second run loads");
    assert_eq!(first.config_hash, second.config_hash);
    assert_eq!(first.model_input_hash, second.model_input_hash);
    let third = Fixture::new("deterministic-c", PackSet::fold())
        .load("run.json")
        .expect("the third run loads");
    assert_eq!(first.config_hash, third.config_hash);
}

fn run_document(fixture: &Fixture) -> Value {
    let mut document = json!({
        "schema": RUN_SCHEMA_V5,
        "name": "v5-fold",
        "model_id": "M1.0",
        "data": "data.json",
        "model": "model.json",
        "strategy_pack": fixture.refs.strategy.to_value(),
        "portfolio_policy": fixture.refs.portfolio.to_value(),
        "execution_model": fixture.refs.execution.to_value(),
        "account": {"initial_cash": fixture.account_cash, "currency": "USD"},
        "output_root": "out"
    });
    let object = document
        .as_object_mut()
        .expect("the run document is an object");
    if let Some(model) = &fixture.refs.model {
        object.insert("model_artifact".to_owned(), model.to_value());
    }
    if let Some(signal) = &fixture.refs.signal {
        object.insert("signal_artifact".to_owned(), signal.to_value());
    }
    document
}
