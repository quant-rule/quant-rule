//! A v5 run end to end through the application service.
//!
//! The test builds a model bundle from a small candle fixture, resolves the
//! three packs a fold-mode run needs, and replays the fold twice. The canonical
//! digest must match the checked-in golden value and must not depend on whether
//! the two folds run one after the other or concurrently.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use quantrule_api::Digest;
use quantrule_app::artifact::model_bundle::model_bundle_from;
use quantrule_app::config::{RUN_SCHEMA_V5, RunConfig};
use quantrule_app::pack::{MemoryReferences, ReferenceKind, canonical_json};
use quantrule_app::pipeline;
use quantrule_app::service::AppService;
use serde_json::{Value, json};

const DATA_JSON: &str = include_str!("fixtures/run-v5/data.json");
const MODEL_JSON: &str = include_str!("fixtures/run-v5/model.json");
const GOLDEN_DIGEST: &str = include_str!("fixtures/run-v5/golden-digest.txt");

static NEXT: AtomicU64 = AtomicU64::new(1);

struct Fixture {
    root: PathBuf,
    run: PathBuf,
    references: MemoryReferences,
}

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "quantrule-run-v5-e2e-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).expect("the fixture directory");
        fs::write(root.join("data.json"), DATA_JSON).expect("data.json");
        fs::write(root.join("model.json"), MODEL_JSON).expect("model.json");
        fs::write(root.join("candles.csv"), candles()).expect("candles.csv");

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

        let run = root.join("run.json");
        let document = json!({
            "schema": RUN_SCHEMA_V5,
            "name": "v5-e2e",
            "model_id": "M1.0",
            "data": "data.json",
            "model": "model.json",
            "strategy_pack": strategy.to_value(),
            "portfolio_policy": portfolio.to_value(),
            "execution_model": execution.to_value(),
            "account": {"initial_cash": 10000.0, "currency": "USD"},
            "output_root": "out"
        });
        fs::write(
            &run,
            serde_json::to_vec_pretty(&document).expect("run.json"),
        )
        .expect("run.json");
        Self {
            root,
            run,
            references,
        }
    }

    fn load(&self) -> RunConfig {
        RunConfig::load_with(&self.run, &self.references).expect("the v5 run loads")
    }

    fn model_bundle(&self, config: &RunConfig) -> PathBuf {
        let out = self.root.join("model");
        let run = pipeline::run_model(config).expect("the model run");
        model_bundle_from(config, &out, &run)
            .expect("the model bundle")
            .path
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn candles() -> String {
    let mut csv = String::from("Date,Open,High,Low,Close,Volume\n");
    for offset in 0..250 {
        let hour = offset / 60;
        let minute = offset % 60;
        let open = 100.0 + f64::from(offset) * 0.01;
        csv.push_str(&format!(
            "2024-01-01 {hour:02}:{minute:02}:00+00:00,{open},{},{},{},10\n",
            open + 0.5,
            open - 0.5,
            open + 0.25
        ));
    }
    csv
}

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

fn read_bundle(root: &Path) -> Vec<(String, Vec<u8>)> {
    fn walk(root: &Path, directory: &Path, files: &mut Vec<(String, Vec<u8>)>) {
        let mut entries: Vec<PathBuf> = fs::read_dir(directory)
            .expect("the bundle directory")
            .map(|entry| entry.expect("a bundle entry").path())
            .collect();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                walk(root, &path, files);
            } else if path.is_file() {
                let name = path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                files.push((name, fs::read(&path).expect("the artifact bytes")));
            }
        }
    }
    let mut files = Vec::new();
    walk(root, root, &mut files);
    files.sort_by(|left, right| left.0.cmp(&right.0));
    files
}

/// The canonical digest the run contract defines: `results.json`, `*.csv`,
/// `metrics.json`, and `*.ndjson`, sorted by path.
fn canonical_digest(files: &[(String, Vec<u8>)]) -> Digest {
    let entries: Vec<(String, Digest)> = files
        .iter()
        .filter(|(name, _)| {
            name == "results.json"
                || name.ends_with(".csv")
                || name.ends_with(".ndjson")
                || name
                    .rsplit('/')
                    .next()
                    .is_some_and(|file| file == "metrics.json")
        })
        .map(|(name, bytes)| (name.clone(), Digest::sha256(bytes)))
        .collect();
    Digest::canonical(&entries)
}

fn run(fixture: &Fixture, model: &Path, out: &Path) -> Digest {
    let config = fixture.load();
    let strategy = config.strategy.clone().expect("the strategy");
    let strategy_id = config
        .v5
        .as_ref()
        .expect("v5")
        .references
        .strategy_pack
        .hash
        .to_string();
    let outcome = AppService::run_backtest_from_model_resolved(
        &config,
        &strategy,
        &strategy_id,
        model,
        Some(out),
    )
    .expect("the v5 fold");
    canonical_digest(&read_bundle(&outcome.root))
}

#[test]
fn a_v5_run_matches_its_golden_digest() {
    let fixture = Fixture::new();
    let config = fixture.load();
    let model = fixture.model_bundle(&config);
    let out = fixture.root.join("one");
    let digest = run(&fixture, &model, &out);
    assert_eq!(
        digest.to_string(),
        GOLDEN_DIGEST.trim(),
        "regenerate the golden only for a deliberate model or fold change"
    );
}

#[test]
fn one_fold_and_two_concurrent_folds_produce_the_same_digest() {
    let fixture = Fixture::new();
    let config = fixture.load();
    let model = fixture.model_bundle(&config);
    let sequential = run(&fixture, &model, &fixture.root.join("sequential"));

    let concurrent: Vec<Digest> = std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for name in ["left", "right"] {
            let model = model.clone();
            let out = fixture.root.join(name);
            let fixture = &fixture;
            handles.push(scope.spawn(move || run(fixture, &model, &out)));
        }
        handles
            .into_iter()
            .map(|handle| handle.join().expect("the fold thread"))
            .collect()
    });
    assert_eq!(concurrent[0], sequential);
    assert_eq!(concurrent[1], sequential);
}
