//! Command parsing and execution for the shell.
//!
//! The shell parses the command text and calls the source and rendering
//! modules. It does not validate candles or assign series indices; those stay
//! in the data engine.

use std::fmt;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use quantrule_analytics::layers::{LayerConfig, LayerError, LayerSet, build_layers};
use quantrule_analytics::research::{
    DatasetMeta, ExportError as ResearchExportError, render_dataset,
};
use quantrule_analytics::summary::{DatasetSummary, summarize};
use quantrule_data::{Candle, DEFAULT_WINDOW, Timeframe};
use quantrule_model::readings::PivotScale;
use quantrule_model::state::{MarketState, MarketStateConfig, TimeframeInput};

use crate::csv::{self, SourceError};
use crate::export::{self, ExportError as FileExportError};
use crate::fetch::{self, DEFAULT_CHART_CANDLES, FetchOptions};
use crate::render;
use crate::render_state;
use crate::source::{DateRange, FetchRequest, HttpTransport, Symbol};
use crate::store::{DatasetRecord, Store, StoreError};

/// The help text shown by the `help` command.
pub(crate) const HELP: &str = r#"QuantRule shell commands:
  candles <csv-path>            Read a CSV and print the retained candle window as NDJSON.
  candles-tolerant <csv-path>   Read a CSV, skip bad rows, and report quality on stderr.
  timeframes <csv-path>         Build M5/M15/H1 from an M1 CSV; print NDJSON and a report.
  market-state <csv-path>       Print the four-timeframe market state as JSON.
  axis <csv-path> <axis>        Print one axis per timeframe: direction, momentum, structure, progression.
  events <csv-path>             Print every event record as NDJSON, in availability order.
  legs <csv-path>               Print the minor and major leg per timeframe.
  research export <csv> <dataset>
                                Write the research dataset; requires --data-version.
      --data-version <id>       Caller-supplied identity of the input series.
      --root <dir>              Export root (default: data).
  research summary <csv-path>   Print counts, ranges, ordinal histograms, and axis correlations.
  store init [--db <path>]      Create or open the SQLite store (default data/quantrule.db).
  store ingest <csv-path>       Ingest one M1 dataset into the store.
      --dataset <name>          Dataset name; required.
      --data-version <id>       Caller-supplied identity; required.
      --db <path>               Database path (default data/quantrule.db).
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
Timeframes: M1 is the source; M5, M15, and H1 are derived by the data engine.
Timeframe NDJSON adds a timeframe field and groups the series M1, M5, M15, H1.
Quality report (stderr, candles-tolerant):
quality rows_read=2 accepted=1 rejected=1 duplicate=0 out_of_order=0 invalid=1 evicted=0 retained=1
skipped line=3 reason=invalid: high is below low
Timeframe report (stderr, timeframes):
aggregation timeframe=5m retained=2 partial=1 gaps=1
partial timeframe=5m bucket=2025-01-01T00:00:00Z present=4 expected=5
gap timeframe=5m after=2025-01-01T00:10:00Z before=2025-01-01T00:20:00Z missing=1
Fetch dates: inclusive UTC dates in YYYY-MM-DD form; no clock is read.
Non-interactive: quantrule candles <csv-path>
Non-interactive: quantrule candles-tolerant <csv-path>
Non-interactive: quantrule timeframes <csv-path>
Non-interactive: quantrule market-state <csv-path>
Non-interactive: quantrule axis <csv-path> direction
Non-interactive: quantrule events <csv-path>
Non-interactive: quantrule legs <csv-path>
Non-interactive: quantrule research export path/to/m1.csv demo --data-version source:demo:60:0-1000
Non-interactive: quantrule research summary path/to/m1.csv
Non-interactive: quantrule store init --db data/quantrule.db
Non-interactive: quantrule store ingest path/to/m1.csv --dataset demo --data-version source:demo:60:0-1000
Non-interactive: quantrule fetch gold 2025-01-01 2025-12-31 --timeframe 5m --csv out.csv
"#;

/// A parsed shell command.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Command {
    /// Read a CSV source strictly and print the retained window.
    Candles(PathBuf),
    /// Read a CSV source tolerantly and print the window plus a quality report.
    CandlesTolerant(PathBuf),
    /// Build M5, M15, and H1 from an M1 CSV, then print the series and report.
    Timeframes(PathBuf),
    /// Print the assembled market state.
    MarketState(PathBuf),
    /// Print one axis per timeframe.
    Axis { path: PathBuf, axis: Axis },
    /// Print the event records.
    Events(PathBuf),
    /// Print the legs.
    Legs(PathBuf),
    /// Write the research dataset.
    ResearchExport(ResearchExportArgs),
    /// Print the research summary.
    ResearchSummary(PathBuf),
    /// Create or open the SQLite store.
    StoreInit(PathBuf),
    /// Ingest one dataset into the SQLite store.
    StoreIngest(StoreIngestArgs),
    /// Fetch a Dukascopy range and write a CSV and optional PNG.
    Fetch(FetchArgs),
    /// Print the command reference.
    Help,
    /// Exit the shell.
    Quit,
}

/// A model axis selected by the `axis` command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Axis {
    /// Direction.
    Direction,
    /// Momentum.
    Momentum,
    /// Structure.
    Structure,
    /// Progression.
    Progression,
}

impl Axis {
    pub(crate) fn from_code(code: &str) -> Option<Self> {
        match code {
            "direction" => Some(Self::Direction),
            "momentum" => Some(Self::Momentum),
            "structure" => Some(Self::Structure),
            "progression" => Some(Self::Progression),
            _ => None,
        }
    }
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

/// A parsed `research export` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResearchExportArgs {
    /// The M1 source CSV.
    pub(crate) path: PathBuf,
    /// The dataset directory name.
    pub(crate) dataset: String,
    /// The caller-supplied data version.
    pub(crate) data_version: String,
    /// The export root.
    pub(crate) root: PathBuf,
}

/// A parsed `store ingest` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoreIngestArgs {
    /// The M1 source CSV.
    pub(crate) path: PathBuf,
    /// The unique dataset name.
    pub(crate) dataset: String,
    /// The caller-supplied data version.
    pub(crate) data_version: String,
    /// The database path.
    pub(crate) database: PathBuf,
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
    /// `axis` needs a CSV path and an axis name.
    AxisUsage,
    /// The axis name is not known.
    UnknownAxis(String),
    /// A `research export` argument list is invalid.
    Research(String),
    /// A `store` argument list is invalid.
    Store(String),
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
            Self::AxisUsage => write!(
                formatter,
                "usage: axis <csv-path> <direction|momentum|structure|progression>"
            ),
            Self::UnknownAxis(axis) => write!(
                formatter,
                "unknown axis '{axis}'; use direction, momentum, structure, or progression"
            ),
            Self::Research(message) => formatter.write_str(message),
            Self::Store(message) => formatter.write_str(message),
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
    /// The research layers could not be built.
    Layers(LayerError),
    /// The research dataset could not be rendered.
    ResearchExport(ResearchExportError),
    /// A research file could not be written.
    Write(FileExportError),
    /// The store could not be opened or queried.
    Store(StoreError),
    /// The output could not be written.
    Output(io::Error),
}

impl fmt::Display for RunError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Source(source) => source.fmt(formatter),
            Self::Fetch(source) => source.fmt(formatter),
            Self::Layers(source) => write!(formatter, "cannot build the research layers: {source}"),
            Self::ResearchExport(source) => {
                write!(formatter, "cannot render the research dataset: {source}")
            }
            Self::Write(source) => write!(formatter, "cannot write the research dataset: {source}"),
            Self::Store(source) => source.fmt(formatter),
            Self::Output(source) => write!(formatter, "cannot write output: {source}"),
        }
    }
}

impl std::error::Error for RunError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Source(source) => Some(source),
            Self::Fetch(source) => Some(source),
            Self::Layers(source) => Some(source),
            Self::ResearchExport(source) => Some(source),
            Self::Write(source) => Some(source),
            Self::Store(source) => Some(source),
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
        "timeframes" => path_command(rest, "timeframes", Command::Timeframes),
        "market-state" => path_command(rest, "market-state", Command::MarketState),
        "events" => path_command(rest, "events", Command::Events),
        "legs" => path_command(rest, "legs", Command::Legs),
        "axis" => parse_axis(rest),
        "research" => {
            let arguments = tokenize(rest)?;
            parse_research_arguments(&arguments)
        }
        "store" => {
            let arguments = tokenize(rest)?;
            parse_store_arguments(&arguments)
        }
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

fn parse_axis(rest: &str) -> Result<Option<Command>, ParseError> {
    let (path, axis) = rest
        .rsplit_once(char::is_whitespace)
        .ok_or(ParseError::AxisUsage)?;
    let path = path.trim();
    let axis = axis.trim();
    if path.is_empty() || axis.is_empty() {
        return Err(ParseError::AxisUsage);
    }
    let axis = Axis::from_code(axis).ok_or_else(|| ParseError::UnknownAxis(axis.to_owned()))?;
    Ok(Some(Command::Axis {
        path: PathBuf::from(path),
        axis,
    }))
}

/// Parses the arguments after `research`.
///
/// The shape is `research export` with a CSV path, a dataset name, a required
/// `--data-version`, and an optional `--root`. This entry point is also used by
/// the non-interactive binary, where the operating system has already split the
/// arguments.
///
/// # Errors
///
/// Returns [`ParseError::Research`] for a missing subcommand, a missing
/// argument, an unknown option, or a repeated positional argument.
pub(crate) fn parse_research_arguments(
    arguments: &[String],
) -> Result<Option<Command>, ParseError> {
    const USAGE: &str = "usage: research export <csv-path> <dataset> --data-version <id> [--root <dir>] | research summary <csv-path>";
    if arguments.first().map(String::as_str) == Some("summary") {
        if arguments.len() != 2 {
            return Err(ParseError::Research(
                "usage: research summary <csv-path>".to_string(),
            ));
        }
        return Ok(Some(Command::ResearchSummary(PathBuf::from(&arguments[1]))));
    }
    if arguments.first().map(String::as_str) != Some("export") {
        return Err(ParseError::Research(USAGE.to_string()));
    }

    let mut path = None;
    let mut dataset = None;
    let mut data_version = None;
    let mut root = PathBuf::from("data");
    let mut index = 1;

    while index < arguments.len() {
        match arguments[index].as_str() {
            "--data-version" => {
                index += 1;
                let value = arguments.get(index).ok_or_else(|| {
                    ParseError::Research(format!("--data-version needs a value\n{USAGE}"))
                })?;
                data_version = Some(value.clone());
            }
            "--root" => {
                index += 1;
                let value = arguments.get(index).ok_or_else(|| {
                    ParseError::Research(format!("--root needs a value\n{USAGE}"))
                })?;
                root = PathBuf::from(value);
            }
            option if option.starts_with("--") => {
                return Err(ParseError::Research(format!(
                    "unknown option '{option}'\n{USAGE}"
                )));
            }
            value => {
                if path.is_none() {
                    path = Some(PathBuf::from(value));
                } else if dataset.is_none() {
                    dataset = Some(value.to_string());
                } else {
                    return Err(ParseError::Research(format!(
                        "unexpected argument '{value}'\n{USAGE}"
                    )));
                }
            }
        }
        index += 1;
    }

    let path = path.ok_or_else(|| ParseError::Research(format!("missing CSV path\n{USAGE}")))?;
    let dataset =
        dataset.ok_or_else(|| ParseError::Research(format!("missing dataset name\n{USAGE}")))?;
    let data_version = data_version
        .ok_or_else(|| ParseError::Research(format!("missing --data-version\n{USAGE}")))?;

    Ok(Some(Command::ResearchExport(ResearchExportArgs {
        path,
        dataset,
        data_version,
        root,
    })))
}

/// Parses the arguments after `store`.
///
/// The subcommands in this issue are `init` and `ingest`. Later issues add
/// `list`, `show`, and `verify`.
///
/// # Errors
///
/// Returns [`ParseError::Store`] for a missing or unknown subcommand, a missing
/// option value, or a repeated option.
pub(crate) fn parse_store_arguments(arguments: &[String]) -> Result<Option<Command>, ParseError> {
    const USAGE: &str = "usage: store init [--db <path>] | store ingest <csv-path> --dataset <name> --data-version <id> [--db <path>]";
    match arguments.first().map(String::as_str) {
        Some("init") => parse_store_init(arguments, USAGE),
        Some("ingest") => parse_store_ingest(arguments, USAGE),
        _ => Err(ParseError::Store(USAGE.to_string())),
    }
}

fn parse_store_init(arguments: &[String], usage: &str) -> Result<Option<Command>, ParseError> {
    let mut path = PathBuf::from("data/quantrule.db");
    let mut index = 1;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--db" => {
                index += 1;
                let value = arguments
                    .get(index)
                    .ok_or_else(|| ParseError::Store(format!("--db needs a value\n{usage}")))?;
                if value.is_empty() {
                    return Err(ParseError::Store(format!(
                        "--db needs a non-empty value\n{usage}"
                    )));
                }
                path = PathBuf::from(value);
            }
            option => {
                return Err(ParseError::Store(format!(
                    "unknown argument '{option}'\n{usage}"
                )));
            }
        }
        index += 1;
    }

    Ok(Some(Command::StoreInit(path)))
}

fn parse_store_ingest(arguments: &[String], usage: &str) -> Result<Option<Command>, ParseError> {
    let mut path = None;
    let mut dataset = None;
    let mut data_version = None;
    let mut database = PathBuf::from("data/quantrule.db");
    let mut index = 1;

    while index < arguments.len() {
        match arguments[index].as_str() {
            "--dataset" => {
                index += 1;
                let value = arguments.get(index).ok_or_else(|| {
                    ParseError::Store(format!("--dataset needs a value\n{usage}"))
                })?;
                if value.is_empty() {
                    return Err(ParseError::Store(format!(
                        "--dataset needs a non-empty value\n{usage}"
                    )));
                }
                if dataset.is_some() {
                    return Err(ParseError::Store(format!(
                        "--dataset was supplied twice\n{usage}"
                    )));
                }
                dataset = Some(value.clone());
            }
            "--data-version" => {
                index += 1;
                let value = arguments.get(index).ok_or_else(|| {
                    ParseError::Store(format!("--data-version needs a value\n{usage}"))
                })?;
                if value.is_empty() {
                    return Err(ParseError::Store(format!(
                        "--data-version needs a non-empty value\n{usage}"
                    )));
                }
                if data_version.is_some() {
                    return Err(ParseError::Store(format!(
                        "--data-version was supplied twice\n{usage}"
                    )));
                }
                data_version = Some(value.clone());
            }
            "--db" => {
                index += 1;
                let value = arguments
                    .get(index)
                    .ok_or_else(|| ParseError::Store(format!("--db needs a value\n{usage}")))?;
                if value.is_empty() {
                    return Err(ParseError::Store(format!(
                        "--db needs a non-empty value\n{usage}"
                    )));
                }
                database = PathBuf::from(value);
            }
            option if option.starts_with("--") => {
                return Err(ParseError::Store(format!(
                    "unknown option '{option}'\n{usage}"
                )));
            }
            value => {
                if path.is_none() {
                    path = Some(PathBuf::from(value));
                } else {
                    return Err(ParseError::Store(format!(
                        "unexpected argument '{value}'\n{usage}"
                    )));
                }
            }
        }
        index += 1;
    }

    let path = path.ok_or_else(|| ParseError::Store(format!("missing CSV path\n{usage}")))?;
    let dataset =
        dataset.ok_or_else(|| ParseError::Store(format!("missing --dataset\n{usage}")))?;
    let data_version = data_version
        .ok_or_else(|| ParseError::Store(format!("missing --data-version\n{usage}")))?;

    Ok(Some(Command::StoreIngest(StoreIngestArgs {
        path,
        dataset,
        data_version,
        database,
    })))
}

/// Creates or opens the store and prints its schema version.
///
/// # Errors
///
/// Returns a [`RunError`] when the store cannot be opened or the writer fails.
pub(crate) fn run_store_init(path: &Path, writer: &mut impl Write) -> Result<(), RunError> {
    let store = Store::open(path).map_err(RunError::Store)?;
    let version = store.schema_version().map_err(RunError::Store)?;
    writeln!(writer, "store {} schema={version}", path.display()).map_err(RunError::Output)?;
    writer.flush().map_err(RunError::Output)
}

/// Builds the layers and writes one dataset to the store.
///
/// The layers are built before the database is opened, so a source, model, or
/// cap failure leaves no database file and no partial dataset. The writer then
/// commits the dataset row and every bar row in one transaction.
///
/// # Errors
///
/// Returns a [`RunError`] when the source, the layer build, the store, or the
/// writer fails.
pub(crate) fn run_store_ingest(
    args: &StoreIngestArgs,
    writer: &mut impl Write,
) -> Result<(), RunError> {
    let LayerBuild { layers, m1 } = load_layers(&args.path)?;
    let dataset = DatasetRecord {
        name: args.dataset.clone(),
        data_version: args.data_version.clone(),
        model_version: env!("CARGO_PKG_VERSION").to_string(),
        structure_scale_code: PivotScale::Major.code(),
        m1_bars: m1.len(),
    };

    let mut store = Store::open(&args.database).map_err(RunError::Store)?;
    let report = store
        .ingest(&dataset, &layers.layer1)
        .map_err(RunError::Store)?;

    let verb = if report.unchanged {
        "unchanged"
    } else {
        "ingested"
    };
    writeln!(
        writer,
        "{verb} {} bars={} timeframes={}",
        args.dataset, report.bars, report.timeframes
    )
    .map_err(RunError::Output)?;
    writer.flush().map_err(RunError::Output)
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

/// Builds M5, M15, and H1 from an M1 CSV and writes the series and report.
///
/// The NDJSON is flushed before the report is written. A source-level failure
/// leaves stdout untouched because the timeframe set is built before rendering.
///
/// # Errors
///
/// Returns a [`RunError`] when the source fails or either writer fails.
pub(crate) fn run_timeframes(
    path: &Path,
    output: &mut impl Write,
    report: &mut impl Write,
) -> Result<(), RunError> {
    let set = csv::load_timeframes(path).map_err(RunError::Source)?;
    render::write_timeframes(&set, output).map_err(RunError::Output)?;
    output.flush().map_err(RunError::Output)?;
    render::write_timeframe_report(&set, report).map_err(RunError::Output)?;
    report.flush().map_err(RunError::Output)
}

/// Prints the assembled market state for the M1/M5/M15/H1 ladder.
///
/// # Errors
///
/// Returns a [`RunError`] when the source or writer fails.
pub(crate) fn run_market_state(path: &Path, writer: &mut impl Write) -> Result<(), RunError> {
    let state = load_market_state(path)?;
    render_state::write_market_state(&state, writer).map_err(RunError::Output)?;
    writer.flush().map_err(RunError::Output)
}

/// Prints one axis per timeframe for the ladder.
///
/// # Errors
///
/// Returns a [`RunError`] when the source or writer fails.
pub(crate) fn run_axis(path: &Path, axis: Axis, writer: &mut impl Write) -> Result<(), RunError> {
    let state = load_market_state(path)?;
    render_state::write_axis(&state, axis, writer).map_err(RunError::Output)?;
    writer.flush().map_err(RunError::Output)
}

/// Prints every event record in availability order.
///
/// # Errors
///
/// Returns a [`RunError`] when the source or writer fails.
pub(crate) fn run_events(path: &Path, writer: &mut impl Write) -> Result<(), RunError> {
    let state = load_market_state(path)?;
    render_state::write_events(&state, writer).map_err(RunError::Output)?;
    writer.flush().map_err(RunError::Output)
}

/// Prints the minor and major leg per timeframe.
///
/// # Errors
///
/// Returns a [`RunError`] when the source or writer fails.
pub(crate) fn run_legs(path: &Path, writer: &mut impl Write) -> Result<(), RunError> {
    let state = load_market_state(path)?;
    render_state::write_legs(&state, writer).map_err(RunError::Output)?;
    writer.flush().map_err(RunError::Output)
}

fn load_market_state(path: &Path) -> Result<MarketState, RunError> {
    let set = csv::load_timeframes(path).map_err(RunError::Source)?;
    let m1: Vec<Candle> = set.series(Timeframe::M1).iter().copied().collect();
    let m5: Vec<Candle> = set.series(Timeframe::M5).iter().copied().collect();
    let m15: Vec<Candle> = set.series(Timeframe::M15).iter().copied().collect();
    let h1: Vec<Candle> = set.series(Timeframe::H1).iter().copied().collect();
    Ok(MarketState::compute(
        &[
            TimeframeInput {
                timeframe_secs: Timeframe::M1.seconds() as u64,
                candles: &m1,
            },
            TimeframeInput {
                timeframe_secs: Timeframe::M5.seconds() as u64,
                candles: &m5,
            },
            TimeframeInput {
                timeframe_secs: Timeframe::M15.seconds() as u64,
                candles: &m15,
            },
            TimeframeInput {
                timeframe_secs: Timeframe::H1.seconds() as u64,
                candles: &h1,
            },
        ],
        &MarketStateConfig::default(),
    ))
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

/// One layer build and its accepted M1 candles.
struct LayerBuild {
    layers: LayerSet,
    m1: Vec<Candle>,
}

/// Loads an M1 CSV and builds the three layers.
fn load_layers(path: &Path) -> Result<LayerBuild, RunError> {
    let set = csv::load_full_timeframes(path).map_err(RunError::Source)?;
    let m1: Vec<Candle> = set.series(Timeframe::M1).iter().copied().collect();
    let m5: Vec<Candle> = set.series(Timeframe::M5).iter().copied().collect();
    let m15: Vec<Candle> = set.series(Timeframe::M15).iter().copied().collect();
    let h1: Vec<Candle> = set.series(Timeframe::H1).iter().copied().collect();

    let layers = build_layers(
        &[
            TimeframeInput {
                timeframe_secs: u64::from(Timeframe::M1.seconds()),
                candles: &m1,
            },
            TimeframeInput {
                timeframe_secs: u64::from(Timeframe::M5.seconds()),
                candles: &m5,
            },
            TimeframeInput {
                timeframe_secs: u64::from(Timeframe::M15.seconds()),
                candles: &m15,
            },
            TimeframeInput {
                timeframe_secs: u64::from(Timeframe::H1.seconds()),
                candles: &h1,
            },
        ],
        &LayerConfig::default(),
    )
    .map_err(RunError::Layers)?;

    Ok(LayerBuild { layers, m1 })
}

/// Prints the deterministic summary of the research layers.
///
/// # Errors
///
/// Returns a [`RunError`] when the source, the layer build, or the writer fails.
pub(crate) fn run_research_summary(path: &Path, writer: &mut impl Write) -> Result<(), RunError> {
    let LayerBuild { layers, .. } = load_layers(path)?;
    let summary = summarize(&layers);
    write_summary(&summary, writer).map_err(RunError::Output)?;
    writer.flush().map_err(RunError::Output)
}

fn write_summary(summary: &DatasetSummary, writer: &mut impl Write) -> io::Result<()> {
    writeln!(
        writer,
        "summary layer1={} layer2={} layer3={}",
        summary.layer1_rows, summary.layer2_rows, summary.layer3_rows
    )?;
    for axis in &summary.axes {
        write!(
            writer,
            "axis timeframe={} axis={} n={} null={} min={} max={} mean={} ordinals=",
            axis.timeframe_secs,
            axis.axis,
            axis.count,
            axis.nulls,
            number(axis.min),
            number(axis.max),
            number(axis.mean)
        )?;
        for (index, count) in axis.ordinal_counts.iter().enumerate() {
            if index > 0 {
                write!(writer, ",")?;
            }
            write!(writer, "{count}")?;
        }
        writeln!(writer, " ordinal_null={}", axis.ordinal_nulls)?;
    }
    for relation in &summary.relations {
        writeln!(
            writer,
            "relation timeframe={} left={} right={} n={} r={}",
            relation.timeframe_secs,
            relation.left,
            relation.right,
            relation.count,
            number(relation.correlation)
        )?;
    }
    Ok(())
}

fn number(value: Option<f64>) -> String {
    value
        .map(export::format_number)
        .unwrap_or_else(|| "null".to_string())
}

/// Writes the research dataset for an M1 CSV.
///
/// The dataset is rendered before any file is written, so a render or
/// validation failure writes nothing. Each file is then written atomically.
///
/// # Errors
///
/// Returns a [`RunError`] when the source, the layer build, the render, or a
/// file write fails.
pub(crate) fn run_research_export(
    args: &ResearchExportArgs,
    writer: &mut impl Write,
) -> Result<(), RunError> {
    let LayerBuild { layers, m1 } = load_layers(&args.path)?;

    let meta = DatasetMeta {
        root: args.root.display().to_string(),
        dataset: args.dataset.clone(),
        data_version: args.data_version.clone(),
        model_version: env!("CARGO_PKG_VERSION").to_string(),
        structure_scale_code: PivotScale::Major as u8,
    };
    let bundle = render_dataset(&layers, &m1, &meta).map_err(RunError::ResearchExport)?;

    for file in &bundle.files {
        export::write_atomic(Path::new(&file.path), file.contents.as_bytes())
            .map_err(RunError::Write)?;
    }

    writeln!(writer, "exported {}", args.dataset).map_err(RunError::Output)?;
    writeln!(writer, "  data version: {}", args.data_version).map_err(RunError::Output)?;
    writeln!(writer, "  raw rows: {}", bundle.raw_rows).map_err(RunError::Output)?;
    writeln!(writer, "  layer1 rows: {}", bundle.layer1_rows).map_err(RunError::Output)?;
    writeln!(writer, "  layer2 rows: {}", bundle.layer2_rows).map_err(RunError::Output)?;
    writeln!(writer, "  layer3 rows: {}", bundle.layer3_rows).map_err(RunError::Output)?;
    writeln!(writer, "  files: {}", bundle.files.len()).map_err(RunError::Output)?;
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
    use super::{
        Command, FetchArgs, ParseError, ResearchExportArgs, StoreIngestArgs, parse,
        parse_fetch_arguments,
    };
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
    fn parses_timeframes_with_a_path() {
        assert_eq!(
            parse("timeframes data/example.csv"),
            Ok(Some(Command::Timeframes(PathBuf::from("data/example.csv"))))
        );
    }

    #[test]
    fn preserves_spaces_in_a_timeframes_path() {
        assert_eq!(
            parse("timeframes my file.csv"),
            Ok(Some(Command::Timeframes(PathBuf::from("my file.csv"))))
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
    fn rejects_timeframes_without_a_path() {
        assert_eq!(
            parse("timeframes"),
            Err(ParseError::MissingPath("timeframes"))
        );
        assert_eq!(
            parse("timeframes   "),
            Err(ParseError::MissingPath("timeframes"))
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
    fn parses_the_market_state_commands() {
        assert_eq!(
            parse("market-state data/example.csv"),
            Ok(Some(Command::MarketState(PathBuf::from(
                "data/example.csv"
            ))))
        );
        assert_eq!(
            parse("events data/example.csv"),
            Ok(Some(Command::Events(PathBuf::from("data/example.csv"))))
        );
        assert_eq!(
            parse("legs data/example.csv"),
            Ok(Some(Command::Legs(PathBuf::from("data/example.csv"))))
        );
    }

    #[test]
    fn parses_an_axis_with_a_path_and_a_name() {
        assert_eq!(
            parse("axis data/example.csv direction"),
            Ok(Some(Command::Axis {
                path: PathBuf::from("data/example.csv"),
                axis: super::Axis::Direction,
            }))
        );
        assert_eq!(
            parse("axis my data.csv progression"),
            Ok(Some(Command::Axis {
                path: PathBuf::from("my data.csv"),
                axis: super::Axis::Progression,
            }))
        );
    }

    #[test]
    fn rejects_a_bad_axis_command() {
        assert_eq!(parse("axis"), Err(ParseError::AxisUsage));
        assert_eq!(parse("axis data/example.csv"), Err(ParseError::AxisUsage));
        assert_eq!(
            parse("axis data/example.csv bogus"),
            Err(ParseError::UnknownAxis(String::from("bogus")))
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

    #[test]
    fn parses_a_research_export() {
        assert_eq!(
            parse("research export data/m1.csv demo --data-version source:1 --root out"),
            Ok(Some(Command::ResearchExport(ResearchExportArgs {
                path: PathBuf::from("data/m1.csv"),
                dataset: "demo".to_string(),
                data_version: "source:1".to_string(),
                root: PathBuf::from("out"),
            })))
        );
    }

    #[test]
    fn a_research_export_requires_a_data_version() {
        assert!(matches!(
            parse("research export data/m1.csv demo"),
            Err(ParseError::Research(_))
        ));
    }

    #[test]
    fn a_research_export_rejects_an_unknown_option() {
        assert!(matches!(
            parse("research export data/m1.csv demo --data-version x --nope y"),
            Err(ParseError::Research(_))
        ));
    }

    #[test]
    fn parses_a_store_ingest_with_defaults() {
        assert_eq!(
            parse("store ingest data/m1.csv --dataset demo --data-version source:1"),
            Ok(Some(Command::StoreIngest(StoreIngestArgs {
                path: PathBuf::from("data/m1.csv"),
                dataset: "demo".to_string(),
                data_version: "source:1".to_string(),
                database: PathBuf::from("data/quantrule.db"),
            })))
        );
    }

    #[test]
    fn parses_a_store_ingest_in_any_option_order() {
        assert_eq!(
            parse("store ingest --db other.db --data-version source:1 data/m1.csv --dataset demo"),
            Ok(Some(Command::StoreIngest(StoreIngestArgs {
                path: PathBuf::from("data/m1.csv"),
                dataset: "demo".to_string(),
                data_version: "source:1".to_string(),
                database: PathBuf::from("other.db"),
            })))
        );
    }

    #[test]
    fn a_store_ingest_requires_a_dataset_and_a_data_version() {
        assert!(matches!(
            parse("store ingest data/m1.csv --data-version source:1"),
            Err(ParseError::Store(_))
        ));
        assert!(matches!(
            parse("store ingest data/m1.csv --dataset demo"),
            Err(ParseError::Store(_))
        ));
        assert!(matches!(
            parse("store ingest --dataset demo --data-version source:1"),
            Err(ParseError::Store(_))
        ));
    }

    #[test]
    fn a_store_ingest_rejects_repeated_options() {
        for line in [
            "store ingest data/m1.csv --dataset a --dataset b --data-version v",
            "store ingest data/m1.csv --dataset a --data-version v --data-version w",
        ] {
            assert!(
                matches!(parse(line), Err(ParseError::Store(_))),
                "{line} should be rejected"
            );
        }
    }

    #[test]
    fn a_store_ingest_rejects_an_unknown_option() {
        assert!(matches!(
            parse("store ingest data/m1.csv --dataset demo --data-version v --nope x"),
            Err(ParseError::Store(_))
        ));
    }

    #[test]
    fn parses_a_store_init_with_defaults() {
        assert_eq!(
            parse("store init"),
            Ok(Some(Command::StoreInit(PathBuf::from("data/quantrule.db"))))
        );
    }
}
