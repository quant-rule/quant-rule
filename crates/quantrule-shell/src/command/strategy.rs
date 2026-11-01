//! `strategy-run` parsing and execution.
//!
//! The command owns source access and the strategy selection. It loads the
//! extended M1 range, hands the strategy's timeframe to the model, feeds every
//! closed model bar to the strategy, and renders the entries and exits. It
//! creates no order, fill, position, equity, or P&L record; those belong to the
//! backtest path.
//!
//! The default output is one NDJSON line per entry or exit. `--json` prints a
//! single provenance envelope whose `records` array carries the same records,
//! so a run can be reproduced from its printed configuration. The envelope
//! marks `execution_state` as `virtual`: the diagnostic walk assumes every
//! decision fills immediately, unlike `report strategy`, which reads the
//! executed backtest walk.

use std::io::{self, Write};
use std::path::PathBuf;

use quantrule_backtest::runner::walk_strategy;
use quantrule_backtest::strategy::{
    Decision, ExitReason, Side, StrategyConfig, StrategyInput, StrategyWalk,
};
use quantrule_data::{Timeframe, format_utc_timestamp};
use quantrule_model::input::MarketStateConfig;
use serde_json::{Value, json};

use super::types::{ParseError, RunError, StrategyRunArgs, StrategyRunError};
use crate::labels;
use crate::pipeline;
use crate::source::DateBounds;

/// The ladder-context strategy name.
pub(crate) const RULE_LADDER_CONTEXT: &str = "ladder-context";
/// The level-retest strategy name.
pub(crate) const RULE_LEVEL_RETEST: &str = "level-retest";
/// The three-timeframe strategy name.
pub(crate) const RULE_THREE_TIMEFRAME: &str = "three-timeframe";
/// The breakout strategy name.
pub(crate) const RULE_BREAKOUT: &str = "breakout";
/// The level-fade strategy name.
pub(crate) const RULE_LEVEL_FADE: &str = "level-fade";

/// Parses the already-split arguments of `strategy-run`.
///
/// # Errors
///
/// Returns [`ParseError::StrategyRun`] for a missing path, a missing or empty
/// required value, a repeated option, an unknown strategy, an invalid UTC date,
/// a reversed range, or an unknown option.
pub(crate) fn parse_strategy_run_arguments(
    arguments: &[String],
) -> Result<StrategyRunArgs, ParseError> {
    const USAGE: &str = "usage: strategy-run <csv-path> --strategy <name> --data-version <id> [--config <run.json>] [--from <date>] [--to <date>] [--json]";
    let mut path = None;
    let mut config = None;
    let mut strategy = None;
    let mut data_version = None;
    let mut from = None;
    let mut to = None;
    let mut json = false;
    let mut index = 0;

    while index < arguments.len() {
        match arguments[index].as_str() {
            "--config" => {
                if config.is_some() {
                    return Err(ParseError::StrategyRun(format!(
                        "--config was supplied twice\n{USAGE}"
                    )));
                }
                index += 1;
                let value = arguments
                    .get(index)
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| {
                        ParseError::StrategyRun(format!(
                            "--config requires a non-empty path\n{USAGE}"
                        ))
                    })?;
                config = Some(PathBuf::from(value));
            }
            "--strategy" => {
                if strategy.is_some() {
                    return Err(ParseError::StrategyRun(format!(
                        "--strategy was supplied twice\n{USAGE}"
                    )));
                }
                index += 1;
                let value = arguments
                    .get(index)
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| {
                        ParseError::StrategyRun(format!(
                            "--strategy requires a non-empty name\n{USAGE}"
                        ))
                    })?;
                if !is_supported_strategy(value) {
                    return Err(ParseError::StrategyRun(format!(
                        "unknown strategy '{value}'; use {RULE_LADDER_CONTEXT}, {RULE_LEVEL_RETEST}, {RULE_THREE_TIMEFRAME}, or {RULE_BREAKOUT}\n{USAGE}"
                    )));
                }
                strategy = Some(value.clone());
            }
            "--data-version" => {
                if data_version.is_some() {
                    return Err(ParseError::StrategyRun(format!(
                        "--data-version was supplied twice\n{USAGE}"
                    )));
                }
                index += 1;
                let value = arguments
                    .get(index)
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| {
                        ParseError::StrategyRun(format!(
                            "--data-version requires a non-empty value\n{USAGE}"
                        ))
                    })?;
                data_version = Some(value.clone());
            }
            "--from" => {
                if from.is_some() {
                    return Err(ParseError::StrategyRun(format!(
                        "--from was supplied twice\n{USAGE}"
                    )));
                }
                index += 1;
                let value = arguments.get(index).ok_or_else(|| {
                    ParseError::StrategyRun(format!("--from requires a date\n{USAGE}"))
                })?;
                from = Some(value.clone());
            }
            "--to" => {
                if to.is_some() {
                    return Err(ParseError::StrategyRun(format!(
                        "--to was supplied twice\n{USAGE}"
                    )));
                }
                index += 1;
                let value = arguments.get(index).ok_or_else(|| {
                    ParseError::StrategyRun(format!("--to requires a date\n{USAGE}"))
                })?;
                to = Some(value.clone());
            }
            "--json" => {
                if json {
                    return Err(ParseError::StrategyRun(format!(
                        "--json was supplied twice\n{USAGE}"
                    )));
                }
                json = true;
            }
            option if option.starts_with("--") => {
                return Err(ParseError::StrategyRun(format!(
                    "unknown option '{option}'\n{USAGE}"
                )));
            }
            value => {
                if path.is_some() {
                    return Err(ParseError::StrategyRun(format!(
                        "unexpected argument '{value}'\n{USAGE}"
                    )));
                }
                if value.is_empty() {
                    return Err(ParseError::StrategyRun(format!(
                        "the CSV path is empty\n{USAGE}"
                    )));
                }
                path = Some(PathBuf::from(value));
            }
        }
        index += 1;
    }

    let path = path.ok_or_else(|| ParseError::StrategyRun(format!("missing CSV path\n{USAGE}")))?;
    let strategy = strategy
        .ok_or_else(|| ParseError::StrategyRun(format!("--strategy is required\n{USAGE}")))?;
    let data_version = data_version
        .ok_or_else(|| ParseError::StrategyRun(format!("--data-version is required\n{USAGE}")))?;
    let bounds = DateBounds::parse(from.as_deref(), to.as_deref())
        .map_err(|error| ParseError::StrategyRun(format!("{error}\n{USAGE}")))?;

    Ok(StrategyRunArgs {
        path,
        config,
        strategy,
        data_version,
        from,
        to,
        bounds,
        json,
    })
}

/// Runs the selected strategy over the command's timeframe and prints the
/// resulting entries and exits.
///
/// # Errors
///
/// Returns [`RunError`] when the source, the model, the strategy selection, or
/// the writer fails.
pub(crate) fn run_strategy_run(
    args: &StrategyRunArgs,
    writer: &mut impl Write,
) -> Result<(), RunError> {
    let (config, thesis, model_config, run_bounds, run_from, run_to) = match &args.config {
        Some(path) => {
            let run = crate::config::RunConfig::load(path).map_err(RunError::Config)?;
            (
                run.strategy.engine,
                run.strategy.thesis,
                run.model,
                Some(run.data.bounds),
                run.data.from,
                run.data.to,
            )
        }
        None => (
            StrategyConfig::default(),
            String::new(),
            MarketStateConfig::default(),
            None,
            None,
            None,
        ),
    };
    if !is_supported_strategy(&args.strategy) {
        return Err(RunError::StrategyRun(StrategyRunError::UnknownStrategy {
            name: args.strategy.clone(),
        }));
    }
    let expected = if config.three_timeframe().is_some() {
        RULE_THREE_TIMEFRAME
    } else if matches!(
        config.mode(),
        quantrule_backtest::strategy::StrategyMode::LevelFade(_)
    ) {
        RULE_LEVEL_FADE
    } else if matches!(
        config.mode(),
        quantrule_backtest::strategy::StrategyMode::Breakout(_)
    ) {
        RULE_BREAKOUT
    } else if config.level().is_some() {
        RULE_LEVEL_RETEST
    } else {
        RULE_LADDER_CONTEXT
    };
    if args.strategy != expected {
        return Err(RunError::StrategyRun(StrategyRunError::ConfigMismatch {
            name: args.strategy.clone(),
            expected: expected.to_owned(),
        }));
    }
    let bounds = if args.from.is_none() && args.to.is_none() {
        run_bounds.unwrap_or(args.bounds)
    } else {
        args.bounds
    };
    // `requested_period` stays the CLI request. This is the range actually
    // applied: CLI dates when either is supplied, otherwise the run's data
    // bounds, otherwise the full file.
    let (effective_from, effective_to) = if args.from.is_some() || args.to.is_some() {
        (args.from.clone(), args.to.clone())
    } else {
        (run_from, run_to)
    };
    let set = pipeline::load_extended_set(&args.path, bounds).map_err(RunError::Source)?;
    if set.series(Timeframe::M1).is_empty() {
        return Err(RunError::StrategyRun(StrategyRunError::EmptyPeriod {
            period: args.requested_period(),
        }));
    }

    let model_run =
        pipeline::model_run_from_set_with_ladder(&set, &active_timeframes(&config)?, &model_config);
    let walk = walk_strategy(&model_run, &config).map_err(RunError::Backtest)?;
    let records = collect_records(&walk, &args.strategy);
    let period = PeriodReport {
        effective_from,
        effective_to,
        actual_first: walk
            .inputs()
            .first()
            .map(|input| format_utc_timestamp(input.open_time)),
        actual_last: walk
            .inputs()
            .last()
            .map(|input| format_utc_timestamp(input.open_time)),
        bars: walk.inputs().len(),
    };
    if args.json {
        write_line(
            &envelope_value(args, &config, &thesis, &records, &period),
            writer,
        )
        .map_err(RunError::Output)?;
    } else {
        for record in &records {
            write_line(record, writer).map_err(RunError::Output)?;
        }
    }
    writer.flush().map_err(RunError::Output)
}

fn is_supported_strategy(name: &str) -> bool {
    name == RULE_LADDER_CONTEXT
        || name == RULE_LEVEL_RETEST
        || name == RULE_THREE_TIMEFRAME
        || name == RULE_BREAKOUT
        || name == RULE_LEVEL_FADE
}

fn timeframe_for(seconds: u64) -> Option<Timeframe> {
    Timeframe::ALL
        .into_iter()
        .find(|timeframe| u64::from(timeframe.seconds()) == seconds)
}

fn active_timeframes(config: &StrategyConfig) -> Result<Vec<Timeframe>, RunError> {
    let mut seconds = Vec::with_capacity(config.ladder().context_timeframes().len() + 1);
    seconds.push(config.timeframe_secs());
    seconds.extend_from_slice(config.ladder().context_timeframes());
    seconds
        .into_iter()
        .map(|seconds| {
            timeframe_for(seconds).ok_or_else(|| {
                RunError::StrategyRun(StrategyRunError::UnsupportedTimeframe {
                    timeframe_secs: seconds,
                })
            })
        })
        .collect()
}

fn collect_records(walk: &StrategyWalk, name: &str) -> Vec<Value> {
    let mut records = Vec::new();
    for (input, decision) in walk.inputs().iter().zip(walk.decisions()) {
        match decision {
            Decision::Hold => {}
            Decision::Enter(entry) => {
                records.push(record_value("entry", name, entry.intent.side, None, input));
            }
            Decision::Exit(exit) => {
                records.push(record_value(
                    "exit",
                    name,
                    exit.side,
                    Some(exit.reason),
                    input,
                ));
            }
        }
    }
    records
}

fn record_value(
    kind: &'static str,
    name: &str,
    side: Side,
    reason: Option<ExitReason>,
    bar: &StrategyInput,
) -> Value {
    let mut value = json!({
        "type": kind,
        "strategy": name,
        "timeframe": labels::timeframe_label(bar.timeframe_secs),
        "timeframe_secs": bar.timeframe_secs,
        "bar": bar.bar_index,
        "datetime": format_utc_timestamp(bar.open_time),
        "side": side.code(),
        "side_label": side_label(side),
        "direction": bar.direction,
        "momentum_force": bar.momentum_force,
        "structure_score": bar.structure_score,
    });
    if let Some(reason) = reason {
        value["reason"] = json!(reason_code(reason));
    }
    value
}

/// The range and bar count the envelope records after the walk.
struct PeriodReport {
    effective_from: Option<String>,
    effective_to: Option<String>,
    actual_first: Option<String>,
    actual_last: Option<String>,
    bars: usize,
}

fn envelope_value(
    args: &StrategyRunArgs,
    config: &StrategyConfig,
    thesis: &str,
    records: &[Value],
    period: &PeriodReport,
) -> Value {
    json!({
        "model_version": env!("CARGO_PKG_VERSION"),
        "strategy": &args.strategy,
        "strategy_config": strategy_config_value(config, thesis),
        "data_version": &args.data_version,
        "execution_state": "virtual",
        "requested_period": {
            "from": args.from.as_deref(),
            "to": args.to.as_deref(),
        },
        "effective_period": {
            "from": period.effective_from.as_deref(),
            "to": period.effective_to.as_deref(),
        },
        "actual_period": {
            "first_open_time": period.actual_first.as_deref(),
            "last_open_time": period.actual_last.as_deref(),
        },
        "bars": period.bars,
        "timeframe_secs": config.timeframe_secs(),
        "records": records,
    })
}

pub(super) fn strategy_config_value(config: &StrategyConfig, thesis: &str) -> Value {
    let context = config.ladder().context_timeframes();
    let entry = config.entry();
    let execution = entry.execution();
    let context_entry = entry.context();
    let exit = config.exit();
    let mut exit_on = Vec::new();
    if exit.on_context_lost() {
        exit_on.push("context_lost");
    }
    if exit.on_structure_lost() {
        exit_on.push("structure_lost");
    }
    if exit.on_level_lost() {
        exit_on.push("level_lost");
    }

    let mut entry_value = json!({
        "execution": {
            "direction_min": execution.direction_min(),
            "momentum_min": execution.momentum_min(),
            "structure_min": execution.structure_min(),
            "structure_shift": {
                "from_max": execution.structure_shift_from_max(),
                "to_min": execution.structure_shift_to_min(),
            },
        },
        "context": {
            "direction": {
                "kind": "bullish",
                "share_min": context_entry.direction_share_min(),
            },
            "momentum": {
                "kind": "not_bearish",
                "bearish_share_max": context_entry.momentum_bearish_share_max(),
            },
            "structure": {
                "conflict_count_max": context_entry.structure_conflict_count_max(),
            },
            "progression": {
                "direction": "bullish",
                "late_or_terminal_count_max": context_entry
                    .progression_late_or_terminal_count_max(),
            },
        },
    });
    if let Some(level) = config.level() {
        entry_value["level"] = json!({
            "max_distance_fraction": level.max_distance_fraction(),
            "scale": level.scale().map_or("both", |scale| match scale {
                quantrule_model::readings::PivotScale::Minor => "minor",
                quantrule_model::readings::PivotScale::Major => "major",
            }),
        });
    }
    if let Some(three) = config.three_timeframe() {
        entry_value["agreement"] = json!({
            "min_bars": three.min_agreement_bars(),
        });
    }

    let (name, risk_value) = if let Some(three) = config.three_timeframe() {
        let level = three.level();
        (
            "three-timeframe",
            json!({
                "stop": {"kind": "level", "buffer_fraction": level.stop_buffer_fraction()},
                "target": {
                    "kind": "next_level",
                    "min_reward_to_risk": level.min_reward_to_risk(),
                },
                "agreement": {"min_bars": three.min_agreement_bars()},
            }),
        )
    } else if let Some(risk) = config.risk() {
        (
            "ladder-context",
            json!({
                "stop": {"kind": "atr", "multiple": risk.stop_atr_multiple()},
                "target": {
                    "kind": "reward_to_risk",
                    "multiple": risk.reward_to_risk(),
                },
            }),
        )
    } else if let Some(level) = config.level() {
        let name = if matches!(
            config.mode(),
            quantrule_backtest::strategy::StrategyMode::LevelFade(_)
        ) {
            "level-fade"
        } else if matches!(
            config.mode(),
            quantrule_backtest::strategy::StrategyMode::Breakout(_)
        ) {
            "breakout"
        } else {
            "level-retest"
        };
        (
            name,
            json!({
                "stop": {"kind": "level", "buffer_fraction": level.stop_buffer_fraction()},
                "target": {
                    "kind": "next_level",
                    "min_reward_to_risk": level.min_reward_to_risk(),
                },
            }),
        )
    } else {
        ("ladder-context", Value::Null)
    };

    json!({
        "name": name,
        "thesis": thesis,
        "timeframe_secs": config.timeframe_secs(),
        "ladder": ladder_labels(config),
        "weights": {
            "direction": weight_map(context, config.weights().direction()),
            "momentum": weight_map(context, config.weights().momentum()),
            "structure": weight_map(context, config.weights().structure()),
            "progression": weight_map(context, config.weights().progression()),
        },
        "entry": entry_value,
        "exit": {"on": exit_on},
        "risk": risk_value,
        "no_trade": {
            "missing_context": "reject",
            "require_contiguous_execution": config.require_contiguous_execution(),
        },
    })
}

fn ladder_labels(config: &StrategyConfig) -> Vec<&'static str> {
    let mut labels = Vec::with_capacity(config.ladder().context_timeframes().len() + 1);
    labels.push(labels::timeframe_label(config.timeframe_secs()));
    for seconds in config.ladder().context_timeframes() {
        labels.push(labels::timeframe_label(*seconds));
    }
    labels
}

fn weight_map(timeframes: &[u64], weights: &[f64]) -> Value {
    let mut map = serde_json::Map::new();
    for (seconds, weight) in timeframes.iter().zip(weights) {
        map.insert(labels::timeframe_label(*seconds).to_owned(), json!(weight));
    }
    Value::Object(map)
}

fn side_label(side: Side) -> &'static str {
    match side {
        Side::Long => "long",
        Side::Short => "short",
    }
}

fn reason_code(reason: ExitReason) -> &'static str {
    match reason {
        ExitReason::ContextLost => "context_lost",
        ExitReason::StructureLost => "structure_lost",
        ExitReason::LevelLost => "level_lost",
    }
}

fn write_line(value: &Value, writer: &mut impl Write) -> io::Result<()> {
    let text = serde_json::to_string(value).map_err(io::Error::other)?;
    writer.write_all(text.as_bytes())?;
    writer.write_all(b"\n")
}

#[cfg(test)]
mod tests {
    use super::{
        RULE_BREAKOUT, RULE_LADDER_CONTEXT, RULE_LEVEL_FADE, RULE_LEVEL_RETEST,
        RULE_THREE_TIMEFRAME, is_supported_strategy, parse_strategy_run_arguments, reason_code,
        record_value, timeframe_for,
    };
    use crate::command::types::ParseError;
    use quantrule_backtest::strategy::{ExitReason, Side, StrategyConfig, StrategyInput};
    use quantrule_data::Timeframe;

    fn arguments(tokens: &[&str]) -> Vec<String> {
        tokens.iter().map(|token| (*token).to_owned()).collect()
    }

    #[test]
    fn parses_every_option() {
        let args = parse_strategy_run_arguments(&arguments(&[
            "data/m1.csv",
            "--strategy",
            "ladder-context",
            "--data-version",
            "source:sol:60:0-1000",
            "--from",
            "2025-01-01",
            "--to",
            "2025-12-31",
            "--json",
        ]))
        .expect("the arguments are valid");

        assert_eq!(args.strategy, RULE_LADDER_CONTEXT);
        assert_eq!(args.data_version, "source:sol:60:0-1000");
        assert_eq!(args.from.as_deref(), Some("2025-01-01"));
        assert_eq!(args.to.as_deref(), Some("2025-12-31"));
        assert!(args.json);
        assert_eq!(args.requested_period(), "2025-01-01..2025-12-31");
    }

    #[test]
    fn bounds_are_optional() {
        let args = parse_strategy_run_arguments(&arguments(&[
            "m1.csv",
            "--strategy",
            "ladder-context",
            "--data-version",
            "v1",
        ]))
        .expect("the arguments are valid");

        assert_eq!(args.bounds.start_seconds(), None);
        assert_eq!(args.bounds.end_exclusive_seconds(), None);
        assert_eq!(args.requested_period(), "full file");
        assert!(!args.json);
    }

    #[test]
    fn rejects_missing_required_options() {
        assert!(matches!(
            parse_strategy_run_arguments(&arguments(&["m1.csv", "--strategy", "ladder-context"])),
            Err(ParseError::StrategyRun(message)) if message.contains("--data-version is required")
        ));
        assert!(matches!(
            parse_strategy_run_arguments(&arguments(&[
                "m1.csv",
                "--data-version",
                "v1",
            ])),
            Err(ParseError::StrategyRun(message)) if message.contains("--strategy is required")
        ));
        assert!(matches!(
            parse_strategy_run_arguments(&arguments(&[
                "--strategy",
                "ladder-context",
                "--data-version",
                "v1",
            ])),
            Err(ParseError::StrategyRun(message)) if message.contains("missing CSV path")
        ));
    }

    #[test]
    fn rejects_an_unknown_strategy() {
        assert!(matches!(
            parse_strategy_run_arguments(&arguments(&[
                "m1.csv",
                "--strategy",
                "ema-cross",
                "--data-version",
                "v1",
            ])),
            Err(ParseError::StrategyRun(message)) if message.contains("unknown strategy")
        ));
    }

    #[test]
    fn rejects_invalid_dates() {
        assert!(matches!(
            parse_strategy_run_arguments(&arguments(&[
                "m1.csv",
                "--strategy",
                "ladder-context",
                "--data-version",
                "v1",
                "--from",
                "2025-02-30",
            ])),
            Err(ParseError::StrategyRun(message)) if message.contains("invalid UTC date")
        ));
        assert!(matches!(
            parse_strategy_run_arguments(&arguments(&[
                "m1.csv",
                "--strategy",
                "ladder-context",
                "--data-version",
                "v1",
                "--from",
                "2025-01-02",
                "--to",
                "2025-01-01",
            ])),
            Err(ParseError::StrategyRun(message)) if message.contains("is after")
        ));
    }

    #[test]
    fn rejects_repeated_and_unknown_options() {
        assert!(matches!(
            parse_strategy_run_arguments(&arguments(&[
                "m1.csv",
                "--strategy",
                "ladder-context",
                "--strategy",
                "ladder-context",
                "--data-version",
                "v1",
            ])),
            Err(ParseError::StrategyRun(message)) if message.contains("twice")
        ));
        assert!(matches!(
            parse_strategy_run_arguments(&arguments(&[
                "m1.csv",
                "--strategy",
                "ladder-context",
                "--data-version",
                "v1",
                "--out",
                "out",
            ])),
            Err(ParseError::StrategyRun(message)) if message.contains("unknown option")
        ));
    }

    #[test]
    fn the_default_strategy_is_the_m5_ladder_context_rule() {
        let config = StrategyConfig::default();
        assert_eq!(config.timeframe_secs(), 300);
        assert_eq!(timeframe_for(config.timeframe_secs()), Some(Timeframe::M5));
        assert!(is_supported_strategy(RULE_LADDER_CONTEXT));
        assert!(is_supported_strategy(RULE_LEVEL_RETEST));
        assert!(is_supported_strategy(RULE_THREE_TIMEFRAME));
        assert!(is_supported_strategy(RULE_BREAKOUT));
        assert!(is_supported_strategy(RULE_LEVEL_FADE));
        assert!(!is_supported_strategy("ema-cross"));
    }

    #[test]
    fn an_entry_record_carries_readings_and_no_fill_fields() {
        let bar = StrategyInput::new(300, 7, 1_700_000_000, Some(0.2), Some(0.5), Some(1.0));
        let value = record_value("entry", RULE_LADDER_CONTEXT, Side::Long, None, &bar);

        assert_eq!(value["type"], "entry");
        assert_eq!(value["strategy"], RULE_LADDER_CONTEXT);
        assert_eq!(value["timeframe"], "5m");
        assert_eq!(value["bar"], 7);
        assert_eq!(value["side"], 1);
        assert_eq!(value["side_label"], "long");
        assert_eq!(value["direction"], 0.2);
        assert_eq!(value["momentum_force"], 0.5);
        assert_eq!(value["structure_score"], 1.0);
        assert!(value.get("reason").is_none());
        assert!(value.get("fill_price").is_none());
        assert!(value.get("quantity").is_none());
        assert!(value.get("equity").is_none());
        assert!(value.get("pnl").is_none());
    }

    #[test]
    fn an_exit_record_carries_its_reason() {
        let bar = StrategyInput::new(900, 8, 1_700_000_900, Some(0.0), Some(0.0), Some(0.0));
        let value = record_value(
            "exit",
            RULE_LADDER_CONTEXT,
            Side::Short,
            Some(ExitReason::ContextLost),
            &bar,
        );

        assert_eq!(value["type"], "exit");
        assert_eq!(value["side"], -1);
        assert_eq!(value["side_label"], "short");
        assert_eq!(value["reason"], "context_lost");
        assert_eq!(reason_code(ExitReason::ContextLost), "context_lost");
    }
}
