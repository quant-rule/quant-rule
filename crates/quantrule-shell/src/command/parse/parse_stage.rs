//! The `stage` family parser.
//!
//! The family owns one command per implemented stage use case.

use super::{pair, parse_limit, parse_options, single, usage};
use crate::command::types::{Command, ParseError};

pub(super) fn parse_stage(args: &[String]) -> Result<Command, ParseError> {
    const FAMILY: &str = "usage: stage run model|strategy|portfolio <input> <config_id>|explain <stage_run_id> --bar <n>|list|show <stage_run_id>|events <stage_run_id> --timeframe <tf>|fetch <stage_run_id> <directory>|cancel <stage_run_id>|delete <stage_run_id>";
    let (name, rest) = args
        .split_first()
        .ok_or_else(|| ParseError::Usage(String::from(FAMILY)))?;
    match name.as_str() {
        "run" => {
            let (kind, rest) = rest
                .split_first()
                .ok_or_else(|| ParseError::Usage(String::from(FAMILY)))?;
            match kind.as_str() {
                "model" => {
                    let line = usage("stage run model");
                    let usage = line.as_str();
                    pair(rest, usage).map(|(data_version, config_id)| Command::StageRunModel {
                        data_version,
                        config_id,
                    })
                }
                "strategy" => {
                    let line = usage("stage run strategy");
                    let usage = line.as_str();
                    pair(rest, usage).map(|(model_artifact_id, config_id)| {
                        Command::StageRunStrategy {
                            model_artifact_id,
                            config_id,
                        }
                    })
                }
                "portfolio" => {
                    let line = usage("stage run portfolio");
                    let usage = line.as_str();
                    pair(rest, usage).map(|(decisions_artifact_id, config_id)| {
                        Command::StageRunPortfolio {
                            decisions_artifact_id,
                            config_id,
                        }
                    })
                }
                _ => Err(ParseError::Usage(format!(
                    "stage run expects model, strategy, or portfolio\n{FAMILY}"
                ))),
            }
        }
        "list" => {
            let line = usage("stage list");
            let usage = line.as_str();
            let options = parse_options(rest, usage, &["kind", "status", "limit"], &[])?;
            options.expect_positionals(0, usage)?;
            let kind = options
                .value("kind")
                .map(|value| parse_stage_kind(value, usage))
                .transpose()?;
            let status = options
                .value("status")
                .map(|value| parse_stage_state(value, usage))
                .transpose()?;
            let limit = options
                .value("limit")
                .map(|value| parse_limit(value, usage))
                .transpose()?;
            Ok(Command::StageList {
                kind,
                status,
                limit,
            })
        }
        "show" => {
            let line = usage("stage show");
            let usage = line.as_str();
            single(rest, usage).map(|stage_run_id| Command::StageShow { stage_run_id })
        }
        "explain" => {
            let line = usage("stage explain");
            let usage = line.as_str();
            let options = parse_options(rest, usage, &["bar"], &[])?;
            options.expect_positionals(1, usage)?;
            Ok(Command::StageExplain {
                stage_run_id: options.positional(0, "the stage run id", usage)?.to_owned(),
                bar: parse_bar(options.required("bar", usage)?, usage)?,
            })
        }
        "events" => {
            let line = usage("stage events");
            let usage = line.as_str();
            let options = parse_options(rest, usage, &["timeframe"], &[])?;
            options.expect_positionals(1, usage)?;
            Ok(Command::StageEvents {
                stage_run_id: options.positional(0, "the stage run id", usage)?.to_owned(),
                timeframe: options.required("timeframe", usage)?.to_owned(),
            })
        }
        "fetch" => {
            let line = usage("stage fetch");
            let usage = line.as_str();
            pair(rest, usage).map(|(stage_run_id, directory)| Command::StageFetch {
                stage_run_id,
                directory: directory.into(),
            })
        }
        "cancel" => {
            let line = usage("stage cancel");
            let usage = line.as_str();
            single(rest, usage).map(|stage_run_id| Command::StageCancel { stage_run_id })
        }
        "delete" => {
            let line = usage("stage delete");
            let usage = line.as_str();
            single(rest, usage).map(|stage_run_id| Command::StageDelete { stage_run_id })
        }
        other => Err(ParseError::Usage(format!(
            "unknown stage command '{other}'\n{FAMILY}"
        ))),
    }
}

fn parse_bar(value: &str, usage: &str) -> Result<usize, ParseError> {
    value
        .parse::<usize>()
        .map_err(|_| ParseError::Usage(format!("--bar expects a non-negative integer\n{usage}")))
}

fn parse_stage_kind(value: &str, usage: &str) -> Result<quantrule_api::StageKind, ParseError> {
    quantrule_api::StageKind::parse(value).ok_or_else(|| {
        ParseError::Usage(format!(
            "--kind expects model, strategy, or portfolio\n{usage}"
        ))
    })
}

fn parse_stage_state(value: &str, usage: &str) -> Result<quantrule_api::StageState, ParseError> {
    quantrule_api::StageState::parse(value).ok_or_else(|| {
        ParseError::Usage(format!(
            "--status expects queued, running, succeeded, failed, or cancelled\n{usage}"
        ))
    })
}
