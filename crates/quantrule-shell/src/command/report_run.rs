//! The config-driven `report` command.
//!
//! R1 wires the model and strategy bundles. The command loads the run
//! configuration, runs only the engines the selected report needs, and writes
//! the bundle through the shared writer. The other three report names answer
//! with a clear not-implemented error until R2 and R3.

use std::fmt::Write as _;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use quantrule_backtest::runner::BacktestRun;
use quantrule_backtest::runner::walk_strategy;
use quantrule_backtest::strategy::{Decision, ExitReason, Side, StrategyInput, StrategyWalk};
use quantrule_data::{Timeframe, format_utc_timestamp};
use quantrule_model::readings::PivotScale;
use quantrule_model::report::MarketReport;
use quantrule_model::run::ModelRun;
use quantrule_model::timeframe::TimeframeModel;
use serde_json::{Value, json};

pub(super) mod bundles;

use super::types::{ParseError, ReportArgs, ReportEngine, ReportError, RunError};
use crate::chart;
use crate::config::RunConfig;
use crate::pipeline::{self, PipelineError};
use crate::render_report::{self, Provenance};
use crate::source::DateRange;

/// Parses the already-split arguments of `report`.
///
/// # Errors
///
/// Returns [`ParseError::Report`] for a missing engine, a missing or repeated
/// `--config`/`--out`, an unknown option, or an extra positional argument.
pub(crate) fn parse_report_arguments(arguments: &[String]) -> Result<ReportArgs, ParseError> {
    const USAGE: &str = "usage: report <model|strategy|execution|portfolio|analytical> --config <run.json> [--out <dir>] [--json]";
    let mut engine = None;
    let mut config = None;
    let mut out = None;
    let mut json = false;
    let mut index = 0;

    while index < arguments.len() {
        match arguments[index].as_str() {
            "--config" => {
                if config.is_some() {
                    return Err(ParseError::Report(format!(
                        "--config was supplied twice\n{USAGE}"
                    )));
                }
                index += 1;
                let value = arguments
                    .get(index)
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| {
                        ParseError::Report(format!("--config requires a non-empty path\n{USAGE}"))
                    })?;
                config = Some(PathBuf::from(value));
            }
            "--out" => {
                if out.is_some() {
                    return Err(ParseError::Report(format!(
                        "--out was supplied twice\n{USAGE}"
                    )));
                }
                index += 1;
                let value = arguments
                    .get(index)
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| {
                        ParseError::Report(format!("--out requires a non-empty path\n{USAGE}"))
                    })?;
                out = Some(PathBuf::from(value));
            }
            "--json" => {
                if json {
                    return Err(ParseError::Report(format!(
                        "--json was supplied twice\n{USAGE}"
                    )));
                }
                json = true;
            }
            option if option.starts_with("--") => {
                return Err(ParseError::Report(format!(
                    "unknown option '{option}'\n{USAGE}"
                )));
            }
            value => {
                if engine.is_some() {
                    return Err(ParseError::Report(format!(
                        "unexpected argument '{value}'\n{USAGE}"
                    )));
                }
                engine = Some(ReportEngine::parse(value).ok_or_else(|| {
                    ParseError::Report(format!(
                        "unknown report engine '{value}'; use model, strategy, execution, portfolio, or analytical\n{USAGE}"
                    ))
                })?);
            }
        }
        index += 1;
    }

    let engine =
        engine.ok_or_else(|| ParseError::Report(format!("missing report engine\n{USAGE}")))?;
    let config =
        config.ok_or_else(|| ParseError::Report(format!("--config is required\n{USAGE}")))?;

    Ok(ReportArgs {
        engine,
        config,
        out,
        json,
    })
}

/// Loads the run configuration and writes the selected bundle.
///
/// # Errors
///
/// Returns a [`RunError`] when the configuration, the source, the model, a
/// chart, or an artifact fails.
pub(crate) fn run_report(args: &ReportArgs, writer: &mut impl Write) -> Result<(), RunError> {
    let config = RunConfig::load(&args.config).map_err(RunError::Config)?;
    let out = args
        .out
        .clone()
        .unwrap_or_else(|| config.output_root.clone());

    let bundle = match args.engine {
        ReportEngine::Model => {
            let model = load_model(&config)?;
            model_bundle_from(&config, &out, &model)?
        }
        ReportEngine::Strategy => {
            let model = load_model(&config)?;
            let timeframe_secs = config.strategy.engine.timeframe_secs();
            let timeframe = model
                .timeframe(timeframe_secs)
                .ok_or_else(|| RunError::Report(ReportError::NoStrategyBars { timeframe_secs }))?;
            let walk =
                walk_strategy(timeframe, &config.strategy.engine).map_err(RunError::Backtest)?;
            strategy_bundle_from_walk(&config, &out, timeframe, &walk)?
        }
        ReportEngine::Execution => {
            let run = run_pipeline(&config)?;
            bundles::execution_bundle_from(&config, &out, &run)?
        }
        ReportEngine::Portfolio => {
            let run = run_pipeline(&config)?;
            bundles::portfolio_bundle_from(&config, &out, &run)?
        }
        ReportEngine::Analytical => {
            let model = load_model(&config)?;
            bundles::analytical_bundle_from(&config, &out, &model)?
        }
    };
    emit(writer, args.json, &bundle)
}

/// One written bundle and the bytes of its `report.json`.
pub(super) struct BundleOutput {
    pub(super) path: PathBuf,
    pub(super) names: Vec<String>,
    pub(super) json: Vec<u8>,
}

pub(super) fn model_bundle_from(
    config: &RunConfig,
    out: &Path,
    model_run: &ModelRun,
) -> Result<BundleOutput, RunError> {
    let bundle = out.join("model");
    render_report::ensure_available(&bundle).map_err(RunError::Bundle)?;

    let report = MarketReport::compute(model_run, &pipeline::ladder_secs());
    if report.charts.bars.is_empty() {
        return Err(RunError::Report(ReportError::EmptyChart {
            period: period_label(config),
        }));
    }

    let summary =
        render_report::summary_bytes(&report, &config.data.data_version, &period_label(config))
            .map_err(RunError::Output)?;
    let json = render_report::json_bytes(
        &report,
        &config.data.data_version,
        config.data.from.as_deref(),
        config.data.to.as_deref(),
    )
    .map_err(RunError::Output)?;
    let price = chart::render_report_price_axes_png(&report.charts).map_err(RunError::Chart)?;
    let events = chart::render_report_events_png(&report.charts).map_err(RunError::Chart)?;

    let names = artifact_names(&[
        "summary.txt",
        "report.json",
        "price-axes.png",
        "events.png",
        "provenance.json",
    ]);
    let files = config_files(config);
    let provenance = render_report::provenance_bytes(&Provenance {
        report: "model",
        data_version: &config.data.data_version,
        config_hash: &config.config_hash,
        from: config.data.from.as_deref(),
        to: config.data.to.as_deref(),
        first_open_time: report.period.first_open_time,
        last_open_time: report.period.last_open_time,
        config_files: &files,
        config: json!({
            "structure_scale": scale_label(config.model.structure_scale),
            "ladder": ladder_codes(config),
            "policy": policy_value(config),
            "strategy": strategy_value(config),
        }),
        artifacts: &names,
    })
    .map_err(RunError::Output)?;

    let artifacts = vec![
        ("summary.txt", summary),
        ("report.json", json.clone()),
        ("price-axes.png", price),
        ("events.png", events),
    ];
    render_report::write_bundle(&bundle, &artifacts, &provenance).map_err(RunError::Bundle)?;
    Ok(BundleOutput {
        path: bundle,
        names,
        json,
    })
}

pub(super) fn strategy_bundle_from_walk(
    config: &RunConfig,
    out: &Path,
    timeframe: &TimeframeModel,
    walk: &StrategyWalk,
) -> Result<BundleOutput, RunError> {
    let bundle = out.join("strategy");
    render_report::ensure_available(&bundle).map_err(RunError::Bundle)?;

    let timeframe_secs = config.strategy.engine.timeframe_secs();
    if timeframe.is_empty() {
        return Err(RunError::Report(ReportError::NoStrategyBars {
            timeframe_secs,
        }));
    }

    let inputs = walk.inputs();
    let decisions = walk.decisions();
    let summary = summarize_walk(walk, timeframe.len());
    let csv = decisions_csv(inputs, decisions, timeframe_secs);
    let report_json = strategy_json(config, timeframe, inputs, decisions, &summary);
    let summary_text = strategy_summary(config, &summary);
    let chart = strategy_chart(config, timeframe, decisions)?;

    let names = artifact_names(&[
        "summary.txt",
        "report.json",
        "decisions.csv",
        "price-decisions.png",
        "provenance.json",
    ]);
    let files = config_files(config);
    let provenance = render_report::provenance_bytes(&Provenance {
        report: "strategy",
        data_version: &config.data.data_version,
        config_hash: &config.config_hash,
        from: config.data.from.as_deref(),
        to: config.data.to.as_deref(),
        first_open_time: timeframe.candle(0).map(|candle| candle.timestamp()),
        last_open_time: timeframe
            .candle(timeframe.len() - 1)
            .map(|candle| candle.timestamp()),
        config_files: &files,
        config: json!({
            "structure_scale": scale_label(config.model.structure_scale),
            "strategy": strategy_value(config),
        }),
        artifacts: &names,
    })
    .map_err(RunError::Output)?;

    let json_bytes = json_bytes(&report_json)?;
    let artifacts = vec![
        ("summary.txt", summary_text.into_bytes()),
        ("report.json", json_bytes.clone()),
        ("decisions.csv", csv.into_bytes()),
        ("price-decisions.png", chart),
    ];
    render_report::write_bundle(&bundle, &artifacts, &provenance).map_err(RunError::Bundle)?;
    Ok(BundleOutput {
        path: bundle,
        names,
        json: json_bytes,
    })
}

#[derive(Debug, Default)]
struct StrategySummary {
    bars: usize,
    entries: usize,
    exits: usize,
    long_entries: usize,
    short_entries: usize,
    agreements_long: usize,
    agreements_short: usize,
    agreements_neutral: usize,
    agreements_unavailable: usize,
    open_position: Option<Side>,
}

fn summarize_walk(walk: &StrategyWalk, bars: usize) -> StrategySummary {
    let agreements = walk.agreements();
    let mut summary = StrategySummary {
        bars,
        agreements_long: agreements.long,
        agreements_short: agreements.short,
        agreements_neutral: agreements.neutral,
        agreements_unavailable: agreements.unavailable,
        open_position: walk.open_position(),
        ..StrategySummary::default()
    };

    for decision in walk.decisions() {
        match decision {
            Decision::Enter(entry) => {
                summary.entries += 1;
                match entry.side {
                    Side::Long => summary.long_entries += 1,
                    Side::Short => summary.short_entries += 1,
                }
            }
            Decision::Exit(_) => summary.exits += 1,
            Decision::Hold => {}
        }
    }
    summary
}

fn decisions_csv(inputs: &[StrategyInput], decisions: &[Decision], timeframe_secs: u64) -> String {
    let mut csv = String::from(
        "type,side,timeframe,timeframe_secs,bar,datetime,direction,momentum_force,structure_score,reason\n",
    );
    let timeframe = timeframe_label(timeframe_secs);
    for (input, decision) in inputs.iter().zip(decisions) {
        let (kind, side, reason) = match decision {
            Decision::Hold => continue,
            Decision::Enter(entry) => ("entry", entry.side, String::new()),
            Decision::Exit(exit) => ("exit", exit.side, exit_reason_code(exit.reason).to_owned()),
        };
        let _ = writeln!(
            csv,
            "{kind},{},{timeframe},{timeframe_secs},{},{},{},{},{},{reason}",
            side.code(),
            input.bar_index,
            format_utc_timestamp(input.open_time),
            number(input.direction),
            number(input.momentum_force),
            number(input.structure_score),
        );
    }
    csv
}

fn strategy_json(
    config: &RunConfig,
    timeframe: &TimeframeModel,
    inputs: &[StrategyInput],
    decisions: &[Decision],
    summary: &StrategySummary,
) -> Value {
    let records: Vec<Value> = inputs
        .iter()
        .zip(decisions)
        .filter_map(|(input, decision)| {
            let (kind, side, reason) = match decision {
                Decision::Hold => return None,
                Decision::Enter(entry) => ("entry", entry.side, Value::Null),
                Decision::Exit(exit) => (
                    "exit",
                    exit.side,
                    Value::String(exit_reason_code(exit.reason).to_owned()),
                ),
            };
            Some(json!({
                "type": kind,
                "side": side.code(),
                "timeframe_secs": input.timeframe_secs,
                "bar": input.bar_index,
                "time": input.open_time,
                "direction": input.direction,
                "momentum_force": input.momentum_force,
                "structure_score": input.structure_score,
                "reason": reason,
            }))
        })
        .collect();

    json!({
        "report": "strategy",
        "report_version": render_report::REPORT_VERSION,
        "model_version": env!("CARGO_PKG_VERSION"),
        "data_version": config.data.data_version,
        "requested_period": {
            "from": config.data.from,
            "to": config.data.to,
        },
        "actual_period": {
            "first_open_time": timeframe.candle(0).map(|candle| candle.timestamp()),
            "last_open_time": timeframe.candle(timeframe.len() - 1).map(|candle| candle.timestamp()),
        },
        "config": {
            "structure_scale": scale_label(config.model.structure_scale),
            "strategy": strategy_value(config),
        },
        "summary": {
            "bars": summary.bars,
            "decisions": summary.entries + summary.exits,
            "entries": summary.entries,
            "exits": summary.exits,
            "long_entries": summary.long_entries,
            "short_entries": summary.short_entries,
            "exit_reasons": {"agreement_lost": summary.exits},
            "agreements": {
                "long": summary.agreements_long,
                "short": summary.agreements_short,
                "neutral": summary.agreements_neutral,
                "unavailable": summary.agreements_unavailable,
            },
            "open_position": summary.open_position.map(|side| side.code()),
        },
        "records": {"decisions": records},
    })
}

fn strategy_summary(config: &RunConfig, summary: &StrategySummary) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "QuantRule strategy report");
    let _ = writeln!(out, "data_version: {}", config.data.data_version);
    let _ = writeln!(out, "requested_period: {}", period_label(config));
    let _ = writeln!(
        out,
        "timeframe_secs: {}",
        config.strategy.engine.timeframe_secs()
    );
    let _ = writeln!(
        out,
        "long_threshold: {}",
        config.strategy.engine.long_threshold()
    );
    let _ = writeln!(
        out,
        "short_threshold: {}",
        config.strategy.engine.short_threshold()
    );
    let _ = writeln!(out, "bars: {}", summary.bars);
    let _ = writeln!(out, "decisions: {}", summary.entries + summary.exits);
    let _ = writeln!(out, "entries: {}", summary.entries);
    let _ = writeln!(out, "exits: {}", summary.exits);
    let _ = writeln!(out, "long_entries: {}", summary.long_entries);
    let _ = writeln!(out, "short_entries: {}", summary.short_entries);
    let _ = writeln!(out, "exit_reasons:");
    let _ = writeln!(out, "  agreement_lost: {}", summary.exits);
    let _ = writeln!(out, "agreements:");
    let _ = writeln!(out, "  long: {}", summary.agreements_long);
    let _ = writeln!(out, "  short: {}", summary.agreements_short);
    let _ = writeln!(out, "  neutral: {}", summary.agreements_neutral);
    let _ = writeln!(out, "  unavailable: {}", summary.agreements_unavailable);
    let _ = writeln!(
        out,
        "open_position: {}",
        summary.open_position.map_or("flat", |side| match side {
            Side::Long => "long",
            Side::Short => "short",
        })
    );
    out
}

fn strategy_chart(
    config: &RunConfig,
    timeframe: &TimeframeModel,
    decisions: &[Decision],
) -> Result<Vec<u8>, RunError> {
    let candles: Vec<_> = (0..timeframe.len())
        .filter_map(|bar| timeframe.candle(bar))
        .collect();
    if candles.is_empty() {
        return Err(RunError::Report(ReportError::NoStrategyBars {
            timeframe_secs: config.strategy.engine.timeframe_secs(),
        }));
    }
    let range = chart_range(config, &candles)?;
    let markers: Vec<(usize, bool)> = decisions
        .iter()
        .enumerate()
        .filter_map(|(bar, decision)| match decision {
            Decision::Enter(_) => Some((bar, true)),
            Decision::Exit(_) => Some((bar, false)),
            Decision::Hold => None,
        })
        .collect();
    let timeframe = Timeframe::ALL
        .iter()
        .copied()
        .find(|timeframe| u64::from(timeframe.seconds()) == config.strategy.engine.timeframe_secs())
        .unwrap_or(Timeframe::M15);
    chart::render_decisions_png(&candles, &markers, &config.name, &range, timeframe)
        .map_err(RunError::Chart)
}

fn chart_range(
    config: &RunConfig,
    candles: &[quantrule_data::Candle],
) -> Result<DateRange, RunError> {
    let Some(first) = candles.first() else {
        return Err(RunError::Report(ReportError::NoStrategyBars {
            timeframe_secs: config.strategy.engine.timeframe_secs(),
        }));
    };
    let Some(last) = candles.last() else {
        return Err(RunError::Report(ReportError::NoStrategyBars {
            timeframe_secs: config.strategy.engine.timeframe_secs(),
        }));
    };
    let from = config
        .data
        .from
        .clone()
        .unwrap_or_else(|| day_label(first.timestamp()));
    let to = config
        .data
        .to
        .clone()
        .unwrap_or_else(|| day_label(last.timestamp()));
    DateRange::parse(&from, &to)
        .map_err(|error| RunError::Report(ReportError::Period(error.to_string())))
}

/// Runs the full pipeline once for the execution and portfolio reports.
pub(super) fn run_pipeline(config: &RunConfig) -> Result<BacktestRun, RunError> {
    pipeline::run_backtest(config).map_err(|error| pipeline_error(config, error))
}

fn load_model(config: &RunConfig) -> Result<ModelRun, RunError> {
    pipeline::run_model(config).map_err(|error| pipeline_error(config, error))
}

fn pipeline_error(config: &RunConfig, error: PipelineError) -> RunError {
    match error {
        PipelineError::Source(error) => RunError::Source(error),
        PipelineError::Backtest(error) => RunError::Backtest(error),
        PipelineError::EmptyPeriod => RunError::Report(ReportError::EmptyPeriod {
            period: period_label(config),
        }),
    }
}

fn strategy_value(config: &RunConfig) -> Value {
    json!({
        "name": "axis-agreement",
        "thesis": config.strategy.thesis,
        "timeframe_secs": config.strategy.engine.timeframe_secs(),
        "long_threshold": config.strategy.engine.long_threshold(),
        "short_threshold": config.strategy.engine.short_threshold(),
        "stop_atr_multiple": config.strategy.stop_atr_multiple,
        "reward_to_risk": config.strategy.reward_to_risk,
        "target_atr_multiple": config.strategy.target_atr_multiple(),
    })
}

fn policy_value(config: &RunConfig) -> Value {
    json!({
        "account": {
            "currency": config.policy.account.currency,
            "initial_cash": config.policy.account.initial_cash,
        },
        "objective": {
            "performance_target": config.policy.objective.performance_target,
            "evaluation_days": config.policy.objective.evaluation_days,
            "max_drawdown": config.policy.objective.max_drawdown,
        },
        "risk": {
            "max_position_fraction": config.policy.risk.max_position_fraction,
            "max_leverage": config.policy.risk.max_leverage,
            "max_open_positions": config.policy.risk.max_open_positions,
        },
        "universe": {
            "tickets": config.policy.universe.tickets,
            "timeframes": config.policy.universe.timeframes.iter().map(|timeframe| timeframe.code()).collect::<Vec<_>>(),
        },
    })
}

fn period_label(config: &RunConfig) -> String {
    match (&config.data.from, &config.data.to) {
        (Some(from), Some(to)) => format!("{from}..{to}"),
        (Some(from), None) => format!("{from}..open"),
        (None, Some(to)) => format!("open..{to}"),
        (None, None) => String::from("full file"),
    }
}

fn day_label(timestamp: i64) -> String {
    format_utc_timestamp(timestamp)[..10].to_owned()
}

fn config_files(config: &RunConfig) -> Vec<String> {
    [
        &config.paths.run,
        &config.paths.policy,
        &config.paths.data,
        &config.paths.model,
        &config.paths.strategy,
        &config.paths.execution,
        &config.paths.portfolio,
        &config.paths.analytics,
    ]
    .into_iter()
    .map(|path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("config.json")
            .to_owned()
    })
    .collect()
}

fn artifact_names(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

fn ladder_codes(config: &RunConfig) -> Vec<&'static str> {
    config
        .data
        .ladder
        .iter()
        .map(|timeframe| timeframe.code())
        .collect()
}

fn scale_label(scale: PivotScale) -> &'static str {
    match scale {
        PivotScale::Major => "major",
        PivotScale::Minor => "minor",
    }
}

fn timeframe_label(timeframe_secs: u64) -> &'static str {
    Timeframe::ALL
        .iter()
        .find(|timeframe| u64::from(timeframe.seconds()) == timeframe_secs)
        .map_or("15m", |timeframe| timeframe.code())
}

fn exit_reason_code(reason: ExitReason) -> &'static str {
    match reason {
        ExitReason::AgreementLost => "agreement_lost",
    }
}

fn number(value: Option<f64>) -> String {
    value.map_or_else(String::new, |value| value.to_string())
}

fn json_bytes(value: &Value) -> Result<Vec<u8>, RunError> {
    let text = serde_json::to_string(value)
        .map_err(io::Error::other)
        .map_err(RunError::Output)?;
    Ok(format!("{text}\n").into_bytes())
}

fn emit(writer: &mut impl Write, json_stdout: bool, bundle: &BundleOutput) -> Result<(), RunError> {
    if json_stdout {
        writer.write_all(&bundle.json).map_err(RunError::Output)?;
    } else {
        for name in &bundle.names {
            writeln!(writer, "{}", bundle.path.join(name).display()).map_err(RunError::Output)?;
        }
    }
    writer.flush().map_err(RunError::Output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::types::ReportEngine;

    fn arguments(tokens: &[&str]) -> Vec<String> {
        tokens.iter().map(|token| (*token).to_owned()).collect()
    }

    #[test]
    fn parses_the_report_command() {
        let args = parse_report_arguments(&arguments(&[
            "strategy",
            "--config",
            "config/run.json",
            "--out",
            "out",
            "--json",
        ]))
        .expect("the arguments are valid");
        assert_eq!(args.engine, ReportEngine::Strategy);
        assert_eq!(args.config, PathBuf::from("config/run.json"));
        assert_eq!(args.out, Some(PathBuf::from("out")));
        assert!(args.json);

        assert!(parse_report_arguments(&arguments(&[])).is_err());
        assert!(parse_report_arguments(&arguments(&["model"])).is_err());
        assert!(
            parse_report_arguments(&arguments(&["model", "--config", "a", "--config", "b",]))
                .is_err()
        );
        assert!(parse_report_arguments(&arguments(&["weekly", "--config", "a",])).is_err());
    }

    #[test]
    fn the_decisions_csv_is_the_fixed_shape() {
        let entry_bar = StrategyInput::new(900, 7, 1_735_695_900, Some(0.2), Some(0.3), Some(1.0));
        let exit_bar = StrategyInput::new(900, 9, 1_735_697_700, Some(0.0), Some(0.0), Some(0.0));
        let inputs = vec![entry_bar, entry_bar, exit_bar];
        let decisions = vec![
            Decision::Enter(quantrule_backtest::strategy::Entry {
                side: Side::Long,
                bar: entry_bar,
            }),
            Decision::Hold,
            Decision::Exit(quantrule_backtest::strategy::Exit {
                side: Side::Long,
                bar: exit_bar,
                reason: ExitReason::AgreementLost,
            }),
        ];

        let csv = decisions_csv(&inputs, &decisions, 900);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 3);
        assert_eq!(
            lines[0],
            "type,side,timeframe,timeframe_secs,bar,datetime,direction,momentum_force,structure_score,reason"
        );
        assert_eq!(
            lines[1],
            "entry,1,15m,900,7,2025-01-01T01:45:00Z,0.2,0.3,1,"
        );
        assert_eq!(
            lines[2],
            "exit,1,15m,900,9,2025-01-01T02:15:00Z,0,0,0,agreement_lost"
        );
    }
}
