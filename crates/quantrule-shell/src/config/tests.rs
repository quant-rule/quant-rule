//! Tests for the config loader.
//!
//! Each table row is one rule from the config contract. The row asserts the
//! error category and the JSON path, not only that the file was rejected, so a
//! rule cannot regress into a different rejection unnoticed. The example
//! configs are loaded from the repository; the invalid-JSON and missing-file
//! fixtures live under `tests/fixtures/config/`.

use std::path::{Path, PathBuf};

use quantrule_data::Timeframe;
use serde_json::{Value, json};

use super::error::ConfigErrorKind;
use super::error::ConfigErrorKind::{Inconsistent, MissingField, Range, Unsupported};
use super::{ConfigError, data, execution, model, policy, portfolio, run};

fn set(mut value: Value, path: &[&str], replacement: Value) -> Value {
    let (last, parents) = path.split_last().expect("the path is not empty");
    let mut current = &mut value;
    for parent in parents {
        current = &mut current[*parent];
    }
    current[*last] = replacement;
    value
}

fn remove(mut value: Value, path: &[&str]) -> Value {
    let (last, parents) = path.split_last().expect("the path is not empty");
    let mut current = &mut value;
    for parent in parents {
        current = &mut current[*parent];
    }
    current
        .as_object_mut()
        .expect("the parent is an object")
        .remove(*last);
    value
}

fn assert_rejected<T: std::fmt::Debug>(
    result: Result<T, ConfigError>,
    kind: ConfigErrorKind,
    path: &str,
) {
    let error = result.expect_err("the config must be rejected");
    assert_eq!(error.kind(), kind, "unexpected category: {error}");
    assert_eq!(error.path(), path, "unexpected path: {error}");
}

fn data_object() -> Value {
    json!({
        "schema": "quantrule.data/v3",
        "source": {"kind": "csv", "path": "candles.csv"},
        "instrument": "EURUSD",
        "data_version": "version",
        "from": "2025-01-01",
        "to": "2025-12-31",
        "source_timeframe": "1m",
        "available_timeframes": ["1m", "5m", "15m", "1h", "4h"],
        "max_candles": 600000
    })
}

fn policy_object() -> Value {
    json!({
        "schema": "quantrule.policy/v1",
        "account": { "currency": "USD", "initial_cash": 10000.0 },
        "objective": {
            "performance_target": 0.10,
            "evaluation_days": 90,
            "max_drawdown": 0.20
        },
        "risk": {
            "max_position_fraction": 1.0,
            "max_leverage": 1.0,
            "max_open_positions": 1
        },
        "universe": {
            "tickets": ["EURUSD", "BTCUSD"],
            "timeframes": ["15m"]
        }
    })
}

fn execution_object() -> Value {
    json!({
        "schema": "quantrule.execution/v2",
        "fill_timeframe": "15m",
        "order_type": "market_next_open",
        "tie_break": "stop_first",
        "fee_rate": 0.0,
        "slippage_bps": 2.0,
        "zero_volume": "reject",
        "end_of_data": "leave_open"
    })
}

fn portfolio_object() -> Value {
    json!({
        "schema": "quantrule.portfolio/v2",
        "initial_cash": 10000.0,
        "currency": "USD",
        "sizing": {
            "mode": "fixed_notional",
            "notional": 10000.0,
            "quantity_step": 0.0001
        },
        "leverage": 1.0,
        "max_open_positions": 1,
        "commission": {"rate": 0.001, "base": "equity", "per": "operation"},
        "funding": {"rate": 0.0001, "interval_hours": 8},
        "max_drawdown": {"limit": 0.2, "action": "halt"},
        "mark_to_market": "bar_close"
    })
}

fn model_v4_object() -> Value {
    json!({
        "schema": "quantrule.model/v4",
        "defaults": {
            "ema": {"periods": [20, 50, 200], "active": true},
            "rsi": {"period": 14, "active": true},
            "atr": {"period": 14, "active": true},
            "rvol": {"period": 20, "active": true},
            "macd": {"fast": 12, "slow": 26, "signal": 9, "active": true},
            "adx": {"period": 14, "threshold": 25.0, "slope_bars": 3, "active": true},
            "events": {
                "level": {"references": ["ema", "pivots"], "test_atr": 0.2, "break_atr": 0.2},
                "divergence": {"indicator": "rsi", "pivot_scale": "major", "window": 2},
                "signal": {
                    "rsi_levels": [30.0, 50.0, 70.0],
                    "ema_pairs": [[20, 50], [50, 200], [20, 200]],
                    "macd": ["line_signal_cross", "histogram_slope_flip", "zero_cross"],
                    "di_cross": true,
                    "trend_continuation": {"enabled": true, "adx_threshold": 25.0, "adx_slope_bars": 3},
                    "compression_breakout": {"enabled": true, "coiling_bars": 6, "break_atr": 0.3, "rvol_min": 1.4},
                    "compression_range": {"enabled": true, "coiling_bars": 6},
                    "compression_range_onset": {"enabled": true, "coiling_bars": 6}
                }
            },
            "progression": {
                "target_atr": 2.0,
                "stability_bars": 3,
                "invalidation_atr": 0.2,
                "min_target_atr": 0.2
            }
        },
        "timeframes": {
            "5m": {
                "ema": {"periods": [9, 21, 55]},
                "events": {
                    "level": {"test_atr": 0.3},
                    "signal": {
                        "di_cross": false,
                        "trend_continuation": {"adx_threshold": 30.0},
                        "compression_breakout": {"coiling_bars": 7},
                        "compression_range": {"coiling_bars": 7},
                        "compression_range_onset": {"coiling_bars": 7}
                    }
                },
                "progression": {"target_atr": 3.0}
            }
        }
    })
}

fn run_object() -> Value {
    json!({
        "schema": "quantrule.run/v2",
        "name": "run",
        "model_id": "M1.0",
        "policy_id": "POL1.0",
        "execution_id": "E1.0",
        "portfolio_id": "P1.0",
        "policy": "policy.json",
        "data": "data.json",
        "model": "model.json",
        "execution": "execution.json",
        "portfolio": "portfolio.json",
        "output_root": "out"
    })
}

#[test]
fn data_matches_the_contract() {
    let base = Path::new("config");
    let valid = data::parse(&data_object(), "data.json", base).expect("the valid config loads");
    assert_eq!(valid.instrument, "EURUSD");
    assert_eq!(valid.data_version, "version");
    assert_eq!(valid.source.path, Path::new("config/candles.csv"));
    assert_eq!(valid.source_timeframe, Timeframe::M1);
    assert_eq!(valid.available_timeframes, Timeframe::ALL);
    assert_eq!(valid.max_candles, 600_000);
    assert_eq!(valid.from.as_deref(), Some("2025-01-01"));
    assert_eq!(valid.to.as_deref(), Some("2025-12-31"));
}

#[test]
fn data_rejects_a_missing_source_and_an_inverted_range() {
    for value in [
        remove(data_object(), &["source"]),
        set(data_object(), &["from"], json!("2026-01-01")),
        set(data_object(), &["source_timeframe"], json!("5m")),
    ] {
        assert!(data::parse(&value, "data.json", Path::new("config")).is_err());
    }
    assert_rejected(
        data::parse(
            &remove(data_object(), &["source"]),
            "data.json",
            Path::new("config"),
        ),
        MissingField,
        "$.source",
    );
}

#[test]
fn policy_matches_the_contract() {
    let valid = policy::parse(&policy_object(), "policy.json").expect("the valid policy loads");
    assert_eq!(valid.account.currency, "USD");
    assert_eq!(valid.account.initial_cash, 10_000.0);
    assert_eq!(valid.objective.performance_target, 0.10);
    assert_eq!(valid.universe.tickets, ["EURUSD", "BTCUSD"]);
}

#[test]
fn policy_rejects_an_out_of_range_risk_envelope() {
    for value in [
        set(policy_object(), &["risk", "max_leverage"], json!(2.0)),
        set(policy_object(), &["risk", "max_open_positions"], json!(2)),
        set(policy_object(), &["objective", "max_drawdown"], json!(1.5)),
        set(policy_object(), &["universe", "tickets"], json!([])),
    ] {
        assert!(policy::parse(&value, "policy.json").is_err());
    }
}

#[test]
fn execution_matches_the_contract() {
    let valid = execution::parse(&execution_object(), "execution.json").expect("valid execution");
    assert_eq!(valid.fill_timeframe, Timeframe::M15);
}

#[test]
fn execution_rejects_an_unknown_timeframe() {
    assert_rejected(
        execution::parse(
            &set(execution_object(), &["fill_timeframe"], json!("2m")),
            "execution.json",
        ),
        Range,
        "$.fill_timeframe",
    );
}

#[test]
fn portfolio_matches_the_contract() {
    let valid = portfolio::parse(&portfolio_object(), "portfolio.json").expect("valid portfolio");
    assert_eq!(valid.initial_cash, 10_000.0);
    assert_eq!(valid.max_open_positions, 1);
}

#[test]
fn portfolio_rejects_the_removed_allow_short_field() {
    let mut value = portfolio_object();
    value["allow_short"] = json!(true);
    assert_rejected(
        portfolio::parse(&value, "portfolio.json"),
        ConfigErrorKind::UnknownField,
        "$.allow_short",
    );
}

#[test]
fn portfolio_rejects_a_missing_drawdown_limit() {
    assert_rejected(
        portfolio::parse(
            &set(portfolio_object(), &["max_drawdown", "limit"], json!(null)),
            "portfolio.json",
        ),
        Inconsistent,
        "$.max_drawdown.action",
    );
}

#[test]
fn model_v4_reads_events_and_progression() {
    let config = model::parse(&model_v4_object(), "model.json").expect("the v4 config loads");
    assert_eq!(config.max_timeframe_secs(), 3_600);
    assert!(config.is_overridden(Timeframe::M5));
    assert_eq!(
        config.events_for(300).signal.ema_pairs,
        [(9, 21), (21, 55), (9, 55)]
    );

    assert_rejected(
        model::parse(
            &set(
                model_v4_object(),
                &["defaults", "ema", "periods"],
                json!([20, 50]),
            ),
            "model.json",
        ),
        Range,
        "$.defaults.ema.periods",
    );
}

#[test]
fn model_v4_rejects_a_4h_override() {
    assert_rejected(
        model::parse(
            &set(
                model_v4_object(),
                &["timeframes"],
                json!({"4h": {"ema": {"periods": [9, 21, 55]}}}),
            ),
            "model.json",
        ),
        Range,
        "$.timeframes.4h",
    );
}

#[test]
fn model_v4_rejects_ema_pairs_outside_the_configured_triple() {
    assert_rejected(
        model::parse(
            &set(
                model_v4_object(),
                &["defaults", "events", "signal", "ema_pairs"],
                json!([[20, 50], [50, 200], [20, 300]]),
            ),
            "model.json",
        ),
        Range,
        "$.defaults.events.signal.ema_pairs[2]",
    );
}

#[test]
fn retired_model_schemas_are_rejected() {
    for schema in [
        "quantrule.model/v1",
        "quantrule.model/v2",
        "quantrule.model/v3",
    ] {
        assert_rejected(
            model::parse(&json!({"schema": schema}), "model.json"),
            Unsupported,
            "$.schema",
        );
    }
}

#[test]
fn run_parses_the_current_shape() {
    let parsed = run::parse(&run_object(), "run.json").expect("the run parses");
    assert_eq!(parsed.name, "run");
}

#[test]
fn run_rejects_the_removed_strategy_and_analytics_files() {
    for field in ["strategy", "analytics"] {
        let value = set(run_object(), &[field], json!("old.json"));
        assert_rejected(
            run::parse(&value, "run.json"),
            ConfigErrorKind::UnknownField,
            &format!("$.{field}"),
        );
    }
}

#[test]
fn run_rejects_the_removed_strategy_id() {
    let value = set(run_object(), &["strategy_id"], json!("S1.0"));
    assert_rejected(
        run::parse(&value, "run.json"),
        ConfigErrorKind::UnknownField,
        "$.strategy_id",
    );
}

#[test]
fn run_accepts_an_optional_model_ladder() {
    let value = set(run_object(), &["model_timeframes"], json!(["1m", "1h"]));
    let parsed = run::parse(&value, "run.json").expect("the ladder parses");
    assert_eq!(
        parsed.model_timeframes,
        Some(vec![Timeframe::M1, Timeframe::H1])
    );
}

#[test]
fn run_rejects_an_unsorted_model_ladder() {
    let value = set(run_object(), &["model_timeframes"], json!(["1h", "1m"]));
    assert_rejected(
        run::parse(&value, "run.json"),
        ConfigErrorKind::Range,
        "$.model_timeframes",
    );
}

#[test]
fn run_rejects_a_bad_identity_prefix() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/config-run/run.json");
    let mut config = run::RunConfig::load(&path).expect("the fixture config is valid");
    config.model_id = Some(String::from("S1.0"));
    assert_rejected(config.validate_identities(), Range, "$.model_id");
}

#[test]
fn run_rejects_an_identity_without_a_major_minor_shape() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/config-run/run.json");
    for identity in ["M", "Mfoo", "M1", "M1.", "M1.0.0", "M1.x"] {
        let mut config = run::RunConfig::load(&path).expect("the fixture config is valid");
        config.model_id = Some(identity.to_string());
        assert_rejected(config.validate_identities(), Range, "$.model_id");
    }
}

#[test]
fn the_example_config_loads_and_resolves() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../config/examples/btcusd-2024/run.json");
    let config = run::RunConfig::load(&path).expect("the example config is valid");

    assert_eq!(config.name, "btcusd-2024");
    assert_eq!(config.model_id.as_deref(), Some("M1.0"));
    assert_eq!(config.policy_id.as_deref(), Some("POL1.0"));
    assert_eq!(config.execution_id.as_deref(), Some("E1.0"));
    assert_eq!(config.portfolio_id.as_deref(), Some("P1.0"));
    assert_eq!(config.data.source_timeframe, Timeframe::M1);
    assert_eq!(config.data.available_timeframes, Timeframe::DEFAULT_LADDER);
}

#[test]
fn the_config_run_fixture_loads() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/config-run/run.json");
    let config = run::RunConfig::load(&path).expect("the fixture config is valid");
    assert_eq!(config.name, "fixture");
    assert_eq!(config.execution.fill_timeframe, Timeframe::M5);
}

#[test]
fn a_model_ladder_must_be_in_the_data_ladder_and_contain_execution() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/config-run/run.json");
    let mut config = run::RunConfig::load(&path).expect("the fixture config is valid");
    config.model_ladder = Some(vec![Timeframe::H4]);
    assert_rejected(config.validate_cross(), Inconsistent, "$.model_timeframes");
    config.model_ladder = Some(vec![Timeframe::M15]);
    assert_rejected(config.validate_cross(), Inconsistent, "$.model_timeframes");
    config.model_ladder = Some(vec![Timeframe::M5, Timeframe::H1]);
    assert!(config.validate_cross().is_ok());
}

#[test]
fn a_cross_config_mismatch_is_rejected() {
    let base = Path::new("config");
    let policy = policy::parse(&policy_object(), "policy.json").expect("policy");
    let mut data_value = data_object();
    data_value["available_timeframes"] = json!(["1m", "5m", "15m", "1h"]);
    let data = data::parse(&data_value, "data.json", base).expect("data");
    let model = model::parse(&model_v4_object(), "model.json").expect("model");
    let execution = execution::parse(&execution_object(), "execution.json").expect("execution");
    let portfolio = portfolio::parse(&portfolio_object(), "portfolio.json").expect("portfolio");

    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/config-run/run.json");
    let mut config = run::RunConfig::load(&path).expect("the fixture config is valid");
    config.policy = policy;
    config.data = data;
    config.model = model;
    config.execution = execution;
    config.portfolio = portfolio;

    assert!(config.validate_cross().is_ok());

    config.data.available_timeframes.push(Timeframe::H4);
    assert_rejected(config.validate_cross(), Range, "$.available_timeframes");
    config.data.available_timeframes.pop();

    config.policy.universe.timeframes = vec![Timeframe::M15, Timeframe::H4];
    assert_rejected(
        config.validate_cross(),
        Inconsistent,
        "$.universe.timeframes",
    );
}
