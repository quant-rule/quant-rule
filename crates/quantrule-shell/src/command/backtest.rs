//! The `backtest` command family.
//!
//! `backtest run` is the coupled model, strategy, execution, and portfolio
//! runner. The parser owns the argument grammar; the stage orchestration lives
//! in `crate::backtest`.

use std::path::PathBuf;

use super::types::{Command, ParseError};
use crate::backtest;
use crate::request::BacktestRunArgs;

const USAGE: &str =
    "usage: backtest run --config <run.json> --strategy <strategy.json> [--out <dir>] [--json]";

/// Parses the already-split arguments after `backtest`.
///
/// # Errors
///
/// Returns [`ParseError::Backtest`] for a missing engine, a repeated or missing
/// option, an unknown option, or an extra positional argument.
pub(crate) fn parse_backtest_arguments(arguments: &[String]) -> Result<Command, ParseError> {
    let Some((engine, rest)) = arguments.split_first() else {
        return Err(ParseError::Backtest(format!(
            "missing backtest engine\n{USAGE}"
        )));
    };
    if engine != "run" {
        return Err(ParseError::Backtest(format!(
            "unknown backtest engine '{engine}'; use run"
        )));
    }

    let mut config = None;
    let mut strategy = None;
    let mut out = None;
    let mut json = false;
    let mut index = 0;
    while index < rest.len() {
        match rest[index].as_str() {
            "--config" => {
                if config.is_some() {
                    return Err(ParseError::Backtest(format!(
                        "--config was supplied twice\n{USAGE}"
                    )));
                }
                config = Some(next_path(rest, &mut index, "--config")?);
            }
            "--strategy" => {
                if strategy.is_some() {
                    return Err(ParseError::Backtest(format!(
                        "--strategy was supplied twice\n{USAGE}"
                    )));
                }
                strategy = Some(next_path(rest, &mut index, "--strategy")?);
            }
            "--out" => {
                if out.is_some() {
                    return Err(ParseError::Backtest(format!(
                        "--out was supplied twice\n{USAGE}"
                    )));
                }
                out = Some(next_path(rest, &mut index, "--out")?);
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

    Ok(Command::BacktestRun(BacktestRunArgs {
        config: config.ok_or_else(|| missing("--config"))?,
        strategy: strategy.ok_or_else(|| missing("--strategy"))?,
        out,
        json,
    }))
}

fn next_path(arguments: &[String], index: &mut usize, option: &str) -> Result<PathBuf, ParseError> {
    *index += 1;
    arguments
        .get(*index)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| ParseError::Backtest(format!("{option} requires a non-empty path\n{USAGE}")))
}

fn missing(option: &str) -> ParseError {
    ParseError::Backtest(format!("{option} is required\n{USAGE}"))
}

/// Runs `backtest run`.
///
/// # Errors
///
/// Returns [`super::types::RunError::Backtest`] when the model, strategy,
/// execution, portfolio, manifest, or writer fails.
pub(crate) fn run_backtest(
    args: &BacktestRunArgs,
    writer: &mut impl std::io::Write,
) -> Result<(), super::types::RunError> {
    backtest::run(args, writer).map_err(super::types::RunError::Backtest)
}

#[cfg(test)]
mod tests {
    use super::parse_backtest_arguments;
    use crate::command::types::Command;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn run_parses_the_config_and_strategy() {
        let command = parse_backtest_arguments(&args(&[
            "run",
            "--config",
            "run.json",
            "--strategy",
            "strategy.json",
            "--out",
            "target",
            "--json",
        ]))
        .expect("the command is valid");
        assert!(matches!(command, Command::BacktestRun(_)));
    }

    #[test]
    fn a_missing_strategy_path_is_rejected() {
        let error = parse_backtest_arguments(&args(&["run", "--config", "run.json"]))
            .expect_err("the strategy path is missing");
        assert!(error.to_string().contains("--strategy"));
    }

    #[test]
    fn an_unknown_engine_is_rejected() {
        let error =
            parse_backtest_arguments(&args(&["replay"])).expect_err("the engine is unknown");
        assert!(error.to_string().contains("unknown backtest engine"));
    }
}
