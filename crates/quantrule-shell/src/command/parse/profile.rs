//! The `profile` family parser.

use super::{no_arguments, parse_options, single, usage};
use crate::command::types::{Command, ParseError};

pub(super) fn parse_profile(args: &[String]) -> Result<Command, ParseError> {
    const FAMILY: &str = "usage: profile list|create <name> [--id <profile_id>]|use <profile_id>";
    let (name, rest) = args
        .split_first()
        .ok_or_else(|| ParseError::Usage(String::from(FAMILY)))?;
    match name.as_str() {
        "list" => {
            let line = usage("profile list");
            no_arguments(rest, line.as_str()).map(|()| Command::ProfileList)
        }
        "create" => {
            let line = usage("profile create");
            let usage = line.as_str();
            let options = parse_options(rest, usage, &["id"], &[])?;
            options.expect_positionals(1, usage)?;
            let name = options.positional(0, "the profile name", usage)?.to_owned();
            if name.trim().is_empty() {
                return Err(ParseError::Usage(format!(
                    "the profile name cannot be blank\n{usage}"
                )));
            }
            if let Some(id) = options.value("id") {
                quantrule_api::ProfileId::parse(id)
                    .map_err(|error| ParseError::Usage(format!("--id {error}")))?;
            }
            Ok(Command::ProfileCreate {
                name,
                id: options.value("id").map(str::to_owned),
            })
        }
        "use" => {
            let line = usage("profile use");
            let usage = line.as_str();
            let profile_id = single(rest, usage)?;
            quantrule_api::ProfileId::parse(&profile_id)
                .map_err(|error| ParseError::Usage(format!("profile use {error}")))?;
            Ok(Command::ProfileUse { profile_id })
        }
        other => Err(ParseError::Usage(format!(
            "unknown profile command '{other}'\n{FAMILY}"
        ))),
    }
}
