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
use crate::config::ConfigError;
use crate::csv::SourceError;
use crate::export::ExportError as FileExportError;
use crate::fetch;
use crate::lab_summary::LabSummaryError;
use crate::render_report::BundleError;
use crate::source::{DateBounds, DateRange, Symbol};
use crate::store::{StoreError, Table};
use quantrule_backtest::runner::BacktestError;

pub(crate) use crate::lab_summary::{LabOperator, LabRole, LabSummaryArgs};
pub(crate) use crate::render_state::Axis;

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
    /// Print the four-axis dashboard.
    MarketDashboard(DashboardArgs),
    /// Render the last candles with the four axis panels.
    MarketChart(MarketChartArgs),
    /// Write the full-period market report and its charts.
    MarketReport(MarketReportArgs),
    /// Write one config-driven report bundle.
    Report(ReportArgs),
    /// Run the full pipeline and write all five bundles.
    Backtest(BacktestArgs),
    /// Write a deterministic summary for a lab result folder.
    LabSummary(LabSummaryArgs),
    /// Print strategy entries and exits without fills or P&L.
    StrategyRun(StrategyRunArgs),
    /// Print the strategy rule in human trader words.
    StrategyExplain(StrategyExplainArgs),
    /// Validate the policy and every engine config against it.
    PolicyCheck(PolicyCheckArgs),
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
    /// Print the command reference, one command, or a reference topic.
    Help(HelpTopic),
    /// Exit the shell.
    Quit,
}

/// A topic selected by the `help` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum HelpTopic {
    /// The one-screen command map.
    Short,
    /// Every command and option.
    All,
    /// The precise-term glossary.
    Glossary,
    /// The full-pipeline tutorial map.
    Tutorial,
    /// The quant role map.
    Roles,
    /// One command, named by one or two words.
    Command(String),
}

pub(crate) use crate::pipeline::WindowMode;

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

/// A parsed `market-dashboard` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DashboardArgs {
    /// The M1 source CSV.
    pub(crate) path: PathBuf,
    /// The input window.
    pub(crate) window: WindowMode,
    /// Whether to print the numeric JSON view instead of the text dashboard.
    pub(crate) json: bool,
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

/// A parsed `market-report` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MarketReportArgs {
    /// The M1 source CSV.
    pub(crate) path: PathBuf,
    /// The caller-supplied identity of the input series.
    pub(crate) data_version: String,
    /// The exact `--from` value, when supplied.
    pub(crate) from: Option<String>,
    /// The exact `--to` value, when supplied.
    pub(crate) to: Option<String>,
    /// The parsed inclusive UTC bounds.
    pub(crate) bounds: DateBounds,
    /// The output directory.
    pub(crate) out: PathBuf,
    /// Whether to print the `report.json` bytes to stdout.
    pub(crate) json: bool,
}

impl MarketReportArgs {
    /// The requested period as a stable one-line label.
    pub(crate) fn requested_period(&self) -> String {
        match (&self.from, &self.to) {
            (Some(from), Some(to)) => format!("{from}..{to}"),
            (Some(from), None) => format!("{from}..open"),
            (None, Some(to)) => format!("open..{to}"),
            (None, None) => String::from("full file"),
        }
    }
}

/// A report bundle selected by `report`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReportEngine {
    /// The model report.
    Model,
    /// The strategy report.
    Strategy,
    /// The execution report.
    Execution,
    /// The portfolio report.
    Portfolio,
    /// The analytical report.
    Analytical,
}

impl ReportEngine {
    /// Parses the report name exactly.
    pub(crate) fn parse(code: &str) -> Option<Self> {
        match code {
            "model" => Some(Self::Model),
            "strategy" => Some(Self::Strategy),
            "execution" => Some(Self::Execution),
            "portfolio" => Some(Self::Portfolio),
            "analytical" => Some(Self::Analytical),
            _ => None,
        }
    }
}

/// A parsed `backtest` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BacktestArgs {
    /// The run configuration.
    pub(crate) config: PathBuf,
    /// The output-root override, when supplied.
    pub(crate) out: Option<PathBuf>,
    /// Whether to print the summary envelope.
    pub(crate) json: bool,
}

/// A parsed `report` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReportArgs {
    /// The report bundle to write.
    pub(crate) engine: ReportEngine,
    /// The run configuration.
    pub(crate) config: PathBuf,
    /// The output-root override, when supplied.
    pub(crate) out: Option<PathBuf>,
    /// Whether to print the `report.json` bytes to stdout.
    pub(crate) json: bool,
}

/// A parsed `strategy-run` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StrategyRunArgs {
    /// The M1 source CSV.
    pub(crate) path: PathBuf,
    /// An optional run configuration; when supplied, its strategy is used.
    pub(crate) config: Option<PathBuf>,
    /// The strategy name.
    pub(crate) strategy: String,
    /// The caller-supplied identity of the input series.
    pub(crate) data_version: String,
    /// The exact `--from` value, when supplied.
    pub(crate) from: Option<String>,
    /// The exact `--to` value, when supplied.
    pub(crate) to: Option<String>,
    /// The parsed inclusive UTC bounds.
    pub(crate) bounds: DateBounds,
    /// Whether to print the provenance envelope instead of bare NDJSON.
    pub(crate) json: bool,
}

impl StrategyRunArgs {
    /// The requested period as a stable one-line label.
    pub(crate) fn requested_period(&self) -> String {
        match (&self.from, &self.to) {
            (Some(from), Some(to)) => format!("{from}..{to}"),
            (Some(from), None) => format!("{from}..open"),
            (None, Some(to)) => format!("open..{to}"),
            (None, None) => String::from("full file"),
        }
    }
}

/// A parsed `strategy explain` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StrategyExplainArgs {
    /// The run configuration that names the strategy and policy.
    pub(crate) config: PathBuf,
}

/// A parsed `policy check` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PolicyCheckArgs {
    /// The run configuration that names the policy and engine configs.
    pub(crate) config: PathBuf,
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
    /// A `market-report` argument list is invalid.
    MarketReport(String),
    /// A `strategy-run` argument list is invalid.
    StrategyRun(String),
    /// A `strategy explain` argument list is invalid.
    StrategyExplain(String),
    /// A `policy` argument list is invalid.
    Policy(String),
    /// A `report` argument list is invalid.
    Report(String),
    /// A `backtest` argument list is invalid.
    Backtest(String),
    /// A `lab summary` argument list is invalid.
    Lab(String),
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
    /// The help topic is not a command or a reference topic.
    UnknownHelpTopic(String),
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
            Self::MarketReport(message) => formatter.write_str(message),
            Self::StrategyRun(message) => formatter.write_str(message),
            Self::StrategyExplain(message) => formatter.write_str(message),
            Self::Policy(message) => formatter.write_str(message),
            Self::Report(message) => formatter.write_str(message),
            Self::Backtest(message) => formatter.write_str(message),
            Self::Lab(message) => formatter.write_str(message),
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
            Self::UnknownHelpTopic(topic) => write!(
                formatter,
                "unknown help topic '{topic}'; type 'help all' for every command and reference topic"
            ),
        }
    }
}

impl std::error::Error for ParseError {}

/// A `market-report` run could not be completed.
#[derive(Debug)]
pub(crate) enum ReportError {
    /// The output directory exists and is not empty.
    OutputNotEmpty {
        /// The output path.
        path: PathBuf,
    },
    /// The output path exists and is not a directory.
    OutputNotDirectory {
        /// The output path.
        path: PathBuf,
    },
    /// The output directory could not be read.
    ReadOutput {
        /// The output path.
        path: PathBuf,
        /// The underlying I/O error.
        source: io::Error,
    },
    /// The requested period selected no M1 candles.
    EmptyPeriod {
        /// The requested period label.
        period: String,
    },
    /// The report's chart projection has no bars.
    EmptyChart {
        /// The requested period label.
        period: String,
    },
    /// The strategy timeframe has no bars.
    NoStrategyBars {
        /// The strategy timeframe in seconds.
        timeframe_secs: u64,
    },
    /// A date range could not be built.
    Period(String),
    /// The output directory could not be created.
    CreateDirectory {
        /// The output path.
        path: PathBuf,
        /// The underlying I/O error.
        source: io::Error,
    },
    /// One artifact could not be written.
    Artifact {
        /// The artifact path.
        path: PathBuf,
        /// The file export error.
        source: FileExportError,
    },
}

impl fmt::Display for ReportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutputNotEmpty { path } => write!(
                formatter,
                "output directory {} must be absent or empty",
                path.display()
            ),
            Self::OutputNotDirectory { path } => write!(
                formatter,
                "output path {} exists and is not a directory",
                path.display()
            ),
            Self::ReadOutput { path, source } => {
                write!(formatter, "cannot read {}: {source}", path.display())
            }
            Self::EmptyPeriod { period } => {
                write!(
                    formatter,
                    "no M1 candles in the requested period '{period}'"
                )
            }
            Self::EmptyChart { period } => write!(
                formatter,
                "the requested period '{period}' produced no chart bars"
            ),
            Self::NoStrategyBars { timeframe_secs } => write!(
                formatter,
                "the strategy timeframe {timeframe_secs} produced no bars"
            ),
            Self::Period(message) => formatter.write_str(message),
            Self::CreateDirectory { path, source } => {
                write!(formatter, "cannot create {}: {source}", path.display())
            }
            Self::Artifact { path, source } => {
                write!(formatter, "cannot write {}: {source}", path.display())
            }
        }
    }
}

impl std::error::Error for ReportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ReadOutput { source, .. } | Self::CreateDirectory { source, .. } => Some(source),
            Self::Artifact { source, .. } => Some(source),
            Self::OutputNotEmpty { .. }
            | Self::OutputNotDirectory { .. }
            | Self::EmptyPeriod { .. }
            | Self::EmptyChart { .. }
            | Self::NoStrategyBars { .. }
            | Self::Period(_) => None,
        }
    }
}

/// A `strategy-run` could not be completed.
#[derive(Debug)]
pub(crate) enum StrategyRunError {
    /// The requested period selected no M1 candles.
    EmptyPeriod {
        /// The requested period label.
        period: String,
    },
    /// The requested period produced no bars for the strategy timeframe.
    NoTimeframeBars {
        /// The strategy timeframe in seconds.
        timeframe_secs: u64,
        /// The requested period label.
        period: String,
    },
    /// The strategy name is not known to the command.
    UnknownStrategy {
        /// The supplied name.
        name: String,
    },
    /// The strategy timeframe is not one the data engine builds.
    UnsupportedTimeframe {
        /// The strategy timeframe in seconds.
        timeframe_secs: u64,
    },
}

impl fmt::Display for StrategyRunError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPeriod { period } => {
                write!(
                    formatter,
                    "no M1 candles in the requested period '{period}'"
                )
            }
            Self::NoTimeframeBars {
                timeframe_secs,
                period,
            } => write!(
                formatter,
                "the requested period '{period}' produced no {timeframe_secs}-second bars"
            ),
            Self::UnknownStrategy { name } => {
                write!(formatter, "unknown strategy '{name}'; use ladder-context")
            }
            Self::UnsupportedTimeframe { timeframe_secs } => write!(
                formatter,
                "the strategy timeframe {timeframe_secs} is not built by the data engine"
            ),
        }
    }
}

impl std::error::Error for StrategyRunError {}

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
    /// The market report could not be completed.
    Report(ReportError),
    /// The run configuration could not be loaded.
    Config(ConfigError),
    /// A report bundle could not be written.
    Bundle(BundleError),
    /// The full backtest pipeline could not be completed.
    Backtest(BacktestError),
    /// The lab result summary could not be completed.
    Lab(LabSummaryError),
    /// The strategy run could not be completed.
    StrategyRun(StrategyRunError),
    /// The research layers could not be built.
    Layers(LayerError),
    /// The research dataset could not be rendered.
    ResearchExport(ResearchExportError),
    /// A research file could not be written.
    Write(FileExportError),
    /// The store could not be opened or queried.
    Store(StoreError),
    /// A help topic could not be rendered.
    Help(String),
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
            Self::Report(source) => source.fmt(formatter),
            Self::Config(source) => source.fmt(formatter),
            Self::Bundle(source) => source.fmt(formatter),
            Self::Backtest(source) => source.fmt(formatter),
            Self::Lab(source) => source.fmt(formatter),
            Self::StrategyRun(source) => source.fmt(formatter),
            Self::Layers(source) => write!(formatter, "cannot build the research layers: {source}"),
            Self::ResearchExport(source) => {
                write!(formatter, "cannot render the research dataset: {source}")
            }
            Self::Write(source) => write!(formatter, "cannot write the research dataset: {source}"),
            Self::Store(source) => source.fmt(formatter),
            Self::Help(message) => formatter.write_str(message),
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
            Self::Report(source) => Some(source),
            Self::Config(source) => Some(source),
            Self::Bundle(source) => Some(source),
            Self::Backtest(source) => Some(source),
            Self::Lab(source) => Some(source),
            Self::StrategyRun(source) => Some(source),
            Self::Layers(source) => Some(source),
            Self::ResearchExport(source) => Some(source),
            Self::Write(source) => Some(source),
            Self::Store(source) => Some(source),
            Self::Help(_) => None,
            Self::Output(source) => Some(source),
        }
    }
}
