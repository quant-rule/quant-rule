//! The `portfolio` command family.
//!
//! `portfolio replay` turns an execution fills bundle into portfolio accounting.
//! The parser owns the argument grammar; the stage implementation lives in
//! `crate::portfolio`.

use std::path::PathBuf;

use super::types::{Command, ParseError};
use crate::request::PortfolioReplayArgs;
use quantrule_app::service::AppService;
use quantrule_app::stages::portfolio::PortfolioRequest;

const USAGE: &str =
    "usage: portfolio replay --fills <execution-bundle> --config <run.json> [--out <dir>] [--json]";

/// Parses the already-split arguments after `portfolio`.
///
/// # Errors
///
/// Returns [`ParseError::Portfolio`] for a missing engine, a repeated or missing
/// option, an unknown option, or an extra positional argument.
pub(crate) fn parse_portfolio_arguments(arguments: &[String]) -> Result<Command, ParseError> {
    let Some((engine, rest)) = arguments.split_first() else {
        return Err(ParseError::Portfolio(format!(
            "missing portfolio engine\n{USAGE}"
        )));
    };
    if engine != "replay" {
        return Err(ParseError::Portfolio(format!(
            "unknown portfolio engine '{engine}'; use replay"
        )));
    }

    let mut fills = None;
    let mut config = None;
    let mut out = None;
    let mut json = false;
    let mut index = 0;
    while index < rest.len() {
        match rest[index].as_str() {
            "--fills" => {
                if fills.is_some() {
                    return Err(ParseError::Portfolio(format!(
                        "--fills was supplied twice\n{USAGE}"
                    )));
                }
                fills = Some(next_path(rest, &mut index, "--fills")?);
            }
            "--config" => {
                if config.is_some() {
                    return Err(ParseError::Portfolio(format!(
                        "--config was supplied twice\n{USAGE}"
                    )));
                }
                config = Some(next_path(rest, &mut index, "--config")?);
            }
            "--out" => {
                if out.is_some() {
                    return Err(ParseError::Portfolio(format!(
                        "--out was supplied twice\n{USAGE}"
                    )));
                }
                out = Some(next_path(rest, &mut index, "--out")?);
            }
            "--json" => {
                if json {
                    return Err(ParseError::Portfolio(format!(
                        "--json was supplied twice\n{USAGE}"
                    )));
                }
                json = true;
            }
            option if option.starts_with("--") => {
                return Err(ParseError::Portfolio(format!(
                    "unknown option '{option}'\n{USAGE}"
                )));
            }
            value => {
                return Err(ParseError::Portfolio(format!(
                    "unexpected argument '{value}'\n{USAGE}"
                )));
            }
        }
        index += 1;
    }

    Ok(Command::PortfolioReplay(PortfolioReplayArgs {
        fills: fills.ok_or_else(|| missing("--fills"))?,
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
            ParseError::Portfolio(format!("{option} requires a non-empty path\n{USAGE}"))
        })
}

fn missing(option: &str) -> ParseError {
    ParseError::Portfolio(format!("{option} is required\n{USAGE}"))
}

/// Runs `portfolio replay`.
///
/// # Errors
///
/// Returns [`super::types::RunError::Portfolio`] when the fills bundle, the run
/// config, the candle series, the ledger, or the output target fails.
pub(crate) fn run_portfolio(
    args: &PortfolioReplayArgs,
    writer: &mut impl std::io::Write,
) -> Result<(), super::types::RunError> {
    let request = PortfolioRequest {
        config: &args.config,
        fills: &args.fills,
        out: args.out.as_deref(),
    };
    let outcome =
        AppService::replay_portfolio(&request).map_err(super::types::RunError::Portfolio)?;
    if args.json {
        writer
            .write_all(&outcome.json)
            .map_err(super::types::RunError::Output)?;
    } else {
        for name in &outcome.artifacts {
            writeln!(writer, "{}", outcome.root.join(name).display())
                .map_err(super::types::RunError::Output)?;
        }
    }
    writer.flush().map_err(super::types::RunError::Output)
}

#[cfg(test)]
mod tests {
    use super::parse_portfolio_arguments;
    use crate::command::types::Command;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn replay_parses_the_fills_and_config() {
        let command = parse_portfolio_arguments(&args(&[
            "replay",
            "--fills",
            "target/report/execution",
            "--config",
            "run.json",
            "--out",
            "target/report",
            "--json",
        ]))
        .expect("the command is valid");
        assert!(matches!(command, Command::PortfolioReplay(_)));
    }

    #[test]
    fn a_missing_fills_path_is_rejected() {
        let error = parse_portfolio_arguments(&args(&["replay", "--config", "run.json"]))
            .expect_err("the fills path is missing");
        assert!(error.to_string().contains("--fills"));
    }

    #[test]
    fn an_unknown_engine_is_rejected() {
        let error = parse_portfolio_arguments(&args(&["run"])).expect_err("the engine is unknown");
        assert!(error.to_string().contains("unknown portfolio engine"));
    }
}
