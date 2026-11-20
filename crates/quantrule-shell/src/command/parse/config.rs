//! The `config` family parser.

use super::{no_arguments, single, usage};
use crate::command::types::{Command, ParseError};

pub(super) fn parse_config(args: &[String]) -> Result<Command, ParseError> {
    const FAMILY: &str = "usage: config list|show <config_id>";
    let (name, rest) = args
        .split_first()
        .ok_or_else(|| ParseError::Usage(String::from(FAMILY)))?;
    match name.as_str() {
        "list" => {
            let line = usage("config list");
            no_arguments(rest, line.as_str()).map(|()| Command::ConfigList)
        }
        "show" => {
            let line = usage("config show");
            let usage = line.as_str();
            single(rest, usage).map(|config_id| Command::ConfigShow { config_id })
        }
        other => Err(ParseError::Usage(format!(
            "unknown config command '{other}'\n{FAMILY}"
        ))),
    }
}
