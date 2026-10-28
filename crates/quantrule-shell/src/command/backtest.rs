//! The `backtest` command: one full run, all five bundles.
//!
//! The command loads the run, loads the candles once, calls `run_backtest`
//! once, and writes the five bundles from that result. The model, strategy, and
//! analytical bundles use the run's model; the analytical layers are built from
//! the same model run, so neither recomputes it. A failure removes every bundle
//! written and the run root when the command created it.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use quantrule_backtest::runner::BacktestRun;
use serde_json::{Map, Value, json};

use super::report_run::{self, BundleOutput};
use super::types::{BacktestArgs, ParseError, RunError};
use crate::config::RunConfig;
use crate::render_report;

/// The five bundle names, in contract order.
const BUNDLES: [&str; 5] = ["model", "strategy", "execution", "portfolio", "analytical"];

/// The five written bundles and their names.
type BundleList = Vec<(&'static str, BundleOutput)>;

/// Parses the already-split arguments of `backtest`.
///
/// # Errors
///
/// Returns [`ParseError::Backtest`] for a missing or repeated `--config`/
/// `--out`, an unknown option, or an extra positional argument.
pub(crate) fn parse_backtest_arguments(arguments: &[String]) -> Result<BacktestArgs, ParseError> {
    const USAGE: &str = "usage: backtest --config <run.json> [--out <dir>] [--json]";
    let mut config = None;
    let mut out = None;
    let mut json = false;
    let mut index = 0;

    while index < arguments.len() {
        match arguments[index].as_str() {
            "--config" => {
                if config.is_some() {
                    return Err(ParseError::Backtest(format!(
                        "--config was supplied twice\n{USAGE}"
                    )));
                }
                index += 1;
                let value = arguments
                    .get(index)
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| {
                        ParseError::Backtest(format!("--config requires a non-empty path\n{USAGE}"))
                    })?;
                config = Some(PathBuf::from(value));
            }
            "--out" => {
                if out.is_some() {
                    return Err(ParseError::Backtest(format!(
                        "--out was supplied twice\n{USAGE}"
                    )));
                }
                index += 1;
                let value = arguments
                    .get(index)
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| {
                        ParseError::Backtest(format!("--out requires a non-empty path\n{USAGE}"))
                    })?;
                out = Some(PathBuf::from(value));
            }
            "--json" => {
                if json {
                    return Err(ParseError::Backtest(format!(
                        "--json was supplied twice\n{USAGE}"
                    )));
                }
                json = true;
            }
            option if option.starts_with("--") => {
                return Err(ParseError::Backtest(format!(
                    "unknown option '{option}'\n{USAGE}"
                )));
            }
            value => {
                return Err(ParseError::Backtest(format!(
                    "unexpected argument '{value}'\n{USAGE}"
                )));
            }
        }
        index += 1;
    }

    let config =
        config.ok_or_else(|| ParseError::Backtest(format!("--config is required\n{USAGE}")))?;

    Ok(BacktestArgs { config, out, json })
}

/// Runs the pipeline once and writes the five bundles.
///
/// # Errors
///
/// Returns a [`RunError`] when the configuration, the source, the pipeline, a
/// bundle, or the writer fails. A failed run leaves no bundle and no run root
/// the command created.
pub(crate) fn run_backtest_command(
    args: &BacktestArgs,
    writer: &mut impl Write,
) -> Result<(), RunError> {
    let config = RunConfig::load(&args.config).map_err(RunError::Config)?;
    let out = args
        .out
        .clone()
        .unwrap_or_else(|| config.output_root.clone());
    render_report::ensure_available(&out).map_err(RunError::Bundle)?;
    let created = !out.exists();

    match execute(&config, &out) {
        Ok((bundles, run)) => output(writer, args.json, &config, &bundles, &run),
        Err(error) => {
            cleanup(&out, created);
            Err(error)
        }
    }
}

fn execute(config: &RunConfig, out: &Path) -> Result<(BundleList, BacktestRun), RunError> {
    let run = report_run::run_pipeline(config)?;
    let model = run.model_run();
    let mut bundles = Vec::with_capacity(BUNDLES.len());
    bundles.push(("model", report_run::model_bundle_from(config, out, model)?));
    bundles.push((
        "strategy",
        report_run::strategy_bundle_from(config, out, model)?,
    ));
    bundles.push((
        "execution",
        report_run::bundles::execution_bundle_from(config, out, &run)?,
    ));
    bundles.push((
        "portfolio",
        report_run::bundles::portfolio_bundle_from(config, out, &run)?,
    ));
    bundles.push((
        "analytical",
        report_run::bundles::analytical_bundle_from(config, out, model)?,
    ));
    Ok((bundles, run))
}

fn output(
    writer: &mut impl Write,
    json_stdout: bool,
    config: &RunConfig,
    bundles: &[(&'static str, BundleOutput)],
    run: &BacktestRun,
) -> Result<(), RunError> {
    if json_stdout {
        let value = summary(config, bundles, run);
        let text = serde_json::to_string(&value)
            .map_err(std::io::Error::other)
            .map_err(RunError::Output)?;
        writeln!(writer, "{text}").map_err(RunError::Output)?;
    } else {
        for (_, bundle) in bundles {
            writeln!(writer, "{}", bundle.path.display()).map_err(RunError::Output)?;
        }
    }
    writer.flush().map_err(RunError::Output)
}

fn summary(
    config: &RunConfig,
    bundles: &[(&'static str, BundleOutput)],
    run: &BacktestRun,
) -> Value {
    let mut paths = Map::new();
    for (name, bundle) in bundles {
        paths.insert(
            (*name).to_owned(),
            Value::String(bundle.path.display().to_string()),
        );
    }
    let metrics = run.metrics();
    let timeframe = run.model_run().timeframe(config.strategy.timeframe_secs());
    let first = timeframe
        .and_then(|timeframe| timeframe.candle(0))
        .map(|candle| candle.timestamp());
    let last = timeframe
        .and_then(|timeframe| timeframe.candle(timeframe.len().saturating_sub(1)))
        .map(|candle| candle.timestamp());
    json!({
        "report": "backtest",
        "report_version": render_report::REPORT_VERSION,
        "model_version": env!("CARGO_PKG_VERSION"),
        "data_version": config.data.data_version,
        "requested_period": {"from": config.data.from, "to": config.data.to},
        "actual_period": {"first_open_time": first, "last_open_time": last},
        "bundles": paths,
        "portfolio": {
            "final_equity": metrics.final_equity,
            "total_return": metrics.total_return,
            "net_pnl": metrics.net_pnl,
            "max_drawdown": metrics.max_drawdown,
            "trades": metrics.trades,
            "halted": metrics.halted,
        },
    })
}

fn cleanup(out: &Path, created: bool) {
    for name in BUNDLES {
        let _ = fs::remove_dir_all(out.join(name));
    }
    if created {
        let _ = fs::remove_dir(out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arguments(tokens: &[&str]) -> Vec<String> {
        tokens.iter().map(|token| (*token).to_owned()).collect()
    }

    #[test]
    fn parses_the_backtest_command() {
        let args = parse_backtest_arguments(&arguments(&[
            "--config",
            "config/run.json",
            "--out",
            "out",
            "--json",
        ]))
        .expect("the arguments are valid");
        assert_eq!(args.config, PathBuf::from("config/run.json"));
        assert_eq!(args.out, Some(PathBuf::from("out")));
        assert!(args.json);

        assert!(parse_backtest_arguments(&arguments(&[])).is_err());
        assert!(
            parse_backtest_arguments(&arguments(&["--config", "a", "--config", "b",])).is_err()
        );
        assert!(parse_backtest_arguments(&arguments(&["--config", "a", "extra"])).is_err());
    }
}
