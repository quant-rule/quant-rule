//! The command vocabulary shared by every command family.
//!
//! The types in this module describe a parsed command and its failures. They
//! contain no parsing or rendering behavior, so a command family can depend on
//! the vocabulary without depending on another family.

use std::fmt;
use std::io;
use std::path::PathBuf;

use quantrule_analytics::layers::LayerError;
use quantrule_analytics::research::ExportError as ResearchExportError;
use quantrule_data::Timeframe;

use crate::chart;
use crate::csv::SourceError;
use crate::export::ExportError as FileExportError;
use crate::fetch;
use crate::source::{DateRange, Symbol};
use crate::store::{StoreError, Table};

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
    MarketState(ModelArgs),
    /// Render the last candles with the four axis panels.
    MarketChart(MarketChartArgs),
    /// Print one axis per timeframe.
    Axis { args: ModelArgs, axis: Axis },
    /// Print the event records.
    Events(ModelArgs),
    /// Print the legs.
    Legs(ModelArgs),
    /// Print the last N complete model bars.
    ModelSeries(ModelSeriesArgs),
    /// Write the research dataset.
    ResearchExport(ResearchExportArgs),
    /// Print the research summary.
    ResearchSummary(PathBuf),
    /// Create or open the SQLite store.
    StoreInit(PathBuf),
    /// Ingest one dataset into the SQLite store.
    StoreIngest(StoreIngestArgs),
    /// Verify one stored dataset against a fresh build.
    StoreVerify(StoreVerifyArgs),
    /// List the stored datasets.
    StoreList(StoreListArgs),
    /// Show one stored table.
    StoreShow(StoreShowArgs),
    /// Remove one stored dataset.
    StoreRemove(StoreRemoveArgs),
    /// Read one stored per-bar view.
    Db(DbArgs),
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

/// The input window used by a model analysis command.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum WindowMode {
    /// Use the full supported series, up to the dataset cap.
    #[default]
    Full,
    /// Use the last [`quantrule_data::DEFAULT_WINDOW`] candles per timeframe.
    Latest,
}

impl WindowMode {
    pub(crate) fn from_code(code: &str) -> Option<Self> {
        match code {
            "full" => Some(Self::Full),
            "latest" => Some(Self::Latest),
            _ => None,
        }
    }
}

/// The path and input window shared by the model analysis commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelArgs {
    /// The M1 source CSV.
    pub(crate) path: PathBuf,
    /// The input window.
    pub(crate) window: WindowMode,
}

impl ModelArgs {
    pub(crate) fn new(path: PathBuf, window: WindowMode) -> Self {
        Self { path, window }
    }
}

/// A parsed `model-series` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelSeriesArgs {
    /// The M1 source CSV.
    pub(crate) path: PathBuf,
    /// The input window.
    pub(crate) window: WindowMode,
    /// One timeframe, or `None` for the whole ladder.
    pub(crate) timeframe: Option<Timeframe>,
    /// The number of bars per timeframe.
    pub(crate) bars: usize,
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

/// A parsed `store remove` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoreRemoveArgs {
    /// The dataset name to remove.
    pub(crate) dataset: String,
    /// The database path.
    pub(crate) database: PathBuf,
}

/// A parsed `market-chart` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MarketChartArgs {
    /// The M1 source CSV.
    pub(crate) path: PathBuf,
    /// The PNG target.
    pub(crate) png: PathBuf,
    /// The charted timeframe.
    pub(crate) timeframe: Timeframe,
    /// The number of bars to draw.
    pub(crate) candles: usize,
    /// The input window used to build the chart.
    pub(crate) window: WindowMode,
}

/// Arguments for `store list`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoreListArgs {
    /// The database path.
    pub(crate) database: PathBuf,
}

/// Arguments for `store show`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoreShowArgs {
    /// The dataset name.
    pub(crate) dataset: String,
    /// The table to show.
    pub(crate) table: Table,
    /// The optional timeframe filter in seconds.
    pub(crate) timeframe: Option<u64>,
    /// The optional bar-index filter.
    pub(crate) bar: Option<usize>,
    /// The maximum rows to print.
    pub(crate) limit: usize,
    /// The database path.
    pub(crate) database: PathBuf,
}

/// The per-bar view selected by `db`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DbKind {
    /// Candle-only series.
    Series,
    /// Full `bar` rows.
    Bars,
    /// Metrics and resolutions.
    Metrics,
    /// Materialized Layer 3 rows.
    Layer3,
}

/// Arguments for one `db` view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DbArgs {
    /// Which view to render.
    pub(crate) kind: DbKind,
    /// The dataset name.
    pub(crate) dataset: String,
    /// The optional timeframe filter in seconds.
    pub(crate) timeframe: Option<u64>,
    /// The optional bar-index filter.
    pub(crate) bar: Option<usize>,
    /// The maximum rows to print.
    pub(crate) limit: usize,
    /// The database path.
    pub(crate) database: PathBuf,
}

/// Arguments for `store verify`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoreVerifyArgs {
    /// The M1 source CSV.
    pub(crate) path: PathBuf,
    /// The dataset name.
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
    /// A `market-chart` argument list is invalid.
    MarketChart(String),
    /// `axis` needs a CSV path and an axis name.
    AxisUsage,
    /// The axis name is not known.
    UnknownAxis(String),
    /// A model command window option is invalid.
    Window(String),
    /// A simple command received the wrong number of arguments.
    Usage(String),
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
            Self::MarketChart(message) => formatter.write_str(message),
            Self::AxisUsage => write!(
                formatter,
                "usage: axis <csv-path> <direction|momentum|structure|progression>"
            ),
            Self::UnknownAxis(axis) => write!(
                formatter,
                "unknown axis '{axis}'; use direction, momentum, structure, or progression"
            ),
            Self::Window(message) => formatter.write_str(message),
            Self::Usage(message) => formatter.write_str(message),
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
    /// The market chart could not be rendered.
    Chart(chart::ChartError),
    /// The market chart could not be written.
    ChartWrite(FileExportError),
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
            Self::Chart(source) => write!(formatter, "cannot render the market chart: {source}"),
            Self::ChartWrite(source) => write!(formatter, "cannot write the chart PNG: {source}"),
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
            Self::Chart(source) => Some(source),
            Self::ChartWrite(source) => Some(source),
            Self::Layers(source) => Some(source),
            Self::ResearchExport(source) => Some(source),
            Self::Write(source) => Some(source),
            Self::Store(source) => Some(source),
            Self::Output(source) => Some(source),
        }
    }
}
