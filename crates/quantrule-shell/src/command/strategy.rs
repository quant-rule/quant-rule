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
//! so a run can be reproduced from its printed configuration.

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

/// The strategy name the command accepts.
pub(crate) const AXIS_AGREEMENT: &str = "axis-agreement";

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
                        "unknown strategy '{value}'; use {AXIS_AGREEMENT}\n{USAGE}"
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
    let (config, model_config) = match &args.config {
        Some(path) => {
            let run = crate::config::RunConfig::load(path).map_err(RunError::Config)?;
            (run.strategy.engine, run.model)
        }
        None => (StrategyConfig::default(), MarketStateConfig::default()),
    };
    if !is_supported_strategy(&args.strategy) {
        return Err(RunError::StrategyRun(StrategyRunError::UnknownStrategy {
            name: args.strategy.clone(),
        }));
    }
    let timeframe = timeframe_for(config.timeframe_secs()).ok_or_else(|| {
        RunError::StrategyRun(StrategyRunError::UnsupportedTimeframe {
            timeframe_secs: config.timeframe_secs(),
        })
    })?;

    let set = pipeline::load_extended_set(&args.path, args.bounds).map_err(RunError::Source)?;
    if set.series(Timeframe::M1).is_empty() {
        return Err(RunError::StrategyRun(StrategyRunError::EmptyPeriod {
            period: args.requested_period(),
        }));
    }

    let run = pipeline::model_run_from_set(&set, &model_config);
    let model = run
        .timeframe(u64::from(timeframe.seconds()))
        .ok_or_else(|| {
            RunError::StrategyRun(StrategyRunError::NoTimeframeBars {
                timeframe_secs: config.timeframe_secs(),
                period: args.requested_period(),
            })
        })?;

    let walk = walk_strategy(model, &config).map_err(RunError::Backtest)?;
    let records = collect_records(&walk);
    if args.json {
        write_line(&envelope_value(args, &config, &records), writer).map_err(RunError::Output)?;
    } else {
        for record in &records {
            write_line(record, writer).map_err(RunError::Output)?;
        }
    }
    writer.flush().map_err(RunError::Output)
}

fn is_supported_strategy(name: &str) -> bool {
    name == AXIS_AGREEMENT
}

fn timeframe_for(seconds: u64) -> Option<Timeframe> {
    Timeframe::ALL
        .into_iter()
        .find(|timeframe| u64::from(timeframe.seconds()) == seconds)
}

fn collect_records(walk: &StrategyWalk) -> Vec<Value> {
    let mut records = Vec::new();
    for (input, decision) in walk.inputs().iter().zip(walk.decisions()) {
        match decision {
            Decision::Hold => {}
            Decision::Enter(entry) => {
                records.push(record_value("entry", entry.side, None, input));
            }
            Decision::Exit(exit) => {
                records.push(record_value("exit", exit.side, Some(exit.reason), input));
            }
        }
    }
    records
}

fn record_value(
    kind: &'static str,
    side: Side,
    reason: Option<ExitReason>,
    bar: &StrategyInput,
) -> Value {
    let mut value = json!({
        "type": kind,
        "strategy": AXIS_AGREEMENT,
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

fn envelope_value(args: &StrategyRunArgs, config: &StrategyConfig, records: &[Value]) -> Value {
    json!({
        "model_version": env!("CARGO_PKG_VERSION"),
        "strategy": &args.strategy,
        "strategy_config": {
            "timeframe_secs": config.timeframe_secs(),
            "long_threshold": config.long_threshold(),
            "short_threshold": config.short_threshold(),
        },
        "data_version": &args.data_version,
        "requested_period": {
            "from": args.from.as_deref(),
            "to": args.to.as_deref(),
        },
        "timeframe_secs": config.timeframe_secs(),
        "records": records,
    })
}

fn side_label(side: Side) -> &'static str {
    match side {
        Side::Long => "long",
        Side::Short => "short",
    }
}

fn reason_code(reason: ExitReason) -> &'static str {
    match reason {
        ExitReason::AgreementLost => "agreement_lost",
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
        AXIS_AGREEMENT, is_supported_strategy, parse_strategy_run_arguments, reason_code,
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
            "axis-agreement",
            "--data-version",
            "source:sol:60:0-1000",
            "--from",
            "2025-01-01",
            "--to",
            "2025-12-31",
            "--json",
        ]))
        .expect("the arguments are valid");

        assert_eq!(args.strategy, AXIS_AGREEMENT);
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
            "axis-agreement",
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
            parse_strategy_run_arguments(&arguments(&["m1.csv", "--strategy", "axis-agreement"])),
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
                "axis-agreement",
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
                "axis-agreement",
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
                "axis-agreement",
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
                "axis-agreement",
                "--strategy",
                "axis-agreement",
                "--data-version",
                "v1",
            ])),
            Err(ParseError::StrategyRun(message)) if message.contains("twice")
        ));
        assert!(matches!(
            parse_strategy_run_arguments(&arguments(&[
                "m1.csv",
                "--strategy",
                "axis-agreement",
                "--data-version",
                "v1",
                "--out",
                "out",
            ])),
            Err(ParseError::StrategyRun(message)) if message.contains("unknown option")
        ));
    }

    #[test]
    fn the_default_strategy_is_the_m15_axis_agreement_rule() {
        let config = StrategyConfig::default();
        assert_eq!(config.timeframe_secs(), 900);
        assert_eq!(timeframe_for(config.timeframe_secs()), Some(Timeframe::M15));
        assert!(is_supported_strategy(AXIS_AGREEMENT));
        assert!(!is_supported_strategy("ema-cross"));
    }

    #[test]
    fn an_entry_record_carries_readings_and_no_fill_fields() {
        let bar = StrategyInput::new(900, 7, 1_700_000_000, Some(0.2), Some(0.5), Some(1.0));
        let value = record_value("entry", Side::Long, None, &bar);

        assert_eq!(value["type"], "entry");
        assert_eq!(value["strategy"], AXIS_AGREEMENT);
        assert_eq!(value["timeframe"], "15m");
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
        let value = record_value("exit", Side::Short, Some(ExitReason::AgreementLost), &bar);

        assert_eq!(value["type"], "exit");
        assert_eq!(value["side"], -1);
        assert_eq!(value["side_label"], "short");
        assert_eq!(value["reason"], "agreement_lost");
        assert_eq!(reason_code(ExitReason::AgreementLost), "agreement_lost");
    }
}
