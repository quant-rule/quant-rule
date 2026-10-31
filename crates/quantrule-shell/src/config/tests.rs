//! Tests for the config loader.
//!
//! Each table row is one rule from the config contract. The row asserts the
//! error category and the JSON path, not only that the file was rejected, so a
//! rule cannot regress into a different rejection unnoticed. The example
//! configs are loaded from the repository; the invalid-JSON and missing-file
//! fixtures live under `tests/fixtures/config/`.

use std::path::{Path, PathBuf};

use quantrule_backtest::portfolio::{DrawdownAction, SizingMode};
use quantrule_data::Timeframe;
use quantrule_model::readings::PivotScale;
use serde_json::{Value, json};

use super::error::ConfigErrorKind;
use super::error::ConfigErrorKind::{
    Inconsistent, MissingField, Range, Schema, Type, UnknownField, Unsupported,
};
use super::{ConfigError, analytics, data, execution, model, policy, portfolio, run, strategy};

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

    let subset = set(
        data_object(),
        &["available_timeframes"],
        json!(["1m", "15m", "4h"]),
    );
    let subset = data::parse(&subset, "data.json", base).expect("a supported subset is valid");
    assert_eq!(
        subset.available_timeframes,
        vec![Timeframe::M1, Timeframe::M15, Timeframe::H4]
    );

    let open = set(
        set(data_object(), &["from"], json!(null)),
        &["to"],
        json!(null),
    );
    assert!(
        data::parse(&open, "data.json", base).is_ok(),
        "a null bound is accepted"
    );

    let absolute = set(
        data_object(),
        &["source", "path"],
        json!("/var/data/candles.csv"),
    );
    let absolute = data::parse(&absolute, "data.json", base).expect("an absolute path is accepted");
    assert_eq!(absolute.source.path, Path::new("/var/data/candles.csv"));

    let tilde = set(data_object(), &["source", "path"], json!("~/candles.csv"));
    let tilde = data::parse(&tilde, "data.json", base).expect("a tilde is a literal path");
    assert_eq!(tilde.source.path, Path::new("config/~/candles.csv"));

    let environment = set(
        data_object(),
        &["source", "path"],
        json!("$HOME/candles.csv"),
    );
    let environment =
        data::parse(&environment, "data.json", base).expect("a variable is a literal path");
    assert_eq!(
        environment.source.path,
        Path::new("config/$HOME/candles.csv")
    );

    let one = set(data_object(), &["max_candles"], json!(1));
    assert!(
        data::parse(&one, "data.json", base).is_ok(),
        "the smallest bound is accepted"
    );

    let cases = vec![
        ("top level", json!([]), Type, "$"),
        (
            "schema",
            set(data_object(), &["schema"], json!("quantrule.data/v2")),
            Schema,
            "$.schema",
        ),
        (
            "unknown field",
            set(data_object(), &["extra"], json!(true)),
            UnknownField,
            "$.extra",
        ),
        (
            "missing source",
            remove(data_object(), &["source"]),
            MissingField,
            "$.source",
        ),
        (
            "source type",
            set(data_object(), &["source"], json!("csv")),
            Type,
            "$.source",
        ),
        (
            "source kind",
            set(data_object(), &["source", "kind"], json!("dukascopy")),
            Unsupported,
            "$.source.kind",
        ),
        (
            "source unknown",
            set(data_object(), &["source", "extra"], json!(1)),
            UnknownField,
            "$.source.extra",
        ),
        (
            "source path",
            set(data_object(), &["source", "path"], json!("")),
            Range,
            "$.source.path",
        ),
        (
            "data version",
            set(data_object(), &["data_version"], json!("")),
            Range,
            "$.data_version",
        ),
        (
            "from type",
            set(data_object(), &["from"], json!(20250101)),
            Type,
            "$.from",
        ),
        (
            "from date",
            set(data_object(), &["from"], json!("2025-13-01")),
            Range,
            "$.from",
        ),
        (
            "to date",
            set(data_object(), &["to"], json!("nope")),
            Range,
            "$.to",
        ),
        (
            "reversed range",
            set(
                set(data_object(), &["from"], json!("2025-12-31")),
                &["to"],
                json!("2025-01-01"),
            ),
            Range,
            "$.from",
        ),
        (
            "source timeframe",
            set(data_object(), &["source_timeframe"], json!("5m")),
            Range,
            "$.source_timeframe",
        ),
        (
            "timeframe case",
            set(data_object(), &["source_timeframe"], json!("1M")),
            Range,
            "$.source_timeframe",
        ),
        (
            "available empty",
            set(data_object(), &["available_timeframes"], json!([])),
            Range,
            "$.available_timeframes",
        ),
        (
            "available missing m1",
            set(
                data_object(),
                &["available_timeframes"],
                json!(["5m", "15m"]),
            ),
            Range,
            "$.available_timeframes[0]",
        ),
        (
            "available duplicate",
            set(
                data_object(),
                &["available_timeframes"],
                json!(["1m", "5m", "5m"]),
            ),
            Range,
            "$.available_timeframes",
        ),
        (
            "available order",
            set(
                data_object(),
                &["available_timeframes"],
                json!(["1h", "15m", "5m", "1m"]),
            ),
            Range,
            "$.available_timeframes",
        ),
        (
            "available element",
            set(
                data_object(),
                &["available_timeframes"],
                json!(["1m", 5, "15m"]),
            ),
            Type,
            "$.available_timeframes[1]",
        ),
        (
            "max candles zero",
            set(data_object(), &["max_candles"], json!(0)),
            Range,
            "$.max_candles",
        ),
        (
            "max candles large",
            set(data_object(), &["max_candles"], json!(600001)),
            Range,
            "$.max_candles",
        ),
        (
            "max candles type",
            set(data_object(), &["max_candles"], json!("600000")),
            Type,
            "$.max_candles",
        ),
    ];
    for (_, value, kind, path) in cases {
        assert_rejected(data::parse(&value, "data.json", base), kind, path);
    }
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
            "tickets": ["EURUSD", "SOLUSD"],
            "timeframes": ["15m"]
        }
    })
}

#[test]
fn policy_matches_the_contract() {
    let valid = policy::parse(&policy_object(), "policy.json").expect("the valid policy loads");
    assert_eq!(valid.account.initial_cash, 10_000.0);
    assert_eq!(valid.objective.evaluation_days, 90);
    assert_eq!(valid.universe.tickets, ["EURUSD", "SOLUSD"]);

    for (value, kind, path) in [
        (json!(1), Type, "$"),
        (
            set(policy_object(), &["schema"], json!("quantrule.policy/v2")),
            Schema,
            "$.schema",
        ),
        (
            set(policy_object(), &["account", "currency"], json!("EUR")),
            Unsupported,
            "$.account.currency",
        ),
        (
            set(policy_object(), &["risk", "max_leverage"], json!(2.0)),
            Unsupported,
            "$.risk.max_leverage",
        ),
        (
            set(policy_object(), &["universe", "tickets"], json!([])),
            Range,
            "$.universe.tickets",
        ),
        (
            set(
                policy_object(),
                &["universe", "timeframes"],
                json!(["15m", "5m"]),
            ),
            Range,
            "$.universe.timeframes[1]",
        ),
    ] {
        assert_rejected(policy::parse(&value, "policy.json"), kind, path);
    }
}

fn model_object() -> Value {
    json!({"schema": "quantrule.model/v1", "structure_scale": "major"})
}

fn model_v2_object() -> Value {
    json!({
        "schema": "quantrule.model/v2",
        "defaults": {
            "ema": [20, 50, 200],
            "rsi": 14,
            "atr": 14,
            "rvol": 20
        },
        "timeframes": {},
        "structure": {"scale": "major"}
    })
}

#[test]
fn model_v2_reads_defaults_and_timeframe_overrides() {
    let defaults = model::parse(&model_v2_object(), "model.json").expect("the v2 config loads");
    assert_eq!(defaults.structure_scale, PivotScale::Major);
    let m1 = defaults.readings_for(u64::from(Timeframe::M1.seconds()));
    assert_eq!(m1, quantrule_model::readings::ReadingsConfig::default());

    let overridden = set(
        model_v2_object(),
        &["timeframes"],
        json!({"1m": {"ema": [10, 30, 90], "rsi": 21}}),
    );
    let config = model::parse(&overridden, "model.json").expect("the override loads");
    assert!(config.is_overridden(Timeframe::M1));
    assert!(!config.is_overridden(Timeframe::M5));
    let m1 = config.readings_for(u64::from(Timeframe::M1.seconds()));
    assert_eq!(m1.ema_fast, 10);
    assert_eq!(m1.ema_mid, 30);
    assert_eq!(m1.ema_slow, 90);
    assert_eq!(m1.rsi, 21);
    assert_eq!(m1.atr, 14);
    assert_eq!(m1.rvol, 20);
    let m5 = config.readings_for(u64::from(Timeframe::M5.seconds()));
    assert_eq!(m5, quantrule_model::readings::ReadingsConfig::default());
}

#[test]
fn model_v2_timeframes_map_is_optional() {
    let without_map = remove(model_v2_object(), &["timeframes"]);
    let config =
        model::parse(&without_map, "model.json").expect("an absent timeframes map is valid");
    assert_eq!(
        config.readings_for(u64::from(Timeframe::M1.seconds())),
        quantrule_model::readings::ReadingsConfig::default()
    );
}

#[test]
fn model_matches_the_contract() {
    let valid = model::parse(&model_object(), "model.json").expect("the valid config loads");
    assert_eq!(valid.structure_scale, PivotScale::Major);
    assert!(
        model::parse(
            &set(model_object(), &["structure_scale"], json!("minor")),
            "model.json"
        )
        .is_ok()
    );

    for (value, kind, path) in [
        (json!(1), Type, "$"),
        (
            set(model_object(), &["schema"], json!("quantrule.model/v3")),
            Unsupported,
            "$.schema",
        ),
        (
            set(model_object(), &["extra"], json!(1)),
            UnknownField,
            "$.extra",
        ),
        (
            remove(model_object(), &["structure_scale"]),
            MissingField,
            "$.structure_scale",
        ),
        (
            set(model_object(), &["structure_scale"], json!("weekly")),
            Range,
            "$.structure_scale",
        ),
        (
            set(model_object(), &["structure_scale"], json!("Major")),
            Range,
            "$.structure_scale",
        ),
        (
            set(model_object(), &["structure_scale"], json!(2)),
            Type,
            "$.structure_scale",
        ),
    ] {
        assert_rejected(model::parse(&value, "model.json"), kind, path);
    }
}

#[test]
fn model_v2_rejects_invalid_readings() {
    for (value, kind, path) in [
        (
            remove(model_v2_object(), &["defaults"]),
            MissingField,
            "$.defaults",
        ),
        (
            remove(model_v2_object(), &["defaults", "rsi"]),
            MissingField,
            "$.defaults.rsi",
        ),
        (
            set(model_v2_object(), &["defaults", "ema"], json!([20, 50])),
            Range,
            "$.defaults.ema",
        ),
        (
            set(
                model_v2_object(),
                &["defaults", "ema"],
                json!([50, 20, 200]),
            ),
            Range,
            "$.defaults",
        ),
        (
            set(model_v2_object(), &["defaults", "rsi"], json!(0)),
            Range,
            "$.defaults.rsi",
        ),
        (
            set(
                model_v2_object(),
                &["timeframes"],
                json!({"30m": {"rsi": 21}}),
            ),
            Range,
            "$.timeframes.30m",
        ),
        (
            set(
                model_v2_object(),
                &["timeframes"],
                json!({"1m": {"unknown": 1}}),
            ),
            UnknownField,
            "$.timeframes.1m.unknown",
        ),
        (
            remove(model_v2_object(), &["structure"]),
            MissingField,
            "$.structure",
        ),
        (
            set(model_v2_object(), &["structure", "scale"], json!("weekly")),
            Range,
            "$.structure.scale",
        ),
    ] {
        assert_rejected(model::parse(&value, "model.json"), kind, path);
    }
}

fn strategy_object() -> Value {
    json!({
        "schema": "quantrule.strategy/v3",
        "rule": "ladder-context",
        "thesis": "Trade with the higher-timeframe regime after a lower-timeframe structure shift.",
        "ladder": ["5m", "15m"],
        "weights": {
            "direction": {"15m": 1.0},
            "momentum": {"15m": 1.0},
            "structure": {"15m": 1.0},
            "progression": {"15m": 1.0}
        },
        "entry": {
            "execution": {
                "direction_min": 0.1,
                "momentum_min": 0.1,
                "structure_min": 0.0,
                "structure_shift": {"from_max": 0, "to_min": 1}
            },
            "context": {
                "direction": {"kind": "bullish", "share_min": 0.5},
                "momentum": {"kind": "not_bearish", "bearish_share_max": 0.5},
                "structure": {"conflict_count_max": 0},
                "progression": {
                    "direction": "bullish",
                    "late_or_terminal_count_max": 0
                }
            }
        },
        "exit": {"on": ["context_lost", "structure_lost"]},
        "risk": {
            "stop": {"kind": "atr", "multiple": 2.0},
            "target": {"kind": "reward_to_risk", "multiple": 1.5}
        },
        "no_trade": {
            "missing_context": "reject",
            "require_contiguous_execution": true
        }
    })
}

#[test]
fn strategy_matches_the_contract() {
    let valid =
        strategy::parse(&strategy_object(), "strategy.json").expect("the valid config loads");
    assert_eq!(valid.engine.timeframe_secs(), 300);
    assert_eq!(valid.engine.ladder().context_timeframes(), &[900]);
    assert_eq!(valid.engine.weights().direction(), &[1.0]);
    assert_eq!(valid.engine.entry().execution().direction_min(), 0.1);
    assert_eq!(valid.engine.risk().stop_atr_multiple(), 2.0);
    assert_eq!(valid.engine.risk().reward_to_risk(), 1.5);
    assert_eq!(valid.stop_atr_multiple(), 2.0);
    assert_eq!(valid.reward_to_risk(), 1.5);

    for (value, kind, path) in [
        (json!(1), Type, "$"),
        (
            set(
                strategy_object(),
                &["schema"],
                json!("quantrule.strategy/v2"),
            ),
            Range,
            "$.schema",
        ),
        (
            set(
                strategy_object(),
                &["schema"],
                json!("quantrule.strategy/v4"),
            ),
            Schema,
            "$.schema",
        ),
        (
            set(strategy_object(), &["extra"], json!(1)),
            UnknownField,
            "$.extra",
        ),
        (
            set(strategy_object(), &["rule"], json!("axis-agreement")),
            Unsupported,
            "$.rule",
        ),
        (
            remove(strategy_object(), &["thesis"]),
            MissingField,
            "$.thesis",
        ),
        (
            set(strategy_object(), &["thesis"], json!("")),
            Range,
            "$.thesis",
        ),
        (
            remove(strategy_object(), &["ladder"]),
            MissingField,
            "$.ladder",
        ),
        (
            set(strategy_object(), &["ladder"], json!(["5m"])),
            Range,
            "$.ladder",
        ),
        (
            set(strategy_object(), &["ladder"], json!(["5m", "30m"])),
            Range,
            "$.ladder[1]",
        ),
        (
            set(strategy_object(), &["ladder"], json!(["15m", "5m"])),
            Range,
            "$.ladder",
        ),
        (
            set(strategy_object(), &["ladder"], json!(["5m", "5m"])),
            Range,
            "$.ladder",
        ),
        (
            set(
                strategy_object(),
                &["weights", "other"],
                json!({"15m": 1.0}),
            ),
            UnknownField,
            "$.weights.other",
        ),
        (
            set(
                strategy_object(),
                &["weights", "direction", "1h"],
                json!(1.0),
            ),
            UnknownField,
            "$.weights.direction.1h",
        ),
        (
            remove(strategy_object(), &["weights", "direction", "15m"]),
            MissingField,
            "$.weights.direction.15m",
        ),
        (
            set(
                strategy_object(),
                &["weights", "direction", "15m"],
                json!(0.0),
            ),
            Range,
            "$.weights.direction.15m",
        ),
        (
            remove(strategy_object(), &["entry", "execution"]),
            MissingField,
            "$.entry.execution",
        ),
        (
            set(
                strategy_object(),
                &["entry", "execution", "direction_min"],
                json!(1.1),
            ),
            Range,
            "$.entry.execution",
        ),
        (
            set(
                strategy_object(),
                &["entry", "execution", "structure_shift", "from_max"],
                json!(1),
            ),
            Range,
            "$.entry.execution",
        ),
        (
            set(
                strategy_object(),
                &["entry", "context", "direction", "kind"],
                json!("bearish"),
            ),
            Unsupported,
            "$.entry.context.direction.kind",
        ),
        (
            set(
                strategy_object(),
                &["entry", "context", "direction", "share_min"],
                json!(1.1),
            ),
            Range,
            "$.entry.context",
        ),
        (
            set(strategy_object(), &["exit", "on"], json!([])),
            Range,
            "$.exit.on",
        ),
        (
            set(
                strategy_object(),
                &["exit", "on"],
                json!(["context_lost", "context_lost"]),
            ),
            Inconsistent,
            "$.exit.on[1]",
        ),
        (
            set(strategy_object(), &["exit", "on"], json!(["unknown"])),
            Unsupported,
            "$.exit.on[0]",
        ),
        (
            set(strategy_object(), &["risk", "stop", "kind"], json!("price")),
            Unsupported,
            "$.risk.stop.kind",
        ),
        (
            set(
                strategy_object(),
                &["risk", "target", "multiple"],
                json!(0.0),
            ),
            Range,
            "$.risk",
        ),
        (
            remove(strategy_object(), &["no_trade", "missing_context"]),
            MissingField,
            "$.no_trade.missing_context",
        ),
        (
            set(
                strategy_object(),
                &["no_trade", "missing_context"],
                json!("skip"),
            ),
            Unsupported,
            "$.no_trade.missing_context",
        ),
        (
            set(
                strategy_object(),
                &["no_trade", "require_contiguous_execution"],
                json!(false),
            ),
            Range,
            "$.no_trade.require_contiguous_execution",
        ),
    ] {
        assert_rejected(strategy::parse(&value, "strategy.json"), kind, path);
    }
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

#[test]
fn strategy_v2_is_rejected_with_a_migration_message() {
    let value = json!({
        "schema": "quantrule.strategy/v2",
        "name": "axis-agreement",
        "thesis": "Trade only when direction, momentum, and structure agree; exit when they disagree.",
        "timeframe": "15m",
        "long_threshold": 0.1,
        "short_threshold": -0.1,
        "stop_atr_multiple": 2.0,
        "reward_to_risk": 1.5
    });
    let error =
        strategy::parse(&value, "strategy.json").expect_err("the v2 schema must be rejected");
    assert!(
        error
            .to_string()
            .contains("quantrule.strategy/v2 is retired; use quantrule.strategy/v3 with rule \"ladder-context\""),
        "the rejection must explain the migration: {error}"
    );
}

#[test]
fn execution_matches_the_contract() {
    let valid =
        execution::parse(&execution_object(), "execution.json").expect("the valid config loads");
    assert_eq!(valid.fill_timeframe, Timeframe::M15);
    assert_eq!(valid.engine.fee_rate(), 0.0);
    assert_eq!(valid.engine.slippage_bps(), 2.0);

    for (value, kind, path) in [
        (json!(1), Type, "$"),
        (
            set(
                execution_object(),
                &["schema"],
                json!("quantrule.execution/v3"),
            ),
            Schema,
            "$.schema",
        ),
        (
            set(execution_object(), &["extra"], json!(1)),
            UnknownField,
            "$.extra",
        ),
        (
            remove(execution_object(), &["fee_rate"]),
            MissingField,
            "$.fee_rate",
        ),
        (
            set(execution_object(), &["fill_timeframe"], json!("30m")),
            Range,
            "$.fill_timeframe",
        ),
        (
            set(execution_object(), &["order_type"], json!("limit")),
            Unsupported,
            "$.order_type",
        ),
        (
            set(execution_object(), &["tie_break"], json!("target_first")),
            Unsupported,
            "$.tie_break",
        ),
        (
            set(execution_object(), &["fee_rate"], json!(-0.001)),
            Range,
            "$.fee_rate",
        ),
        (
            set(execution_object(), &["slippage_bps"], json!(10_000.0)),
            Range,
            "$.slippage_bps",
        ),
        (
            set(execution_object(), &["slippage_bps"], json!(-1.0)),
            Range,
            "$.slippage_bps",
        ),
        (
            set(execution_object(), &["zero_volume"], json!("skip")),
            Unsupported,
            "$.zero_volume",
        ),
        (
            set(execution_object(), &["end_of_data"], json!("close")),
            Unsupported,
            "$.end_of_data",
        ),
    ] {
        assert_rejected(execution::parse(&value, "execution.json"), kind, path);
    }
}

fn portfolio_object() -> Value {
    json!({
        "schema": "quantrule.portfolio/v1",
        "initial_cash": 10000.0,
        "currency": "USD",
        "sizing": {
            "mode": "fixed_notional",
            "notional": 10000.0,
            "quantity_step": 0.0001
        },
        "leverage": 1.0,
        "max_open_positions": 1,
        "allow_short": true,
        "commission": {"rate": 0.001, "base": "equity", "per": "operation"},
        "funding": {"rate": 0.0001, "interval_hours": 8},
        "max_drawdown": {"limit": 0.2, "action": "halt"},
        "mark_to_market": "bar_close"
    })
}

#[test]
fn portfolio_matches_the_contract() {
    let valid =
        portfolio::parse(&portfolio_object(), "portfolio.json").expect("the valid config loads");
    assert_eq!(valid.initial_cash, 10_000.0);
    assert_eq!(
        valid.sizing,
        SizingMode::FixedNotional {
            notional: 10_000.0,
            quantity_step: 0.0001,
        }
    );
    assert_eq!(valid.leverage, 1.0);
    assert_eq!(valid.max_open_positions, 1);
    assert!(valid.allow_short);
    assert_eq!(valid.commission.rate(), 0.001);
    assert_eq!(valid.funding.rate(), 0.0001);
    assert_eq!(valid.funding.interval().hours(), 8);
    assert_eq!(valid.max_drawdown.limit(), Some(0.2));
    assert_eq!(valid.max_drawdown.action(), DrawdownAction::Halt);

    let fraction = set(
        portfolio_object(),
        &["sizing"],
        json!({"mode": "equity_fraction", "fraction": 0.25, "quantity_step": 0.0001}),
    );
    assert!(
        portfolio::parse(&fraction, "portfolio.json").is_ok(),
        "the equity-fraction mode is valid"
    );

    let no_limit = set(
        set(portfolio_object(), &["max_drawdown", "limit"], json!(null)),
        &["max_drawdown", "action"],
        json!("none"),
    );
    assert!(
        portfolio::parse(&no_limit, "portfolio.json").is_ok(),
        "a null limit with no action is valid"
    );

    let measured = set(
        portfolio_object(),
        &["max_drawdown", "action"],
        json!("none"),
    );
    assert!(
        portfolio::parse(&measured, "portfolio.json").is_ok(),
        "a measured limit with no action is valid"
    );

    let receiving = set(portfolio_object(), &["funding", "rate"], json!(-0.0001));
    assert!(
        portfolio::parse(&receiving, "portfolio.json").is_ok(),
        "a negative funding rate reverses the payer"
    );

    let long_only = set(portfolio_object(), &["allow_short"], json!(false));
    assert!(
        portfolio::parse(&long_only, "portfolio.json").is_ok(),
        "long-only is valid"
    );

    let cases = vec![
        ("top level", json!([]), Type, "$"),
        (
            "schema",
            set(
                portfolio_object(),
                &["schema"],
                json!("quantrule.portfolio/v2"),
            ),
            Schema,
            "$.schema",
        ),
        (
            "unknown field",
            set(portfolio_object(), &["extra"], json!(1)),
            UnknownField,
            "$.extra",
        ),
        (
            "initial cash",
            set(portfolio_object(), &["initial_cash"], json!(0.0)),
            Range,
            "$.initial_cash",
        ),
        (
            "initial cash type",
            set(portfolio_object(), &["initial_cash"], json!("10000")),
            Type,
            "$.initial_cash",
        ),
        (
            "currency",
            set(portfolio_object(), &["currency"], json!("EUR")),
            Unsupported,
            "$.currency",
        ),
        (
            "sizing type",
            set(portfolio_object(), &["sizing"], json!([])),
            Type,
            "$.sizing",
        ),
        (
            "sizing mode",
            set(portfolio_object(), &["sizing", "mode"], json!("percent")),
            Unsupported,
            "$.sizing.mode",
        ),
        (
            "sizing unknown",
            set(portfolio_object(), &["sizing", "fraction"], json!(0.1)),
            UnknownField,
            "$.sizing.fraction",
        ),
        (
            "notional",
            set(portfolio_object(), &["sizing", "notional"], json!(0.0)),
            Range,
            "$.sizing.notional",
        ),
        (
            "step",
            set(portfolio_object(), &["sizing", "quantity_step"], json!(0.0)),
            Range,
            "$.sizing.quantity_step",
        ),
        (
            "fraction low",
            set(
                portfolio_object(),
                &["sizing"],
                json!({"mode": "equity_fraction", "fraction": 0.0, "quantity_step": 0.0001}),
            ),
            Range,
            "$.sizing.fraction",
        ),
        (
            "fraction high",
            set(
                portfolio_object(),
                &["sizing"],
                json!({"mode": "equity_fraction", "fraction": 1.5, "quantity_step": 0.0001}),
            ),
            Range,
            "$.sizing.fraction",
        ),
        (
            "equity unknown",
            set(
                portfolio_object(),
                &["sizing"],
                json!({"mode": "equity_fraction", "notional": 1.0, "quantity_step": 0.0001}),
            ),
            UnknownField,
            "$.sizing.notional",
        ),
        (
            "leverage",
            set(portfolio_object(), &["leverage"], json!(2.0)),
            Unsupported,
            "$.leverage",
        ),
        (
            "position limit",
            set(portfolio_object(), &["max_open_positions"], json!(2)),
            Unsupported,
            "$.max_open_positions",
        ),
        (
            "position type",
            set(portfolio_object(), &["max_open_positions"], json!(1.0)),
            Type,
            "$.max_open_positions",
        ),
        (
            "allow short",
            set(portfolio_object(), &["allow_short"], json!("true")),
            Type,
            "$.allow_short",
        ),
        (
            "commission unknown",
            set(portfolio_object(), &["commission", "extra"], json!(1)),
            UnknownField,
            "$.commission.extra",
        ),
        (
            "commission rate",
            set(portfolio_object(), &["commission", "rate"], json!(-1.0)),
            Range,
            "$.commission.rate",
        ),
        (
            "commission base",
            set(
                portfolio_object(),
                &["commission", "base"],
                json!("notional"),
            ),
            Unsupported,
            "$.commission.base",
        ),
        (
            "commission per",
            set(portfolio_object(), &["commission", "per"], json!("bar")),
            Unsupported,
            "$.commission.per",
        ),
        (
            "funding unknown",
            set(portfolio_object(), &["funding", "extra"], json!(1)),
            UnknownField,
            "$.funding.extra",
        ),
        (
            "funding interval",
            set(portfolio_object(), &["funding", "interval_hours"], json!(3)),
            Range,
            "$.funding.interval_hours",
        ),
        (
            "funding rate",
            set(portfolio_object(), &["funding", "rate"], json!("0.0001")),
            Type,
            "$.funding.rate",
        ),
        (
            "drawdown low",
            set(portfolio_object(), &["max_drawdown", "limit"], json!(0.0)),
            Range,
            "$.max_drawdown.limit",
        ),
        (
            "drawdown high",
            set(portfolio_object(), &["max_drawdown", "limit"], json!(1.5)),
            Range,
            "$.max_drawdown.limit",
        ),
        (
            "drawdown action",
            set(
                portfolio_object(),
                &["max_drawdown", "action"],
                json!("liquidate"),
            ),
            Unsupported,
            "$.max_drawdown.action",
        ),
        (
            "drawdown without limit",
            set(portfolio_object(), &["max_drawdown", "limit"], json!(null)),
            Inconsistent,
            "$.max_drawdown.action",
        ),
        (
            "mark to market",
            set(portfolio_object(), &["mark_to_market"], json!("bar_open")),
            Unsupported,
            "$.mark_to_market",
        ),
        (
            "missing cash",
            remove(portfolio_object(), &["initial_cash"]),
            MissingField,
            "$.initial_cash",
        ),
    ];
    for (_, value, kind, path) in cases {
        assert_rejected(portfolio::parse(&value, "portfolio.json"), kind, path);
    }
}

fn analytics_object() -> Value {
    json!({
        "schema": "quantrule.analytics/v1",
        "structure_scale": "major",
        "timeframes": ["1m", "5m", "15m", "1h"],
        "relations": true
    })
}

#[test]
fn analytics_matches_the_contract() {
    let valid =
        analytics::parse(&analytics_object(), "analytics.json").expect("the valid config loads");
    assert_eq!(valid.structure_scale, PivotScale::Major);
    assert_eq!(valid.timeframes, Timeframe::DEFAULT_LADDER);
    assert!(valid.relations);

    let subset = set(analytics_object(), &["timeframes"], json!(["1m", "15m"]));
    let subset = analytics::parse(&subset, "analytics.json").expect("a subset is valid");
    assert_eq!(subset.timeframes, vec![Timeframe::M1, Timeframe::M15]);

    let no_relations = set(analytics_object(), &["relations"], json!(false));
    let no_relations =
        analytics::parse(&no_relations, "analytics.json").expect("relations may be false");
    assert!(!no_relations.relations);

    for (value, kind, path) in [
        (json!(1), Type, "$"),
        (
            set(
                analytics_object(),
                &["schema"],
                json!("quantrule.analytics/v2"),
            ),
            Schema,
            "$.schema",
        ),
        (
            set(analytics_object(), &["extra"], json!(1)),
            UnknownField,
            "$.extra",
        ),
        (
            set(analytics_object(), &["structure_scale"], json!("weekly")),
            Range,
            "$.structure_scale",
        ),
        (
            set(analytics_object(), &["timeframes"], json!([])),
            Range,
            "$.timeframes",
        ),
        (
            set(analytics_object(), &["timeframes"], json!(["1m", "1m"])),
            Range,
            "$.timeframes[1]",
        ),
        (
            set(analytics_object(), &["timeframes"], json!(["5m", "1m"])),
            Range,
            "$.timeframes[1]",
        ),
        (
            set(analytics_object(), &["timeframes"], json!(["1m", "30m"])),
            Range,
            "$.timeframes[1]",
        ),
        (
            set(analytics_object(), &["relations"], json!("true")),
            Type,
            "$.relations",
        ),
    ] {
        assert_rejected(analytics::parse(&value, "analytics.json"), kind, path);
    }
}

fn run_object() -> Value {
    json!({
        "schema": "quantrule.run/v2",
        "name": "run",
        "policy": "policy.json",
        "data": "data.json",
        "model": "model.json",
        "strategy": "strategy.json",
        "execution": "execution.json",
        "portfolio": "portfolio.json",
        "analytics": "analytics.json",
        "output_root": "out"
    })
}

#[test]
fn run_matches_the_contract() {
    let valid = run::parse(&run_object(), "run.json").expect("the valid config loads");
    assert_eq!(valid.name, "run");
    assert_eq!(valid.output_root, "out");

    for (value, kind, path) in [
        (json!(1), Type, "$"),
        (
            set(run_object(), &["schema"], json!("quantrule.run/v3")),
            Schema,
            "$.schema",
        ),
        (
            set(run_object(), &["extra"], json!(1)),
            UnknownField,
            "$.extra",
        ),
        (set(run_object(), &["name"], json!("")), Range, "$.name"),
        (set(run_object(), &["name"], json!(1)), Type, "$.name"),
        (set(run_object(), &["data"], json!("")), Range, "$.data"),
        (
            remove(run_object(), &["output_root"]),
            MissingField,
            "$.output_root",
        ),
        (
            set(run_object(), &["output_root"], json!("")),
            Range,
            "$.output_root",
        ),
    ] {
        assert_rejected(run::parse(&value, "run.json"), kind, path);
    }
}

fn example_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../config/examples/solusd-2025/run.json")
}

fn fixtures_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/config")
}

#[test]
fn the_example_config_loads_and_resolves() {
    let config = run::RunConfig::load(&example_path()).expect("the example config is valid");

    assert_eq!(config.name, "solusd-2025");
    assert_eq!(
        config.data.data_version,
        "dukascopy-bid-solusd-m1-2025-01-01_2025-12-31"
    );
    assert_eq!(config.data.from.as_deref(), Some("2025-01-01"));
    assert_eq!(config.data.to.as_deref(), Some("2025-12-31"));
    assert_eq!(config.data.source_timeframe, Timeframe::M1);
    assert_eq!(config.data.available_timeframes, Timeframe::DEFAULT_LADDER);
    assert_eq!(config.data.max_candles, 600_000);
    assert!(
        config
            .data
            .source
            .path
            .ends_with("data/sources/SOLUSD_M1_2025-01-01_2025-12-31.csv")
    );
    assert!(
        config
            .paths
            .data
            .ends_with("config/examples/solusd-2025/data.json")
    );
    assert!(
        config
            .output_root
            .ends_with("data/labs/backtest-solusd-2025")
    );

    assert_eq!(config.model.structure_scale, PivotScale::Major);
    assert_eq!(config.strategy.engine.timeframe_secs(), 300);
    assert_eq!(config.strategy.engine.ladder().context_timeframes(), &[900]);
    assert_eq!(
        config.strategy.engine.entry().execution().direction_min(),
        0.1
    );
    assert_eq!(config.strategy.stop_atr_multiple(), 2.0);
    assert_eq!(config.strategy.reward_to_risk(), 1.5);
    assert_eq!(config.strategy.engine.risk().stop_atr_multiple(), 2.0);

    assert_eq!(config.execution.fill_timeframe, Timeframe::M5);
    assert_eq!(config.execution.engine.fee_rate(), 0.0);
    assert_eq!(config.execution.engine.slippage_bps(), 2.0);

    assert_eq!(config.portfolio.initial_cash, 10_000.0);
    assert_eq!(
        config.portfolio.sizing,
        SizingMode::FixedNotional {
            notional: 10_000.0,
            quantity_step: 0.0001,
        }
    );
    assert_eq!(config.portfolio.leverage, 1.0);
    assert_eq!(config.portfolio.max_open_positions, 1);
    assert!(config.portfolio.allow_short);
    assert_eq!(config.portfolio.commission.rate(), 0.001);
    assert_eq!(config.portfolio.funding.rate(), 0.0001);
    assert_eq!(config.portfolio.funding.interval().hours(), 8);
    assert_eq!(config.portfolio.max_drawdown.limit(), Some(0.2));
    assert_eq!(config.portfolio.max_drawdown.action(), DrawdownAction::Halt);

    assert_eq!(config.analytics.structure_scale, PivotScale::Major);
    assert_eq!(config.analytics.timeframes, Timeframe::DEFAULT_LADDER);
    assert!(config.analytics.relations);
}

fn assert_cross(config: &run::RunConfig, kind: ConfigErrorKind, path: &str, file: &str) {
    let error = config
        .validate_cross()
        .expect_err("the cross-config check must fail");
    assert_eq!(error.kind(), kind, "unexpected category: {error}");
    assert_eq!(error.path(), path, "unexpected path: {error}");
    assert!(
        error.to_string().contains(file),
        "the error must name {file}: {error}"
    );
}

#[test]
fn cross_config_invariants_are_enforced() {
    let base = run::RunConfig::load(&example_path()).expect("the example config is valid");

    let mut missing_strategy_timeframe = base.clone();
    missing_strategy_timeframe.data.available_timeframes = vec![Timeframe::M1, Timeframe::M5];
    assert_cross(
        &missing_strategy_timeframe,
        Inconsistent,
        "$.ladder",
        "strategy.json",
    );

    let mut wrong_fill_timeframe = base.clone();
    wrong_fill_timeframe.execution.fill_timeframe = Timeframe::M15;
    assert_cross(
        &wrong_fill_timeframe,
        Inconsistent,
        "$.fill_timeframe",
        "execution.json",
    );

    let mut wrong_scale = base.clone();
    wrong_scale.analytics.structure_scale = PivotScale::Minor;
    assert_cross(
        &wrong_scale,
        Inconsistent,
        "$.structure_scale",
        "analytics.json",
    );

    let mut missing_timeframe = base.clone();
    missing_timeframe.data.available_timeframes =
        vec![Timeframe::M1, Timeframe::M5, Timeframe::M15];
    missing_timeframe.analytics.timeframes = vec![Timeframe::H1];
    assert_cross(
        &missing_timeframe,
        Inconsistent,
        "$.timeframes",
        "analytics.json",
    );
}

#[test]
fn the_config_fingerprint_is_stable_and_visible_in_provenance() {
    let first = run::RunConfig::load(&example_path()).expect("the example loads");
    let second = run::RunConfig::load(&example_path()).expect("the example loads again");
    assert_eq!(first.config_hash, second.config_hash);
    assert_eq!(first.config_hash.len(), 16);
    assert!(first.config_hash.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn a_missing_file_is_reported_without_an_absolute_path() {
    let error = run::RunConfig::load(&fixtures_path().join("run-missing.json"))
        .expect_err("the referenced file is missing");
    assert_eq!(error.kind(), ConfigErrorKind::MissingFile);
    let message = error.to_string();
    assert!(message.contains("absent.json"), "{message}");
    assert!(
        !message.contains(env!("CARGO_MANIFEST_DIR")),
        "the message must not print an absolute path: {message}"
    );
}

#[test]
fn invalid_json_is_reported() {
    let error = run::RunConfig::load(&fixtures_path().join("invalid.json"))
        .expect_err("the file is not valid JSON");
    assert_eq!(error.kind(), ConfigErrorKind::InvalidJson);
    let message = error.to_string();
    assert!(message.contains("invalid.json"), "{message}");
    assert!(message.contains("valid JSON"), "{message}");
}

#[test]
fn the_error_format_names_the_file_path_and_values() {
    let error = data::parse(
        &set(data_object(), &["schema"], json!("quantrule.data/v2")),
        "data.json",
        Path::new("config"),
    )
    .expect_err("the schema is wrong");
    assert_eq!(
        error.to_string(),
        "config error: data.json: $.schema: expected quantrule.data/v3, found quantrule.data/v2"
    );

    let error = data::parse(
        &remove(data_object(), &["max_candles"]),
        "data.json",
        Path::new("config"),
    )
    .expect_err("the field is missing");
    assert_eq!(
        error.to_string(),
        "config error: data.json: $.max_candles: expected a required field, found missing"
    );
}
