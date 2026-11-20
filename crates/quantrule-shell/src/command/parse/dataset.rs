//! The `dataset` family parser.

use super::{no_arguments, parse_options, single, usage};
use crate::command::types::{Command, ParseError};

pub(super) fn parse_dataset(args: &[String]) -> Result<Command, ParseError> {
    const FAMILY: &str = "usage: dataset list|show <data_version>|upload <csv-path>";
    let (name, rest) = args
        .split_first()
        .ok_or_else(|| ParseError::Usage(String::from(FAMILY)))?;
    match name.as_str() {
        "list" => {
            let line = usage("dataset list");
            no_arguments(rest, line.as_str()).map(|()| Command::DatasetList)
        }
        "show" => {
            let line = usage("dataset show");
            let usage = line.as_str();
            single(rest, usage).map(|data_version| Command::DatasetShow { data_version })
        }
        "upload" => {
            let line = usage("dataset upload");
            let usage = line.as_str();
            let options = parse_options(rest, usage, &["name"], &["tolerant"])?;
            options.expect_positionals(1, usage)?;
            Ok(Command::DatasetUpload {
                path: options.positional(0, "the CSV path", usage)?.into(),
                name: options.value("name").map(str::to_owned),
                tolerant: options.flag("tolerant"),
            })
        }
        other => Err(ParseError::Usage(format!(
            "unknown dataset command '{other}'\n{FAMILY}"
        ))),
    }
}
