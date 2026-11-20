//! The `inspect` family parser.

use super::{parse_options, single, usage};
use crate::command::types::{Command, ParseError};

pub(super) fn parse_inspect(args: &[String]) -> Result<Command, ParseError> {
    const FAMILY: &str = "usage: inspect candles <data_version>|timeframes <data_version>|events <data_version> --timeframe <tf>";
    let (name, rest) = args
        .split_first()
        .ok_or_else(|| ParseError::Usage(String::from(FAMILY)))?;
    match name.as_str() {
        "candles" => {
            let line = usage("inspect candles");
            let usage = line.as_str();
            single(rest, usage).map(|data_version| Command::InspectCandles { data_version })
        }
        "timeframes" => {
            let line = usage("inspect timeframes");
            let usage = line.as_str();
            single(rest, usage).map(|data_version| Command::InspectTimeframes { data_version })
        }
        "events" => {
            let line = usage("inspect events");
            let usage = line.as_str();
            let options = parse_options(rest, usage, &["timeframe"], &[])?;
            options.expect_positionals(1, usage)?;
            Ok(Command::InspectEvents {
                data_version: options.positional(0, "the data version", usage)?.to_owned(),
                timeframe: options.required("timeframe", usage)?.to_owned(),
            })
        }
        other => Err(ParseError::Usage(format!(
            "unknown inspect command '{other}'\n{FAMILY}"
        ))),
    }
}
