//! The `analytics` command family.
//!
//! `analytics evaluate` publishes descriptive metrics and evaluation tables from
//! a backtest bundle. The parser owns the argument grammar; the stage
//! implementation lives in `crate::analytics`.

use std::path::PathBuf;

use super::types::{AnalyticsEvaluateArgs, Command, ParseError};
use crate::analytics;

const USAGE: &str = "usage: analytics evaluate --evidence <backtest-bundle> [--out <dir>] [--json]";

/// Parses the already-split arguments after `analytics`.
///
/// # Errors
///
/// Returns [`ParseError::Analytics`] for a missing or unknown engine, a repeated
/// or missing option, an unknown option, or an extra positional argument.
pub(crate) fn parse_analytics_arguments(arguments: &[String]) -> Result<Command, ParseError> {
    let Some((engine, rest)) = arguments.split_first() else {
        return Err(ParseError::Analytics(format!(
            "missing analytics engine\n{USAGE}"
        )));
    };
    if engine != "evaluate" {
        return Err(ParseError::Analytics(format!(
            "unknown analytics engine '{engine}'; use evaluate"
        )));
    }

    let mut evidence = None;
    let mut out = None;
    let mut json = false;
    let mut index = 0;
    while index < rest.len() {
        match rest[index].as_str() {
            "--evidence" => {
                if evidence.is_some() {
                    return Err(ParseError::Analytics(format!(
                        "--evidence was supplied twice\n{USAGE}"
                    )));
                }
                evidence = Some(next_path(rest, &mut index, "--evidence")?);
            }
            "--out" => {
                if out.is_some() {
                    return Err(ParseError::Analytics(format!(
                        "--out was supplied twice\n{USAGE}"
                    )));
                }
                out = Some(next_path(rest, &mut index, "--out")?);
            }
            "--json" => {
                if json {
                    return Err(ParseError::Analytics(format!(
                        "--json was supplied twice\n{USAGE}"
                    )));
                }
                json = true;
            }
            option if option.starts_with("--") => {
                return Err(ParseError::Analytics(format!(
                    "unknown option '{option}'\n{USAGE}"
                )));
            }
            value => {
                return Err(ParseError::Analytics(format!(
                    "unexpected argument '{value}'\n{USAGE}"
                )));
            }
        }
        index += 1;
    }

    Ok(Command::AnalyticsEvaluate(AnalyticsEvaluateArgs {
        evidence: evidence.ok_or_else(|| missing("--evidence"))?,
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
            ParseError::Analytics(format!("{option} requires a non-empty path\n{USAGE}"))
        })
}

fn missing(option: &str) -> ParseError {
    ParseError::Analytics(format!("{option} is required\n{USAGE}"))
}

/// Runs `analytics evaluate`.
///
/// # Errors
///
/// Returns [`super::types::RunError::Analytics`] when the evidence bundle, the
/// output target, or the writer fails.
pub(crate) fn run_analytics(
    args: &AnalyticsEvaluateArgs,
    writer: &mut impl std::io::Write,
) -> Result<(), super::types::RunError> {
    analytics::run(args, writer).map_err(super::types::RunError::Analytics)
}

#[cfg(test)]
mod tests {
    use super::parse_analytics_arguments;
    use crate::command::types::Command;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn evaluate_parses_the_evidence_and_output() {
        let command = parse_analytics_arguments(&args(&[
            "evaluate",
            "--evidence",
            "target/backtest",
            "--out",
            "target",
            "--json",
        ]))
        .expect("the command is valid");
        assert!(matches!(command, Command::AnalyticsEvaluate(_)));
    }

    #[test]
    fn a_missing_evidence_path_is_rejected() {
        let error = parse_analytics_arguments(&args(&["evaluate"]))
            .expect_err("the evidence path is missing");
        assert!(error.to_string().contains("--evidence"));
    }

    #[test]
    fn an_unknown_engine_is_rejected() {
        let error =
            parse_analytics_arguments(&args(&["report"])).expect_err("the engine is unknown");
        assert!(error.to_string().contains("unknown analytics engine"));
    }
}
