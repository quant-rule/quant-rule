//! The command boundary: one argv parser and one executor.
//!
//! Interactive input and OS argv both resolve to the same [`Command`]. The
//! executor owns the error-to-exit-code policy, so adding a command touches one
//! dispatcher instead of every entry point. It performs no domain computation.

use std::io::Write;
use std::path::PathBuf;

use super::{
    Command, ParseError, run_axis, run_backtest_command, run_candles, run_candles_tolerant, run_db,
    run_events, run_fetch, run_help, run_lab_summary, run_legs, run_market_chart,
    run_market_dashboard, run_market_report, run_market_state, run_model_series, run_report,
    run_research_export, run_research_summary, run_store_ingest, run_store_init, run_store_list,
    run_store_remove, run_store_show, run_store_verify, run_strategy_run, run_timeframes,
};

/// Parses the already-split arguments after the binary name.
///
/// This is the non-interactive counterpart of [`super::parse()`]. It accepts the
/// same commands and options; only the token source differs.
///
/// # Errors
///
/// Returns the same parse errors as the interactive parser.
pub(crate) fn parse_argv(arguments: &[String]) -> Result<Option<Command>, ParseError> {
    let Some((name, rest)) = arguments.split_first() else {
        return Ok(None);
    };

    match name.as_str() {
        "candles" => path_command(rest, "candles", Command::Candles),
        "candles-tolerant" => path_command(rest, "candles-tolerant", Command::CandlesTolerant),
        "timeframes" => path_command(rest, "timeframes", Command::Timeframes),
        "market-state" => super::parse_model_arguments(rest, "market-state")
            .map(|args| Some(Command::MarketState(args))),
        "market-dashboard" => {
            super::parse_dashboard_arguments(rest).map(|args| Some(Command::MarketDashboard(args)))
        }
        "events" => {
            super::parse_model_arguments(rest, "events").map(|args| Some(Command::Events(args)))
        }
        "legs" => super::parse_model_arguments(rest, "legs").map(|args| Some(Command::Legs(args))),
        "axis" => {
            super::parse_axis_arguments(rest).map(|(args, axis)| Some(Command::Axis { args, axis }))
        }
        "model-series" => {
            super::parse_model_series_arguments(rest).map(|args| Some(Command::ModelSeries(args)))
        }
        "market-chart" => {
            super::parse_market_chart_arguments(rest).map(|args| Some(Command::MarketChart(args)))
        }
        "market-report" => {
            super::parse_market_report_arguments(rest).map(|args| Some(Command::MarketReport(args)))
        }
        "report" => super::parse_report_arguments(rest).map(|args| Some(Command::Report(args))),
        "backtest" => {
            super::parse_backtest_arguments(rest).map(|args| Some(Command::Backtest(args)))
        }
        "strategy-run" => {
            super::parse_strategy_run_arguments(rest).map(|args| Some(Command::StrategyRun(args)))
        }
        "lab" => super::parse_lab_arguments(rest),
        "fetch" => super::parse_fetch_arguments(rest).map(|args| Some(Command::Fetch(args))),
        "research" => super::parse_research_arguments(rest),
        "store" => super::parse_store_arguments(rest),
        "db" => super::store_query::parse_db_arguments(rest),
        "help" => super::parse_help_arguments(rest).map(|topic| Some(Command::Help(topic))),
        "quit" => {
            if rest.is_empty() {
                Ok(Some(Command::Quit))
            } else {
                Err(ParseError::UnexpectedArguments("quit"))
            }
        }
        other => Err(ParseError::Unknown(other.to_owned())),
    }
}

fn path_command(
    arguments: &[String],
    name: &'static str,
    build: fn(PathBuf) -> Command,
) -> Result<Option<Command>, ParseError> {
    if arguments.len() != 1 {
        return Err(ParseError::Usage(format!("usage: {name} <csv-path>")));
    }
    Ok(Some(build(PathBuf::from(&arguments[0]))))
}

/// Executes one parsed command and returns the process exit code.
///
/// The interactive loop ignores the code and keeps the session open; the
/// non-interactive entry point returns it. Failures are written once to stderr.
pub(crate) fn execute(command: Command, stdout: &mut impl Write, stderr: &mut impl Write) -> i32 {
    let result = match command {
        Command::Candles(path) => run_candles(&path, stdout),
        Command::CandlesTolerant(path) => run_candles_tolerant(&path, stdout, stderr),
        Command::Timeframes(path) => run_timeframes(&path, stdout, stderr),
        Command::MarketState(args) => run_market_state(&args, stdout),
        Command::MarketDashboard(args) => run_market_dashboard(&args, stdout),
        Command::MarketChart(args) => run_market_chart(&args, stdout),
        Command::MarketReport(args) => run_market_report(&args, stdout),
        Command::Report(args) => run_report(&args, stdout),
        Command::Backtest(args) => run_backtest_command(&args, stdout),
        Command::LabSummary(args) => run_lab_summary(&args, stdout),
        Command::StrategyRun(args) => run_strategy_run(&args, stdout),
        Command::Axis { args, axis } => run_axis(&args, axis, stdout),
        Command::Events(args) => run_events(&args, stdout),
        Command::Legs(args) => run_legs(&args, stdout),
        Command::ModelSeries(args) => run_model_series(&args, stdout),
        Command::ResearchExport(args) => run_research_export(&args, stdout),
        Command::ResearchSummary(path) => run_research_summary(&path, stdout),
        Command::StoreInit(path) => run_store_init(&path, stdout),
        Command::StoreIngest(args) => run_store_ingest(&args, stdout),
        Command::StoreVerify(args) => run_store_verify(&args, stdout),
        Command::StoreList(args) => run_store_list(&args, stdout),
        Command::StoreShow(args) => run_store_show(&args, stdout),
        Command::StoreRemove(args) => run_store_remove(&args, stdout),
        Command::Db(args) => run_db(&args, stdout),
        Command::Fetch(args) => run_fetch(&args, stdout, stderr),
        Command::Help(topic) => run_help(&topic, stdout),
        Command::Quit => return 0,
    };

    match result {
        Ok(()) => 0,
        Err(error) => {
            let _ = writeln!(stderr, "error: {error}");
            1
        }
    }
}
