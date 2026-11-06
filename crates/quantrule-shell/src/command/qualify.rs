//! The `qualify` command family.
//!
//! `qualify` evaluates a backtest evidence bundle against human-owned criteria.
//! The parser owns the argument grammar; the stage implementation lives in
//! `crate::qualification`.

use std::path::PathBuf;

use super::types::{Command, ParseError};
use crate::qualification;
use crate::request::QualifyArgs;

const USAGE: &str = "usage: qualify --evidence <backtest-bundle> --criteria <qualification.json> [--out <dir>] [--json]";

/// Parses the already-split arguments after `qualify`.
///
/// # Errors
///
/// Returns [`ParseError::Qualification`] for a repeated or missing option, an
/// unknown option, or an extra positional argument.
pub(crate) fn parse_qualify_arguments(arguments: &[String]) -> Result<Command, ParseError> {
    let mut evidence = None;
    let mut criteria = None;
    let mut out = None;
    let mut json = false;
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--evidence" => {
                if evidence.is_some() {
                    return Err(ParseError::Qualification(format!(
                        "--evidence was supplied twice\n{USAGE}"
                    )));
                }
                evidence = Some(next_path(arguments, &mut index, "--evidence")?);
            }
            "--criteria" => {
                if criteria.is_some() {
                    return Err(ParseError::Qualification(format!(
                        "--criteria was supplied twice\n{USAGE}"
                    )));
                }
                criteria = Some(next_path(arguments, &mut index, "--criteria")?);
            }
            "--out" => {
                if out.is_some() {
                    return Err(ParseError::Qualification(format!(
                        "--out was supplied twice\n{USAGE}"
                    )));
                }
                out = Some(next_path(arguments, &mut index, "--out")?);
            }
            "--json" => {
                if json {
                    return Err(ParseError::Qualification(format!(
                        "--json was supplied twice\n{USAGE}"
                    )));
                }
                json = true;
            }
            option if option.starts_with("--") => {
                return Err(ParseError::Qualification(format!(
                    "unknown option '{option}'\n{USAGE}"
                )));
            }
            value => {
                return Err(ParseError::Qualification(format!(
                    "unexpected argument '{value}'\n{USAGE}"
                )));
            }
        }
        index += 1;
    }

    Ok(Command::Qualify(QualifyArgs {
        evidence: evidence.ok_or_else(|| missing("--evidence"))?,
        criteria: criteria.ok_or_else(|| missing("--criteria"))?,
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
            ParseError::Qualification(format!("{option} requires a non-empty path\n{USAGE}"))
        })
}

fn missing(option: &str) -> ParseError {
    ParseError::Qualification(format!("{option} is required\n{USAGE}"))
}

/// Runs `qualify`.
///
/// # Errors
///
/// Returns [`super::types::RunError::Qualification`] when the criteria, the
/// evidence bundle, the output target, or the writer fails.
pub(crate) fn run_qualify(
    args: &QualifyArgs,
    writer: &mut impl std::io::Write,
) -> Result<(), super::types::RunError> {
    qualification::run(args, writer).map_err(super::types::RunError::Qualification)
}

#[cfg(test)]
mod tests {
    use super::parse_qualify_arguments;
    use crate::command::types::Command;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn qualify_parses_the_evidence_and_criteria() {
        let command = parse_qualify_arguments(&args(&[
            "--evidence",
            "target/backtest",
            "--criteria",
            "qualification.json",
            "--out",
            "target",
            "--json",
        ]))
        .expect("the command is valid");
        assert!(matches!(command, Command::Qualify(_)));
    }

    #[test]
    fn a_missing_criteria_path_is_rejected() {
        let error = parse_qualify_arguments(&args(&["--evidence", "target/backtest"]))
            .expect_err("the criteria path is missing");
        assert!(error.to_string().contains("--criteria"));
    }

    #[test]
    fn an_unknown_option_is_rejected() {
        let error = parse_qualify_arguments(&args(&[
            "--evidence",
            "target/backtest",
            "--criteria",
            "qualification.json",
            "--verdict",
            "pass",
        ]))
        .expect_err("the option is unknown");
        assert!(error.to_string().contains("unknown option"));
    }
}
