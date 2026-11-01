//! Fetch command parsing and execution.
//!
//! The command streams a Dukascopy range through the shell's transport, writes
//! the accepted candles, and warns when the result exceeds the batch cap. The
//! engine never reaches for the network itself.

use std::io::{self, Write};
use std::path::PathBuf;

use quantrule_data::{DEFAULT_WINDOW, EXTENDED_SERIES, MAX_SERIES, Timeframe};

use super::types::{FetchArgs, ParseError, RunError};
use crate::fetch::{self, DEFAULT_CHART_CANDLES, FetchOptions};
use crate::source::{DateRange, FetchRequest, HttpTransport, Symbol};

/// Parses the arguments of a `fetch` command.
///
/// This entry point is also used by the non-interactive binary, where the
/// operating system has already split the arguments and a path may contain
/// spaces without quoting.
///
/// # Errors
///
/// Returns a [`ParseError::Fetch`] with a usage message when a positional
/// argument, option value, date, symbol, or chart candle count is invalid.
pub(crate) fn parse_fetch_arguments(arguments: &[String]) -> Result<FetchArgs, ParseError> {
    if arguments.len() < 3 {
        return Err(ParseError::Fetch(String::from(
            "usage: fetch <symbol> <start> <end> [--timeframe <1m|5m|15m|1h>] [--csv <path>] [--png <path>] [--candles <n>]",
        )));
    }

    let symbol = Symbol::resolve(&arguments[0])
        .map_err(|error| ParseError::Fetch(format!("invalid symbol: {error}")))?;
    let range = DateRange::parse(&arguments[1], &arguments[2])
        .map_err(|error| ParseError::Fetch(format!("invalid date range: {error}")))?;

    let mut timeframe = None;
    let mut csv = None;
    let mut png = None;
    let mut candles = DEFAULT_CHART_CANDLES;
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
                timeframe = Some(Timeframe::from_code(value).ok_or_else(|| {
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
            "--png" => {
                let value = option_value(arguments, &mut index, "--png")?;
                if png.is_some() {
                    return Err(ParseError::Fetch(String::from("--png was supplied twice")));
                }
                png = Some(PathBuf::from(value));
            }
            "--candles" => {
                let value = option_value(arguments, &mut index, "--candles")?;
                let count = value.parse::<usize>().map_err(|_| {
                    ParseError::Fetch(format!(
                        "--candles expects a positive integer, found '{value}'"
                    ))
                })?;
                if count == 0 || count > DEFAULT_WINDOW {
                    return Err(ParseError::Fetch(format!(
                        "--candles must be between 1 and {DEFAULT_WINDOW}"
                    )));
                }
                candles = count;
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

    if let (Some(csv_path), Some(png_path)) = (&csv, &png)
        && crate::export::outputs_conflict(csv_path, png_path)
    {
        return Err(ParseError::Fetch(String::from(
            "--csv and --png must not resolve to the same file or temporary path",
        )));
    }

    Ok(FetchArgs {
        symbol,
        range,
        timeframe: timeframe.unwrap_or(Timeframe::M1),
        csv,
        png,
        candles,
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

/// Fetches a Dukascopy range and writes the CSV and optional chart.
///
/// The CSV path defaults to
/// `data/sources/<SYMBOL>_<M1|M5|M15|H1>_<start>_<end>.csv`, where `<SYMBOL>` is the
/// uppercase compact symbol. A PNG is written only when the command supplied a
/// path.
///
/// # Errors
///
/// Returns a [`RunError`] when the source, CSV, chart, PNG, or summary writer
/// fails.
pub(crate) fn run_fetch(
    args: &FetchArgs,
    writer: &mut impl Write,
    warnings: &mut impl Write,
) -> Result<(), RunError> {
    let csv_path = args
        .csv
        .clone()
        .unwrap_or_else(|| fetch::default_csv_path(&args.symbol, &args.range, args.timeframe));
    let options = FetchOptions::new(csv_path, args.png.clone(), args.candles);
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
/// The fetch itself streams the whole range and writes every accepted row, so
/// the warning is the shell's way of making the downstream limit visible before
/// the user tries to ingest or export the file.
pub(crate) fn write_fetch_cap_warning(
    accepted: usize,
    warnings: &mut impl Write,
) -> io::Result<()> {
    if accepted > MAX_SERIES {
        writeln!(
            warnings,
            "warning: accepted={accepted} exceeds the {MAX_SERIES}-row batch cap; store and research reject this file, market-chart requires --window latest, and market-report accepts up to {EXTENDED_SERIES} rows"
        )?;
    }
    Ok(())
}
