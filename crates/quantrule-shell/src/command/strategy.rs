//! The `strategy` command family.
//!
//! `strategy explain` prints one decision and its as-of evidence. `strategy
//! run` writes the decisions bundle for one model bundle. The parser owns the
//! argument grammar; the stage implementation lives in `crate::strategy`.

use std::path::PathBuf;

use quantrule_backtest::intent::StrategyPosition;
use quantrule_backtest::side::Side;

use super::types::{Command, ParseError, StrategyExplainArgs, StrategyRunArgs};
use crate::strategy;

const EXPLAIN_USAGE: &str = "usage: strategy explain --model <model-bundle> --strategy <strategy.json> --bar <index> [--position flat|long|short]";
const RUN_USAGE: &str =
    "usage: strategy run --model <model-bundle> --strategy <strategy.json> [--out <dir>] [--json]";

/// Parses the already-split arguments after `strategy`.
///
/// # Errors
///
/// Returns [`ParseError::Strategy`] for a missing engine, a repeated or missing
/// option, an unknown option, an invalid bar, or an invalid position.
pub(crate) fn parse_strategy_arguments(arguments: &[String]) -> Result<Command, ParseError> {
    let Some((engine, rest)) = arguments.split_first() else {
        return Err(ParseError::Strategy(format!(
            "missing strategy engine\n{EXPLAIN_USAGE}"
        )));
    };
    match engine.as_str() {
        "explain" => parse_explain(rest),
        "run" => parse_run(rest),
        other => Err(ParseError::Strategy(format!(
            "unknown strategy engine '{other}'; use explain or run"
        ))),
    }
}

fn parse_explain(arguments: &[String]) -> Result<Command, ParseError> {
    let mut model = None;
    let mut strategy = None;
    let mut bar = None;
    let mut position = StrategyPosition::flat();
    let mut position_set = false;
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--model" => {
                if model.is_some() {
                    return Err(ParseError::Strategy(format!(
                        "--model was supplied twice\n{EXPLAIN_USAGE}"
                    )));
                }
                model = Some(next_path(arguments, &mut index, "--model", EXPLAIN_USAGE)?);
            }
            "--strategy" => {
                if strategy.is_some() {
                    return Err(ParseError::Strategy(format!(
                        "--strategy was supplied twice\n{EXPLAIN_USAGE}"
                    )));
                }
                strategy = Some(next_path(
                    arguments,
                    &mut index,
                    "--strategy",
                    EXPLAIN_USAGE,
                )?);
            }
            "--bar" => {
                if bar.is_some() {
                    return Err(ParseError::Strategy(format!(
                        "--bar was supplied twice\n{EXPLAIN_USAGE}"
                    )));
                }
                let value = next_value(arguments, &mut index, "--bar", EXPLAIN_USAGE)?;
                bar = Some(value.parse::<usize>().map_err(|_| {
                    ParseError::Strategy(format!(
                        "--bar expects a non-negative integer\n{EXPLAIN_USAGE}"
                    ))
                })?);
            }
            "--position" => {
                if position_set {
                    return Err(ParseError::Strategy(format!(
                        "--position was supplied twice\n{EXPLAIN_USAGE}"
                    )));
                }
                let value = next_value(arguments, &mut index, "--position", EXPLAIN_USAGE)?;
                position = parse_position(value)?;
                position_set = true;
            }
            option if option.starts_with("--") => {
                return Err(ParseError::Strategy(format!(
                    "unknown option '{option}'\n{EXPLAIN_USAGE}"
                )));
            }
            value => {
                return Err(ParseError::Strategy(format!(
                    "unexpected argument '{value}'\n{EXPLAIN_USAGE}"
                )));
            }
        }
        index += 1;
    }
    Ok(Command::StrategyExplain(StrategyExplainArgs {
        model: model.ok_or_else(|| missing("--model", EXPLAIN_USAGE))?,
        strategy: strategy.ok_or_else(|| missing("--strategy", EXPLAIN_USAGE))?,
        bar: bar.ok_or_else(|| missing("--bar", EXPLAIN_USAGE))?,
        position,
    }))
}

fn parse_run(arguments: &[String]) -> Result<Command, ParseError> {
    let mut model = None;
    let mut strategy = None;
    let mut out = None;
    let mut json = false;
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--model" => {
                if model.is_some() {
                    return Err(ParseError::Strategy(format!(
                        "--model was supplied twice\n{RUN_USAGE}"
                    )));
                }
                model = Some(next_path(arguments, &mut index, "--model", RUN_USAGE)?);
            }
            "--strategy" => {
                if strategy.is_some() {
                    return Err(ParseError::Strategy(format!(
                        "--strategy was supplied twice\n{RUN_USAGE}"
                    )));
                }
                strategy = Some(next_path(arguments, &mut index, "--strategy", RUN_USAGE)?);
            }
            "--out" => {
                if out.is_some() {
                    return Err(ParseError::Strategy(format!(
                        "--out was supplied twice\n{RUN_USAGE}"
                    )));
                }
                out = Some(next_path(arguments, &mut index, "--out", RUN_USAGE)?);
            }
            "--json" => {
                if json {
                    return Err(ParseError::Strategy(format!(
                        "--json was supplied twice\n{RUN_USAGE}"
                    )));
                }
                json = true;
            }
            option if option.starts_with("--") => {
                return Err(ParseError::Strategy(format!(
                    "unknown option '{option}'\n{RUN_USAGE}"
                )));
            }
            value => {
                return Err(ParseError::Strategy(format!(
                    "unexpected argument '{value}'\n{RUN_USAGE}"
                )));
            }
        }
        index += 1;
    }
    Ok(Command::StrategyRun(StrategyRunArgs {
        model: model.ok_or_else(|| missing("--model", RUN_USAGE))?,
        strategy: strategy.ok_or_else(|| missing("--strategy", RUN_USAGE))?,
        out,
        json,
    }))
}

fn next_value<'a>(
    arguments: &'a [String],
    index: &mut usize,
    option: &str,
    usage: &str,
) -> Result<&'a str, ParseError> {
    *index += 1;
    arguments
        .get(*index)
        .filter(|value| !value.is_empty())
        .map(String::as_str)
        .ok_or_else(|| {
            ParseError::Strategy(format!("{option} requires a non-empty value\n{usage}"))
        })
}

fn next_path(
    arguments: &[String],
    index: &mut usize,
    option: &str,
    usage: &str,
) -> Result<PathBuf, ParseError> {
    next_value(arguments, index, option, usage).map(PathBuf::from)
}

fn parse_position(value: &str) -> Result<StrategyPosition, ParseError> {
    match value {
        "flat" => Ok(StrategyPosition::flat()),
        "long" => Ok(StrategyPosition::with_side(Side::Long)),
        "short" => Ok(StrategyPosition::with_side(Side::Short)),
        other => Err(ParseError::Strategy(format!(
            "--position expects flat, long, or short, found '{other}'\n{EXPLAIN_USAGE}"
        ))),
    }
}

fn missing(option: &str, usage: &str) -> ParseError {
    ParseError::Strategy(format!("{option} is required\n{usage}"))
}

/// Runs `strategy explain`.
///
/// # Errors
///
/// Returns [`super::types::RunError::Strategy`] when the strategy or model
/// bundle is invalid or the writer fails.
pub(crate) fn run_explain(
    args: &StrategyExplainArgs,
    writer: &mut impl std::io::Write,
) -> Result<(), super::types::RunError> {
    strategy::explain(args, writer).map_err(super::types::RunError::Strategy)
}

/// Runs `strategy run`.
///
/// # Errors
///
/// Returns [`super::types::RunError::Strategy`] when the strategy or model
/// bundle is invalid, the output target is unavailable, or the writer fails.
pub(crate) fn run_run(
    args: &StrategyRunArgs,
    writer: &mut impl std::io::Write,
) -> Result<(), super::types::RunError> {
    strategy::run(args, writer).map_err(super::types::RunError::Strategy)
}

#[cfg(test)]
mod tests {
    use super::parse_strategy_arguments;
    use crate::command::types::Command;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn explain_parses_the_model_strategy_and_bar() {
        let command = parse_strategy_arguments(&args(&[
            "explain",
            "--model",
            "target/model",
            "--strategy",
            "strategy.json",
            "--bar",
            "12",
            "--position",
            "long",
        ]))
        .expect("the command is valid");
        assert!(matches!(command, Command::StrategyExplain(_)));
    }

    #[test]
    fn run_parses_the_model_and_strategy() {
        let command = parse_strategy_arguments(&args(&[
            "run",
            "--model",
            "target/model",
            "--strategy",
            "strategy.json",
            "--out",
            "target",
            "--json",
        ]))
        .expect("the command is valid");
        assert!(matches!(command, Command::StrategyRun(_)));
    }

    #[test]
    fn a_missing_bar_is_rejected() {
        let error = parse_strategy_arguments(&args(&[
            "explain",
            "--model",
            "target/model",
            "--strategy",
            "strategy.json",
        ]))
        .expect_err("the bar is missing");
        assert!(error.to_string().contains("--bar"));
    }

    #[test]
    fn an_invalid_position_is_rejected() {
        let error = parse_strategy_arguments(&args(&[
            "explain",
            "--model",
            "target/model",
            "--strategy",
            "strategy.json",
            "--bar",
            "0",
            "--position",
            "sideways",
        ]))
        .expect_err("the position is invalid");
        assert!(error.to_string().contains("flat, long, or short"));
    }
}
