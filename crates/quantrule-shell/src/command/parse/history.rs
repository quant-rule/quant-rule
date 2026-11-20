//! The `history` family parser.

use super::{parse_limit, parse_options, usage};
use crate::command::types::{Command, HistoryKind, ParseError};

const FAMILY: &str = "usage: history list --kind <run|stage|research|simulation|dataset> [--include-archived] [--limit <n>]|archive --kind <run|research|simulation|dataset> --id <id> [--dry-run]|erase --kind <run|stage|research|simulation|dataset> --id <id> --confirm <id> [--dry-run]";

pub(super) fn parse_history(args: &[String]) -> Result<Command, ParseError> {
    let (name, rest) = args
        .split_first()
        .ok_or_else(|| ParseError::Usage(String::from(FAMILY)))?;
    match name.as_str() {
        "list" => {
            let line = usage("history list");
            let usage = line.as_str();
            let options = parse_options(rest, usage, &["kind", "limit"], &["include-archived"])?;
            options.expect_positionals(0, usage)?;
            let kind = kind(options.required("kind", usage)?, usage)?;
            let limit = options
                .value("limit")
                .map(|value| parse_limit(value, usage))
                .transpose()?;
            Ok(Command::HistoryList {
                kind,
                include_archived: options.flag("include-archived"),
                limit,
            })
        }
        "archive" => {
            let line = usage("history archive");
            let usage = line.as_str();
            let options = parse_options(rest, usage, &["kind", "id"], &["dry-run"])?;
            options.expect_positionals(0, usage)?;
            let kind = kind(options.required("kind", usage)?, usage)?;
            if kind == HistoryKind::Stage {
                return Err(ParseError::Usage(format!(
                    "a stage has no archive; use history erase\n{usage}"
                )));
            }
            Ok(Command::HistoryArchive {
                kind,
                id: options.required("id", usage)?.to_owned(),
                dry_run: options.flag("dry-run"),
            })
        }
        "erase" => {
            let line = usage("history erase");
            let usage = line.as_str();
            let options = parse_options(rest, usage, &["kind", "id", "confirm"], &["dry-run"])?;
            options.expect_positionals(0, usage)?;
            let kind = kind(options.required("kind", usage)?, usage)?;
            let id = options.required("id", usage)?.to_owned();
            let confirm = options.required("confirm", usage)?.to_owned();
            if confirm != id {
                return Err(ParseError::Usage(format!(
                    "--confirm must equal the record id\n{usage}"
                )));
            }
            Ok(Command::HistoryErase {
                kind,
                id,
                confirm,
                dry_run: options.flag("dry-run"),
            })
        }
        other => Err(ParseError::Usage(format!(
            "unknown history command '{other}'\n{FAMILY}"
        ))),
    }
}

fn kind(value: &str, usage: &str) -> Result<HistoryKind, ParseError> {
    HistoryKind::parse(value).ok_or_else(|| {
        ParseError::Usage(format!(
            "--kind expects run, stage, research, simulation, or dataset\n{usage}"
        ))
    })
}
