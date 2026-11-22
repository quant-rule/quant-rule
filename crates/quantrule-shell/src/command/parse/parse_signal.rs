//! The `signal` family parser.
//!
//! A signal request names references only: a frozen model artifact, the dataset
//! and config that produced it, and a frozen signal-mode strategy pack.

use super::{pair, parse_digest, parse_limit, parse_options, single, usage};
use crate::command::types::{Command, ParseError};

pub(super) fn parse_signal(args: &[String]) -> Result<Command, ParseError> {
    const FAMILY: &str = "usage: signal create --model-artifact <id> --data-version <digest> --config <config_id> --strategy-pack <pack_id>|list [--limit <n>]|show <signal_id>|fetch <signal_id> <file>";
    let (name, rest) = args
        .split_first()
        .ok_or_else(|| ParseError::Usage(String::from(FAMILY)))?;
    match name.as_str() {
        "create" => {
            let line = usage("signal create");
            let usage = line.as_str();
            let options = parse_options(
                rest,
                usage,
                &["model-artifact", "data-version", "config", "strategy-pack"],
                &[],
            )?;
            options.expect_positionals(0, usage)?;
            Ok(Command::SignalCreate {
                model_artifact_id: parse_digest(
                    options.required("model-artifact", usage)?,
                    "--model-artifact",
                    usage,
                )?,
                data_version: parse_digest(
                    options.required("data-version", usage)?,
                    "--data-version",
                    usage,
                )?,
                config_id: options.required("config", usage)?.to_owned(),
                strategy_pack_id: parse_digest(
                    options.required("strategy-pack", usage)?,
                    "--strategy-pack",
                    usage,
                )?,
            })
        }
        "list" => {
            let line = usage("signal list");
            let usage = line.as_str();
            let options = parse_options(rest, usage, &["limit"], &[])?;
            options.expect_positionals(0, usage)?;
            let limit = options
                .value("limit")
                .map(|value| parse_limit(value, usage))
                .transpose()?;
            Ok(Command::SignalList { limit })
        }
        "show" => {
            let line = usage("signal show");
            let usage = line.as_str();
            Ok(Command::SignalShow {
                signal_id: parse_digest(&single(rest, usage)?, "signal_id", usage)?,
            })
        }
        "fetch" => {
            let line = usage("signal fetch");
            let usage = line.as_str();
            let (signal_id, file) = pair(rest, usage)?;
            Ok(Command::SignalFetch {
                signal_id: parse_digest(&signal_id, "signal_id", usage)?,
                file: file.into(),
            })
        }
        other => Err(ParseError::Usage(format!(
            "unknown signal command '{other}'\n{FAMILY}"
        ))),
    }
}
