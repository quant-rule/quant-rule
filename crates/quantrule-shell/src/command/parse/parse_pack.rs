//! The `pack` family parser.
//!
//! A pack is named by its content id, never by a local path. The create command
//! is the one exception: it reads the operator's chosen file, which the
//! dispatcher sends as the request body.

use super::{pair, parse_digest, parse_options, single, usage};
use crate::command::types::{Command, ParseError};

pub(super) fn parse_pack(args: &[String]) -> Result<Command, ParseError> {
    const FAMILY: &str = "usage: pack create <pack.json>|list [--kind <kind>] [--state <state>]|show <pack_id>|validate <pack_id>|freeze <pack_id>|retire <pack_id>|fetch <pack_id> <file>";
    let (name, rest) = args
        .split_first()
        .ok_or_else(|| ParseError::Usage(String::from(FAMILY)))?;
    match name.as_str() {
        "create" => {
            let line = usage("pack create");
            let usage = line.as_str();
            single(rest, usage).map(|path| Command::PackCreate { path: path.into() })
        }
        "list" => {
            let line = usage("pack list");
            let usage = line.as_str();
            let options = parse_options(rest, usage, &["kind", "state"], &[])?;
            options.expect_positionals(0, usage)?;
            let kind = options
                .value("kind")
                .map(|value| parse_kind(value, usage))
                .transpose()?;
            let state = options
                .value("state")
                .map(|value| parse_state(value, usage))
                .transpose()?;
            Ok(Command::PackList { kind, state })
        }
        "show" | "validate" | "freeze" | "retire" => {
            let line = usage(&format!("pack {name}"));
            let usage = line.as_str();
            let pack_id = parse_digest(&single(rest, usage)?, "pack_id", usage)?;
            Ok(match name.as_str() {
                "show" => Command::PackShow { pack_id },
                "validate" => Command::PackValidate { pack_id },
                "freeze" => Command::PackFreeze { pack_id },
                _ => Command::PackRetire { pack_id },
            })
        }
        "fetch" => {
            let line = usage("pack fetch");
            let usage = line.as_str();
            let (pack_id, file) = pair(rest, usage)?;
            Ok(Command::PackFetch {
                pack_id: parse_digest(&pack_id, "pack_id", usage)?,
                file: file.into(),
            })
        }
        other => Err(ParseError::Usage(format!(
            "unknown pack command '{other}'\n{FAMILY}"
        ))),
    }
}

fn parse_kind(value: &str, usage: &str) -> Result<quantrule_api::PackKind, ParseError> {
    quantrule_api::PackKind::from_code(value).ok_or_else(|| {
        ParseError::Usage(format!(
            "--kind expects strategy, portfolio, or execution\n{usage}"
        ))
    })
}

fn parse_state(value: &str, usage: &str) -> Result<quantrule_api::PackState, ParseError> {
    quantrule_api::PackState::from_code(value).ok_or_else(|| {
        ParseError::Usage(format!(
            "--state expects draft, validated, frozen, or retired\n{usage}"
        ))
    })
}
