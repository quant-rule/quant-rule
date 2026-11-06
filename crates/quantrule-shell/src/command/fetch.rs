//! Fetch command parsing and execution.
//!
//! The command streams a Dukascopy range through the shell's transport and
//! writes the accepted candles as CSV. The engine never reaches for the network
//! itself.

use std::io::{self, Write};
use std::path::PathBuf;

use quantrule_data::{EXTENDED_SERIES, MAX_SERIES, Timeframe};

use super::parse_native_timeframe;

use super::types::{ParseError, RunError};
use crate::date::DateRange;
use crate::fetch::{self, FetchOptions};
use crate::request::FetchArgs;
use crate::source::{FetchRequest, HttpTransport, Symbol};

/// Parses the arguments of a `fetch` command.
///
/// This entry point is also used by the non-interactive binary, where the
/// operating system has already split the arguments and a path may contain
/// spaces without quoting.
///
/// # Errors
///
/// Returns a [`ParseError::Fetch`] with a usage message when a positional
/// argument, option value, date, symbol, or timeframe is invalid.
pub(crate) fn parse_fetch_arguments(arguments: &[String]) -> Result<FetchArgs, ParseError> {
    if arguments.len() < 3 {
        return Err(ParseError::Fetch(String::from(
            "usage: fetch <symbol> <start> <end> [--timeframe <1m|5m|15m|1h>] [--csv <path>]",
        )));
    }

    let symbol = Symbol::resolve(&arguments[0])
        .map_err(|error| ParseError::Fetch(format!("invalid symbol: {error}")))?;
    let range = DateRange::parse(&arguments[1], &arguments[2])
        .map_err(|error| ParseError::Fetch(format!("invalid date range: {error}")))?;

    let mut timeframe = None;
    let mut csv = None;
    let mut index = 3;

    while index < arguments.len() {
        match arguments[index].as_str() {
            "--timeframe" => {
                let value = option_value(arguments, &mut index, "--timeframe")?;
                if timeframe.is_some() {
                    return Err(ParseError::Fetch(String::from(
                        "--timeframe was supplied twice",
                    )));
                }
                timeframe = Some(parse_native_timeframe(value).ok_or_else(|| {
                    ParseError::Fetch(format!(
                        "--timeframe expects 1m, 5m, 15m, or 1h, found '{value}'"
                    ))
                })?);
            }
            "--csv" => {
                let value = option_value(arguments, &mut index, "--csv")?;
                if csv.is_some() {
                    return Err(ParseError::Fetch(String::from("--csv was supplied twice")));
                }
                csv = Some(PathBuf::from(value));
            }
            option if option.starts_with("--") => {
                return Err(ParseError::Fetch(format!("unknown option '{option}'")));
            }
            argument => {
                return Err(ParseError::Fetch(format!(
                    "unexpected argument '{argument}'"
                )));
            }
        }
        index += 1;
    }

    Ok(FetchArgs {
        symbol,
        range,
        timeframe: timeframe.unwrap_or(Timeframe::M1),
        csv,
    })
}

fn option_value<'a>(
    arguments: &'a [String],
    index: &mut usize,
    option: &str,
) -> Result<&'a str, ParseError> {
    *index += 1;
    let value = arguments
        .get(*index)
        .ok_or_else(|| ParseError::Fetch(format!("{option} requires a value")))?;
    if value.is_empty() {
        return Err(ParseError::Fetch(format!(
            "{option} requires a non-empty value"
        )));
    }
    Ok(value)
}

/// Fetches a Dukascopy range and writes the CSV.
///
/// The CSV path defaults to
/// `data/sources/<SYMBOL>_<M1|M5|M15|H1>_<start>_<end>.csv`, where `<SYMBOL>` is the
/// uppercase compact symbol.
///
/// # Errors
///
/// Returns a [`RunError`] when the source, CSV, or summary writer fails.
pub(crate) fn run_fetch(
    args: &FetchArgs,
    writer: &mut impl Write,
    warnings: &mut impl Write,
) -> Result<(), RunError> {
    let csv_path = args
        .csv
        .clone()
        .unwrap_or_else(|| fetch::default_csv_path(&args.symbol, &args.range, args.timeframe));
    let options = FetchOptions::new(csv_path);
    let request = FetchRequest::new(args.symbol.clone(), args.range.clone(), args.timeframe)
        .map_err(|source| RunError::Fetch(fetch::FetchRunError::Source(source)))?;
    let transport = HttpTransport::new();

    let outcome = fetch::run_fetch(&transport, &request, &options).map_err(RunError::Fetch)?;
    fetch::write_summary(&outcome, &request, writer).map_err(RunError::Output)?;
    write_fetch_cap_warning(outcome.accepted, warnings).map_err(RunError::Output)?;
    writer.flush().map_err(RunError::Output)?;
    warnings.flush().map_err(RunError::Output)
}

/// Warns when a fetch produced more rows than the batch dataset cap.
///
/// The fetch streams the whole range and writes every accepted row, so the
/// warning makes the downstream limit visible before the user tries to read the
/// file as a batch dataset.
pub(crate) fn write_fetch_cap_warning(
    accepted: usize,
    warnings: &mut impl Write,
) -> io::Result<()> {
    if accepted > MAX_SERIES {
        writeln!(
            warnings,
            "warning: accepted={accepted} exceeds the {MAX_SERIES}-row batch cap; report model accepts up to {EXTENDED_SERIES} rows"
        )?;
    }
    Ok(())
}
