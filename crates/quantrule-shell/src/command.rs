//! Command parsing and execution for the shell.
//!
//! The shell parses the command text and calls the source and rendering
//! modules. It does not validate candles or assign series indices; those stay
//! in the data engine.

use std::fmt;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use quantrule_data::{DEFAULT_WINDOW, Timeframe};

use crate::csv::{self, SourceError};
use crate::fetch::{self, DEFAULT_CHART_CANDLES, FetchOptions};
use crate::render;
use crate::source::{DateRange, FetchRequest, HttpTransport, Symbol};

/// The help text shown by the `help` command.
pub(crate) const HELP: &str = r#"QuantRule shell commands:
  candles <csv-path>            Read a CSV and print the retained candle window as NDJSON.
  candles-tolerant <csv-path>   Read a CSV, skip bad rows, and report quality on stderr.
  fetch <symbol> <start> <end>  Fetch native BID candles; write CSV, print summary.
      [--timeframe <tf>]        Native timeframe: 1m, 5m, 15m, or 1h (default 1m).
      [--csv <path>]            CSV target (default: data/<SYMBOL>_<M1|M5|M15|H1>_<start>_<end>.csv).
      [--png <path>]            Also write a PNG chart.
      [--candles <n>]           Chart candle count (default 1000, max 1000).
  help                          Show this help.
  quit                          Exit the shell.

CSV header: Date,Open,High,Low,Close,Volume
Timestamp: YYYY-MM-DD HH:MM:SS+00:00
The window retains the last 1000 candles.
NDJSON: one candle per line, oldest first:
{"series_index":0,"window_index":0,"datetime":"2025-01-01T00:00:00Z","open":1,"high":2,"low":1,"close":2,"volume":null}
Quality report (stderr, candles-tolerant):
quality rows_read=2 accepted=1 rejected=1 duplicate=0 out_of_order=0 invalid=1 evicted=0 retained=1
skipped line=3 reason=invalid: high is below low
Fetch dates: inclusive UTC dates in YYYY-MM-DD form; no clock is read.
Non-interactive: quantrule candles <csv-path>
Non-interactive: quantrule candles-tolerant <csv-path>
Non-interactive: quantrule fetch gold 2025-01-01 2025-12-31 --timeframe 5m --csv out.csv
"#;

/// A parsed shell command.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Command {
    /// Read a CSV source strictly and print the retained window.
    Candles(PathBuf),
    /// Read a CSV source tolerantly and print the window plus a quality report.
    CandlesTolerant(PathBuf),
    /// Fetch a Dukascopy range and write a CSV and optional PNG.
    Fetch(FetchArgs),
    /// Print the command reference.
    Help,
    /// Exit the shell.
    Quit,
}

/// A parsed `fetch` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FetchArgs {
    /// The resolved provider instrument.
    pub(crate) symbol: Symbol,
    /// The inclusive UTC date range.
    pub(crate) range: DateRange,
    /// The native candle timeframe.
    pub(crate) timeframe: Timeframe,
    /// An explicit CSV target, or the default path.
    pub(crate) csv: Option<PathBuf>,
    /// An optional PNG target.
    pub(crate) png: Option<PathBuf>,
    /// The number of candles in the chart.
    pub(crate) candles: usize,
}

/// A command line could not be parsed.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ParseError {
    /// A command that takes a path was entered without one.
    MissingPath(&'static str),
    /// A command that takes no arguments received some.
    UnexpectedArguments(&'static str),
    /// A `fetch` argument list is invalid.
    Fetch(String),
    /// The command name is not recognized.
    Unknown(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingPath(command) => {
                write!(formatter, "missing CSV path; usage: {command} <csv-path>")
            }
            Self::UnexpectedArguments(command) => {
                write!(formatter, "{command} takes no arguments")
            }
            Self::Fetch(message) => formatter.write_str(message),
            Self::Unknown(command) => {
                write!(formatter, "unknown command '{command}'; type 'help'")
            }
        }
    }
}

impl std::error::Error for ParseError {}

/// A command was parsed but could not be completed.
#[derive(Debug)]
pub(crate) enum RunError {
    /// The source could not be loaded.
    Source(SourceError),
    /// A fetch could not be completed.
    Fetch(fetch::FetchRunError),
    /// The output could not be written.
    Output(io::Error),
}

impl fmt::Display for RunError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Source(source) => source.fmt(formatter),
            Self::Fetch(source) => source.fmt(formatter),
            Self::Output(source) => write!(formatter, "cannot write output: {source}"),
        }
    }
}

impl std::error::Error for RunError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Source(source) => Some(source),
            Self::Fetch(source) => Some(source),
            Self::Output(source) => Some(source),
        }
    }
}

/// Parses one command line.
///
/// A blank line returns `Ok(None)`. A `candles` or `candles-tolerant` path is
/// the trimmed remainder of the line, so a path may contain spaces.
///
/// # Errors
///
/// Returns a [`ParseError`] for a missing path, unexpected arguments, or an
/// unknown command.
pub(crate) fn parse(line: &str) -> Result<Option<Command>, ParseError> {
    let line = line.trim();
    if line.is_empty() {
        return Ok(None);
    }

    let (name, rest) = match line.split_once(char::is_whitespace) {
        Some((name, rest)) => (name, rest.trim()),
        None => (line, ""),
    };

    match name {
        "candles" => path_command(rest, "candles", Command::Candles),
        "candles-tolerant" => path_command(rest, "candles-tolerant", Command::CandlesTolerant),
        "fetch" => {
            let arguments = tokenize(rest)?;
            parse_fetch_arguments(&arguments).map(|args| Some(Command::Fetch(args)))
        }
        "help" => {
            if rest.is_empty() {
                Ok(Some(Command::Help))
            } else {
                Err(ParseError::UnexpectedArguments("help"))
            }
        }
        "quit" => {
            if rest.is_empty() {
                Ok(Some(Command::Quit))
            } else {
                Err(ParseError::UnexpectedArguments("quit"))
            }
        }
        _ => Err(ParseError::Unknown(name.to_owned())),
    }
}

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
        && csv_path == png_path
    {
        return Err(ParseError::Fetch(String::from(
            "--csv and --png must be different paths",
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

fn tokenize(input: &str) -> Result<Vec<String>, ParseError> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut in_token = false;

    for character in input.chars() {
        match quote {
            Some(delimiter) if character == delimiter => {
                quote = None;
                in_token = true;
            }
            Some(_) => {
                current.push(character);
                in_token = true;
            }
            None if character == '"' || character == '\'' => {
                quote = Some(character);
                in_token = true;
            }
            None if character.is_whitespace() => {
                if in_token {
                    tokens.push(std::mem::take(&mut current));
                    in_token = false;
                }
            }
            None => {
                current.push(character);
                in_token = true;
            }
        }
    }

    if quote.is_some() {
        return Err(ParseError::Fetch(String::from(
            "unterminated quoted argument",
        )));
    }
    if in_token {
        tokens.push(current);
    }

    Ok(tokens)
}

fn path_command(
    rest: &str,
    name: &'static str,
    build: fn(PathBuf) -> Command,
) -> Result<Option<Command>, ParseError> {
    if rest.is_empty() {
        Err(ParseError::MissingPath(name))
    } else {
        Ok(Some(build(PathBuf::from(rest))))
    }
}

/// Loads a CSV source strictly and writes its retained window as NDJSON.
///
/// # Errors
///
/// Returns a [`RunError`] when the source fails or the writer fails.
pub(crate) fn run_candles(path: &Path, writer: &mut impl Write) -> Result<(), RunError> {
    let window = csv::load_window(path).map_err(RunError::Source)?;
    render::write_ndjson(&window, writer).map_err(RunError::Output)?;
    writer.flush().map_err(RunError::Output)
}

/// Loads a CSV source tolerantly, writes the retained window as NDJSON, and
/// writes the quality report to a second writer.
///
/// The NDJSON is flushed before the report is written. A source-level failure
/// leaves stdout untouched because the window is built before rendering.
///
/// # Errors
///
/// Returns a [`RunError`] when the source fails or either writer fails.
pub(crate) fn run_candles_tolerant(
    path: &Path,
    output: &mut impl Write,
    report: &mut impl Write,
) -> Result<(), RunError> {
    let loaded = csv::load_window_tolerant(path).map_err(RunError::Source)?;
    render::write_ndjson(&loaded.window, output).map_err(RunError::Output)?;
    output.flush().map_err(RunError::Output)?;
    loaded.report.write(report).map_err(RunError::Output)?;
    report.flush().map_err(RunError::Output)
}

/// Fetches a Dukascopy range and writes the CSV and optional chart.
///
/// The CSV path defaults to
/// `data/<SYMBOL>_<M1|M5|M15|H1>_<start>_<end>.csv`, where `<SYMBOL>` is the
/// uppercase compact symbol. A PNG is written only when the command supplied a
/// path.
///
/// # Errors
///
/// Returns a [`RunError`] when the source, CSV, chart, PNG, or summary writer
/// fails.
pub(crate) fn run_fetch(args: &FetchArgs, writer: &mut impl Write) -> Result<(), RunError> {
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
    writer.flush().map_err(RunError::Output)
}

/// Writes the command reference.
///
/// # Errors
///
/// Returns a [`RunError`] when the writer fails.
pub(crate) fn run_help(writer: &mut impl Write) -> Result<(), RunError> {
    writer
        .write_all(HELP.as_bytes())
        .map_err(RunError::Output)?;
    writer.flush().map_err(RunError::Output)
}

#[cfg(test)]
mod tests {
    use super::{Command, FetchArgs, ParseError, parse, parse_fetch_arguments};
    use std::path::PathBuf;

    use crate::source::{DateRange, Symbol};
    use quantrule_data::Timeframe;

    fn fetch_args(csv: Option<PathBuf>, png: Option<PathBuf>, candles: usize) -> FetchArgs {
        fetch_args_with_timeframe(Timeframe::M1, csv, png, candles)
    }

    fn fetch_args_with_timeframe(
        timeframe: Timeframe,
        csv: Option<PathBuf>,
        png: Option<PathBuf>,
        candles: usize,
    ) -> FetchArgs {
        FetchArgs {
            symbol: Symbol::resolve("gold").expect("gold is known"),
            range: DateRange::parse("2025-01-01", "2025-12-31").expect("the dates are valid"),
            timeframe,
            csv,
            png,
            candles,
        }
    }

    #[test]
    fn blank_lines_are_ignored() {
        assert_eq!(parse(""), Ok(None));
        assert_eq!(parse("   \t "), Ok(None));
    }

    #[test]
    fn parses_candles_with_a_path() {
        assert_eq!(
            parse("candles data/example.csv"),
            Ok(Some(Command::Candles(PathBuf::from("data/example.csv"))))
        );
    }

    #[test]
    fn parses_tolerant_candles_with_a_path() {
        assert_eq!(
            parse("candles-tolerant data/example.csv"),
            Ok(Some(Command::CandlesTolerant(PathBuf::from(
                "data/example.csv"
            ))))
        );
    }

    #[test]
    fn preserves_spaces_in_a_candles_path() {
        assert_eq!(
            parse("candles my file.csv"),
            Ok(Some(Command::Candles(PathBuf::from("my file.csv"))))
        );
    }

    #[test]
    fn preserves_spaces_in_a_tolerant_candles_path() {
        assert_eq!(
            parse("candles-tolerant my file.csv"),
            Ok(Some(Command::CandlesTolerant(PathBuf::from("my file.csv"))))
        );
    }

    #[test]
    fn rejects_candles_without_a_path() {
        assert_eq!(parse("candles"), Err(ParseError::MissingPath("candles")));
        assert_eq!(parse("candles   "), Err(ParseError::MissingPath("candles")));
    }

    #[test]
    fn rejects_tolerant_candles_without_a_path() {
        assert_eq!(
            parse("candles-tolerant"),
            Err(ParseError::MissingPath("candles-tolerant"))
        );
        assert_eq!(
            parse("candles-tolerant   "),
            Err(ParseError::MissingPath("candles-tolerant"))
        );
    }

    #[test]
    fn parses_help_and_quit() {
        assert_eq!(parse("help"), Ok(Some(Command::Help)));
        assert_eq!(parse("quit"), Ok(Some(Command::Quit)));
    }

    #[test]
    fn rejects_arguments_to_help_and_quit() {
        assert_eq!(
            parse("help now"),
            Err(ParseError::UnexpectedArguments("help"))
        );
        assert_eq!(
            parse("quit now"),
            Err(ParseError::UnexpectedArguments("quit"))
        );
    }

    #[test]
    fn rejects_an_unknown_command() {
        assert_eq!(
            parse("bogus"),
            Err(ParseError::Unknown(String::from("bogus")))
        );
    }

    #[test]
    fn parses_fetch_with_defaults() {
        assert_eq!(
            parse("fetch gold 2025-01-01 2025-12-31"),
            Ok(Some(Command::Fetch(fetch_args(None, None, 1_000))))
        );
    }

    #[test]
    fn parses_each_supported_timeframe() {
        for (code, timeframe) in [
            ("1m", Timeframe::M1),
            ("5m", Timeframe::M5),
            ("15m", Timeframe::M15),
            ("1h", Timeframe::H1),
        ] {
            let line = format!("fetch gold 2025-01-01 2025-12-31 --timeframe {code}");
            assert_eq!(
                parse(&line),
                Ok(Some(Command::Fetch(fetch_args_with_timeframe(
                    timeframe, None, None, 1_000
                )))),
                "timeframe {code} is parsed"
            );
        }
    }

    #[test]
    fn parses_fetch_options_in_any_order() {
        assert_eq!(
            parse("fetch gold 2025-01-01 2025-12-31 --candles 250 --png out.png --csv out.csv"),
            Ok(Some(Command::Fetch(fetch_args(
                Some(PathBuf::from("out.csv")),
                Some(PathBuf::from("out.png")),
                250,
            ))))
        );
        assert_eq!(
            parse("fetch gold 2025-01-01 2025-12-31 --png out.png --timeframe 5m --csv out.csv"),
            Ok(Some(Command::Fetch(fetch_args_with_timeframe(
                Timeframe::M5,
                Some(PathBuf::from("out.csv")),
                Some(PathBuf::from("out.png")),
                1_000,
            ))))
        );
    }

    #[test]
    fn parses_a_quoted_path_with_spaces() {
        assert_eq!(
            parse("fetch gold 2025-01-01 2025-12-31 --csv \"my data/out.csv\""),
            Ok(Some(Command::Fetch(fetch_args(
                Some(PathBuf::from("my data/out.csv")),
                None,
                1_000,
            ))))
        );
    }

    #[test]
    fn parses_fetch_arguments_from_an_already_split_argv() {
        let arguments = [
            String::from("gold"),
            String::from("2025-01-01"),
            String::from("2025-12-31"),
            String::from("--csv"),
            String::from("my data/out.csv"),
        ];

        assert_eq!(
            parse_fetch_arguments(&arguments),
            Ok(fetch_args(
                Some(PathBuf::from("my data/out.csv")),
                None,
                1_000
            ))
        );
    }

    #[test]
    fn rejects_fetch_without_positionals() {
        assert!(matches!(parse("fetch"), Err(ParseError::Fetch(_))));
        assert!(matches!(
            parse("fetch gold 2025-01-01"),
            Err(ParseError::Fetch(_))
        ));
    }

    #[test]
    fn rejects_an_invalid_fetch_symbol_or_date() {
        assert!(matches!(
            parse("fetch notreal 2025-01-01 2025-12-31"),
            Err(ParseError::Fetch(_))
        ));
        assert!(matches!(
            parse("fetch gold 2025-02-30 2025-12-31"),
            Err(ParseError::Fetch(_))
        ));
        assert!(matches!(
            parse("fetch gold 2025-12-31 2025-01-01"),
            Err(ParseError::Fetch(_))
        ));
    }

    #[test]
    fn rejects_bad_fetch_options() {
        assert!(matches!(
            parse("fetch gold 2025-01-01 2025-12-31 --csv"),
            Err(ParseError::Fetch(_))
        ));
        assert!(matches!(
            parse("fetch gold 2025-01-01 2025-12-31 --csv a.csv --csv b.csv"),
            Err(ParseError::Fetch(_))
        ));
        assert!(matches!(
            parse("fetch gold 2025-01-01 2025-12-31 --bogus x"),
            Err(ParseError::Fetch(_))
        ));
        assert!(matches!(
            parse("fetch gold 2025-01-01 2025-12-31 extra"),
            Err(ParseError::Fetch(_))
        ));
        assert!(matches!(
            parse("fetch gold 2025-01-01 2025-12-31 --csv same.csv --png same.csv"),
            Err(ParseError::Fetch(_))
        ));
        assert!(matches!(
            parse("fetch gold 2025-01-01 2025-12-31 --timeframe 30m"),
            Err(ParseError::Fetch(_))
        ));
        assert!(matches!(
            parse("fetch gold 2025-01-01 2025-12-31 --timeframe 5m --timeframe 15m"),
            Err(ParseError::Fetch(_))
        ));
        assert!(matches!(
            parse("fetch gold 2025-01-01 2025-12-31 --timeframe"),
            Err(ParseError::Fetch(_))
        ));
    }

    #[test]
    fn rejects_an_out_of_range_chart_count() {
        for count in ["0", "1001", "many", "-1"] {
            let line = format!("fetch gold 2025-01-01 2025-12-31 --candles {count}");
            assert!(
                matches!(parse(&line), Err(ParseError::Fetch(_))),
                "count {count} should be rejected"
            );
        }
    }

    #[test]
    fn rejects_the_same_csv_and_png_path() {
        assert!(matches!(
            parse("fetch gold 2025-01-01 2025-12-31 --csv out --png out"),
            Err(ParseError::Fetch(_))
        ));
    }

    #[test]
    fn rejects_an_unterminated_quote() {
        assert!(matches!(
            parse("fetch gold 2025-01-01 2025-12-31 --csv \"out.csv"),
            Err(ParseError::Fetch(_))
        ));
    }
}
