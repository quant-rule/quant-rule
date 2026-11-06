//! The `execution` command family.
//!
//! `execution run` turns a strategy decisions bundle into orders, fills, and
//! rejections. The parser owns the argument grammar; the stage implementation
//! lives in `crate::execution`.

use std::path::PathBuf;

use super::types::{Command, ParseError};
use crate::execution;
use crate::request::ExecutionRunArgs;

const USAGE: &str =
    "usage: execution run --decisions <strategy-bundle> --config <run.json> [--out <dir>] [--json]";

/// Parses the already-split arguments after `execution`.
///
/// # Errors
///
/// Returns [`ParseError::Execution`] for a missing engine, a repeated or missing
/// option, an unknown option, or an extra positional argument.
pub(crate) fn parse_execution_arguments(arguments: &[String]) -> Result<Command, ParseError> {
    let Some((engine, rest)) = arguments.split_first() else {
        return Err(ParseError::Execution(format!(
            "missing execution engine\n{USAGE}"
        )));
    };
    if engine != "run" {
        return Err(ParseError::Execution(format!(
            "unknown execution engine '{engine}'; use run"
        )));
    }

    let mut decisions = None;
    let mut config = None;
    let mut out = None;
    let mut json = false;
    let mut index = 0;
    while index < rest.len() {
        match rest[index].as_str() {
            "--decisions" => {
                if decisions.is_some() {
                    return Err(ParseError::Execution(format!(
                        "--decisions was supplied twice\n{USAGE}"
                    )));
                }
                decisions = Some(next_path(rest, &mut index, "--decisions")?);
            }
            "--config" => {
                if config.is_some() {
                    return Err(ParseError::Execution(format!(
                        "--config was supplied twice\n{USAGE}"
                    )));
                }
                config = Some(next_path(rest, &mut index, "--config")?);
            }
            "--out" => {
                if out.is_some() {
                    return Err(ParseError::Execution(format!(
                        "--out was supplied twice\n{USAGE}"
                    )));
                }
                out = Some(next_path(rest, &mut index, "--out")?);
            }
            "--json" => {
                if json {
                    return Err(ParseError::Execution(format!(
                        "--json was supplied twice\n{USAGE}"
                    )));
                }
                json = true;
            }
            option if option.starts_with("--") => {
                return Err(ParseError::Execution(format!(
                    "unknown option '{option}'\n{USAGE}"
                )));
            }
            value => {
                return Err(ParseError::Execution(format!(
                    "unexpected argument '{value}'\n{USAGE}"
                )));
            }
        }
        index += 1;
    }

    Ok(Command::ExecutionRun(ExecutionRunArgs {
        decisions: decisions.ok_or_else(|| missing("--decisions"))?,
        config: config.ok_or_else(|| missing("--config"))?,
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
        .ok_or_else(|| {
            ParseError::Execution(format!("{option} requires a non-empty path\n{USAGE}"))
        })
}

fn missing(option: &str) -> ParseError {
    ParseError::Execution(format!("{option} is required\n{USAGE}"))
}

/// Runs `execution run`.
///
/// # Errors
///
/// Returns [`super::types::RunError::Execution`] when the decisions bundle, the
/// run config, the candle series, or the output target fails.
pub(crate) fn run_execution(
    args: &ExecutionRunArgs,
    writer: &mut impl std::io::Write,
) -> Result<(), super::types::RunError> {
    execution::run(args, writer).map_err(super::types::RunError::Execution)
}

#[cfg(test)]
mod tests {
    use super::parse_execution_arguments;
    use crate::command::types::Command;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn run_parses_the_decisions_and_config() {
        let command = parse_execution_arguments(&args(&[
            "run",
            "--decisions",
            "target/report/strategy",
            "--config",
            "run.json",
            "--out",
            "target/report",
            "--json",
        ]))
        .expect("the command is valid");
        assert!(matches!(command, Command::ExecutionRun(_)));
    }

    #[test]
    fn a_missing_decisions_path_is_rejected() {
        let error = parse_execution_arguments(&args(&["run", "--config", "run.json"]))
            .expect_err("the decisions path is missing");
        assert!(error.to_string().contains("--decisions"));
    }

    #[test]
    fn an_unknown_engine_is_rejected() {
        let error =
            parse_execution_arguments(&args(&["replay"])).expect_err("the engine is unknown");
        assert!(error.to_string().contains("unknown execution engine"));
    }
}
