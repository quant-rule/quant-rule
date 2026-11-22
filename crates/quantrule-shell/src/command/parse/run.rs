//! The `run` family parser.

use quantrule_api::RunState;

use super::{pair, parse_limit, parse_options, single, usage};
use crate::command::types::{Command, ParseError};

pub(super) fn parse_run(args: &[String]) -> Result<Command, ParseError> {
    const FAMILY: &str = "usage: run submit <data_version> <config_id> [--model-artifact <id>]|status <run_id>|list|cancel <run_id>|fetch <run_id> <directory>|lineage <run_id>";
    let (name, rest) = args
        .split_first()
        .ok_or_else(|| ParseError::Usage(String::from(FAMILY)))?;
    match name.as_str() {
        "submit" => {
            let line = usage("run submit");
            let usage = line.as_str();
            let options = parse_options(rest, usage, &["model-artifact"], &[])?;
            options.expect_positionals(2, usage)?;
            Ok(Command::RunSubmit {
                data_version: options.positional(0, "the data_version", usage)?.to_owned(),
                config_id: options.positional(1, "the config_id", usage)?.to_owned(),
                model_artifact_id: options.value("model-artifact").map(str::to_owned),
            })
        }
        "status" => {
            let line = usage("run status");
            let usage = line.as_str();
            single(rest, usage).map(|run_id| Command::RunStatus { run_id })
        }
        "list" => {
            let line = usage("run list");
            let usage = line.as_str();
            let options = parse_options(rest, usage, &["status", "limit"], &[])?;
            options.expect_positionals(0, usage)?;
            let status = options
                .value("status")
                .map(|value| parse_run_state(value, usage))
                .transpose()?;
            let limit = options
                .value("limit")
                .map(|value| parse_limit(value, usage))
                .transpose()?;
            Ok(Command::RunList { status, limit })
        }
        "fetch" => {
            let line = usage("run fetch");
            let usage = line.as_str();
            pair(rest, usage).map(|(run_id, directory)| Command::RunFetch {
                run_id,
                directory: directory.into(),
            })
        }
        "cancel" => {
            let line = usage("run cancel");
            let usage = line.as_str();
            single(rest, usage).map(|run_id| Command::RunCancel { run_id })
        }
        "lineage" => {
            let line = usage("run lineage");
            let usage = line.as_str();
            single(rest, usage).map(|run_id| Command::RunLineage { run_id })
        }
        other => Err(ParseError::Usage(format!(
            "unknown run command '{other}'\n{FAMILY}"
        ))),
    }
}

fn parse_run_state(value: &str, usage: &str) -> Result<RunState, ParseError> {
    RunState::parse(value).ok_or_else(|| {
        ParseError::Usage(format!(
            "--status expects queued, running, succeeded, failed, or cancelled\n{usage}"
        ))
    })
}
