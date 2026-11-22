//! The public signal-artifact seam.
//!
//! The test uses only `quantrule_app::signal`, `quantrule_app::pack`, and the
//! public model-bundle loader. It proves the artifact is causal, deterministic,
//! content-addressed, reusable under different execution and portfolio packs,
//! and that a fold pack or a mismatched source is refused.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use quantrule_api::Digest;
use quantrule_app::pack::{ExecutionPack, PortfolioPack, StrategyPack};
use quantrule_app::signal::{
    SignalAction, SignalArtifact, SignalComputeRequest, SignalError, SignalReason, compute,
};
use quantrule_app::stages::strategy::ModelBundle;
use serde_json::json;

const FIXTURE: &str = "tests/fixtures/signal/model";

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURE)
}

fn pack_document(mode: &str) -> serde_json::Value {
    json!({
        "schema": "quantrule.pack/v1",
        "kind": "strategy",
        "version": "1.0.0",
        "name": format!("signal-event-5m-{mode}"),
        "owner": "operator",
        "payload": {
            "mode": mode,
            "engine": "event_composite",
            "execution_timeframe": "5m",
            "context_timeframes": [],
            "universe": ["BTCUSD"],
            "event_window_bars": 1,
            "time_stop_bars": 0,
            "require_contiguous": true,
            "risk": {"stop_atr_multiple": 2.0, "reward_to_risk": 1.5},
            "entry": [
                {"kind": "event", "timeframe": "5m", "event_type": 3, "subtype": 1,
                 "source": 4, "direction": 1}
            ],
            "exit": []
        }
    })
}

fn signal_pack() -> StrategyPack {
    StrategyPack::parse(&pack_document("signal"), "pack.json").expect("the signal pack is valid")
}

fn fold_pack() -> StrategyPack {
    let mut value = pack_document("fold");
    value["payload"]["time_stop_bars"] = json!(12);
    value["payload"]["giveback_atr"] = json!(1.0);
    value["payload"]["giveback_arm_atr"] = json!(0.5);
    StrategyPack::parse(&value, "pack.json").expect("the fold pack is valid")
}

fn model_artifact_id() -> Digest {
    let root = fixture();
    let entries: Vec<(String, Digest)> = ["provenance.json", "readings.csv", "events.csv"]
        .iter()
        .map(|name| {
            (
                (*name).to_owned(),
                Digest::sha256(&fs::read(root.join(name)).expect("the fixture artifact is read")),
            )
        })
        .collect();
    Digest::canonical(&entries)
}

fn compute_fixture(pack: &StrategyPack) -> SignalArtifact {
    let bundle = ModelBundle::load(&fixture()).expect("the fixture model bundle loads");
    let model = model_artifact_id();
    compute(&SignalComputeRequest {
        bundle: &bundle,
        pack,
        model_artifact_id: &model,
    })
    .expect("the signal computes")
}

fn temp_dir() -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let root = std::env::temp_dir().join(format!(
        "quantrule-signal-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&root).expect("the temp directory is created");
    root
}

fn copy_truncated_model(rows: usize) -> PathBuf {
    let root = temp_dir();
    let model = root.join("model");
    fs::create_dir_all(&model).expect("the model directory is created");
    for name in ["provenance.json", "events.csv"] {
        fs::copy(fixture().join(name), model.join(name)).expect("the fixture file is copied");
    }
    let readings = fs::read_to_string(fixture().join("readings.csv")).expect("readings are read");
    let mut lines = readings.lines();
    let mut truncated = String::new();
    truncated.push_str(lines.next().expect("the header"));
    truncated.push('\n');
    for line in lines.take(rows) {
        truncated.push_str(line);
        truncated.push('\n');
    }
    fs::write(model.join("readings.csv"), truncated).expect("the truncated readings are written");
    root
}

#[test]
fn the_fixture_artifact_has_the_golden_identity_and_rows() {
    let pack = signal_pack();
    let artifact = compute_fixture(&pack);
    assert_eq!(artifact.strategy_pack_id().to_wire(), pack.id().to_wire());
    assert_eq!(
        artifact.model_artifact_id().to_wire(),
        model_artifact_id().to_wire()
    );
    assert_eq!(artifact.data_version(), "fixture-data-v1");
    assert_eq!(artifact.timeframe().code(), "5m");
    assert_eq!(artifact.period_from(), "2024-01-01");
    assert_eq!(artifact.period_to(), "2024-12-31");
    assert_eq!(artifact.rows().len(), 6);
    assert_eq!(artifact.rows()[0].action, SignalAction::Hold);
    assert_eq!(artifact.rows()[1].action, SignalAction::Enter);
    assert_eq!(artifact.rows()[1].reason, Some(SignalReason::Entry));
    assert_eq!(artifact.rows()[2].action, SignalAction::Hold);
    assert_eq!(artifact.rows()[3].action, SignalAction::Exit);
    assert_eq!(artifact.rows()[3].reason, Some(SignalReason::TakeProfit));
    artifact
        .verify_sources(&pack, &model_artifact_id())
        .expect("the frozen sources match");
    // The golden id is the SHA-256 of the stored canonical bytes. A schema,
    // pack, model, row or serializer change fails here loudly.
    assert_eq!(
        artifact.id().to_wire(),
        "sha256:2551537003307cd1c2b324e3eb006c1a7f14abef6ede728f0cf9510ffaaf7689"
    );
    for forbidden in ["quantity", "fee", "cash", "equity", "pnl", "edge"] {
        assert!(
            !artifact.canonical_text().contains(forbidden),
            "the signal artifact must not carry {forbidden}"
        );
    }
}

#[test]
fn the_same_inputs_produce_the_same_bytes() {
    let pack = signal_pack();
    let first = compute_fixture(&pack);
    let second = compute_fixture(&pack);
    assert_eq!(first.id(), second.id());
    assert_eq!(first.canonical_bytes(), second.canonical_bytes());
}

#[test]
fn a_later_bar_cannot_change_an_earlier_row() {
    let pack = signal_pack();
    let full = compute_fixture(&pack);
    let root = copy_truncated_model(4);
    let truncated = ModelBundle::load(&root.join("model")).expect("the truncated bundle loads");
    let model = model_artifact_id();
    let artifact = compute(&SignalComputeRequest {
        bundle: &truncated,
        pack: &pack,
        model_artifact_id: &model,
    })
    .expect("the truncated signal computes");
    assert_eq!(artifact.rows(), &full.rows()[..4]);
    fs::remove_dir_all(root).ok();
}

#[test]
fn the_signal_is_reusable_under_two_execution_and_portfolio_packs() {
    let pack = signal_pack();
    let before = compute_fixture(&pack).canonical_bytes().to_vec();
    let conservative = execution_pack(2.0);
    let aggressive = execution_pack(0.5);
    let single = portfolio_pack(0.05);
    let larger = portfolio_pack(0.10);
    assert_ne!(conservative.id(), aggressive.id());
    assert_ne!(single.id(), larger.id());
    let after = compute_fixture(&pack).canonical_bytes().to_vec();
    assert_eq!(
        before, after,
        "the signal must not depend on execution or portfolio policy"
    );
}

#[test]
fn a_fold_pack_is_refused() {
    let bundle = ModelBundle::load(&fixture()).expect("the fixture model bundle loads");
    let model = model_artifact_id();
    let error = compute(&SignalComputeRequest {
        bundle: &bundle,
        pack: &fold_pack(),
        model_artifact_id: &model,
    })
    .expect_err("a fold pack has no signal artifact");
    assert!(matches!(error, SignalError::FoldMode));
}

#[test]
fn a_mismatched_model_artifact_is_refused() {
    let artifact = compute_fixture(&signal_pack());
    let wrong = Digest::sha256(b"another-model");
    let error = artifact
        .verify_sources(&signal_pack(), &wrong)
        .expect_err("the model id differs");
    assert!(matches!(
        error,
        SignalError::SourceMismatch {
            field: "model_artifact_id",
            ..
        }
    ));
}

fn execution_pack(slippage_bps: f64) -> ExecutionPack {
    ExecutionPack::parse(
        &json!({
            "schema": "quantrule.pack/v1",
            "kind": "execution",
            "version": "1.0.0",
            "name": format!("fills-{slippage_bps}"),
            "owner": "operator",
            "payload": {
                "fill_timeframe": "5m",
                "order_type": "market_next_open",
                "tie_break": "stop_first",
                "fee_rate": 0.0,
                "slippage_bps": slippage_bps,
                "commission": {"rate": 0.0, "base": "equity", "per": "operation"},
                "funding": {"rate": 0.0, "interval_hours": 8},
                "zero_volume": "reject",
                "end_of_data": "leave_open"
            }
        }),
        "execution.json",
    )
    .expect("the execution pack is valid")
}

fn portfolio_pack(fraction: f64) -> PortfolioPack {
    PortfolioPack::parse(
        &json!({
            "schema": "quantrule.pack/v1",
            "kind": "portfolio",
            "version": "1.0.0",
            "name": format!("size-{fraction}"),
            "owner": "operator",
            "payload": {
                "sizing": {"mode": "equity_fraction", "fraction": fraction, "quantity_step": 0.0001},
                "risk": {"per_trade_risk": 0.01, "max_exposure": 1.0, "max_open_positions": 1},
                "drawdown": {"limit": 0.15, "action": "halt"}
            }
        }),
        "portfolio.json",
    )
    .expect("the portfolio pack is valid")
}
