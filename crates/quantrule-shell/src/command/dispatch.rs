//! The command boundary: one argv parser and one executor.
//!
//! Interactive input and OS argv both resolve to the same [`Command`]. The
//! executor owns the error-to-exit-code policy, so adding a command touches one
//! dispatcher instead of every entry point. It performs no domain computation.

use std::io::Write;
use std::path::PathBuf;

use super::{
    Command, ParseError, run_candles, run_candles_tolerant, run_events, run_execution, run_explain,
    run_fetch, run_help, run_policy_check, run_report, run_run, run_timeframes,
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
        "events" => {
            super::parse_model_arguments(rest, "events").map(|args| Some(Command::Events(args)))
        }
        "report" => super::parse_report_arguments(rest).map(|args| Some(Command::Report(args))),
        "policy" => super::parse_policy_arguments(rest).map(Some),
        "strategy" => super::parse_strategy_arguments(rest).map(Some),
        "execution" => super::parse_execution_arguments(rest).map(Some),
        "fetch" => super::parse_fetch_arguments(rest).map(|args| Some(Command::Fetch(args))),
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
        Command::Events(args) => run_events(&args, stdout),
        Command::Report(args) => run_report(&args, stdout),
        Command::PolicyCheck(args) => run_policy_check(&args, stdout),
        Command::StrategyExplain(args) => run_explain(&args, stdout),
        Command::StrategyRun(args) => run_run(&args, stdout),
        Command::ExecutionRun(args) => run_execution(&args, stdout),
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
