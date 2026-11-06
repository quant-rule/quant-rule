//! The command vocabulary shared by every command family.
//!
//! The types in this module describe a parsed command and its failures. They
//! contain no parsing or rendering behavior, so a command family can depend on
//! the vocabulary without depending on another family.

use std::fmt;
use std::io;
use std::path::PathBuf;

use quantrule_backtest::intent::StrategyPosition;
use quantrule_data::Timeframe;

use crate::config::ConfigError;
use crate::csv::SourceError;
use crate::fetch::FetchRunError;
use crate::render_report::BundleError;
use crate::source::{DateRange, Symbol};
use crate::strategy::StrategyRunError;

/// A parsed shell command.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Command {
    /// Read a CSV source strictly and print the retained window.
    Candles(PathBuf),
    /// Read a CSV source tolerantly and print the window plus a quality report.
    CandlesTolerant(PathBuf),
    /// Build M5, M15, and H1 from an M1 CSV, then print the series and report.
    Timeframes(PathBuf),
    /// Print the event records.
    Events(ModelArgs),
    /// Write the flat model report bundle.
    Report(ReportArgs),
    /// Validate the policy and every engine config against it.
    PolicyCheck(PolicyCheckArgs),
    /// Explain one strategy decision from a model bundle.
    StrategyExplain(StrategyExplainArgs),
    /// Write the strategy decisions bundle for one model bundle.
    StrategyRun(StrategyRunArgs),
    /// Fetch a Dukascopy range and write a CSV.
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
    /// One command, named by one or two words.
    Command(String),
}

pub(crate) use crate::pipeline::WindowMode;

/// The path and input window shared by the event command.
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

/// A parsed `report model` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReportArgs {
    /// The run configuration.
    pub(crate) config: PathBuf,
    /// An optional measurement spec.
    pub(crate) spec: Option<PathBuf>,
    /// The output root, or the run configuration's root.
    pub(crate) out: Option<PathBuf>,
    /// Whether to print the bundle path as JSON.
    pub(crate) json: bool,
}

/// A parsed `policy check` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PolicyCheckArgs {
    /// The run configuration.
    pub(crate) config: PathBuf,
}

/// A parsed `strategy explain` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StrategyExplainArgs {
    /// The completed model bundle.
    pub(crate) model: PathBuf,
    /// The strategy configuration.
    pub(crate) strategy: PathBuf,
    /// The zero-based execution bar.
    pub(crate) bar: usize,
    /// The position the runner supplies to the strategy.
    pub(crate) position: StrategyPosition,
}

/// A parsed `strategy run` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StrategyRunArgs {
    /// The completed model bundle.
    pub(crate) model: PathBuf,
    /// The strategy configuration.
    pub(crate) strategy: PathBuf,
    /// The output root, or the model bundle's parent.
    pub(crate) out: Option<PathBuf>,
    /// Whether to print a JSON envelope of the artifact paths.
    pub(crate) json: bool,
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
}

/// A command line could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ParseError {
    /// The command is not known.
    Unknown(String),
    /// A usage line is invalid.
    Usage(String),
    /// A path-based command received extra arguments.
    UnexpectedArguments(&'static str),
    /// A path-based command has no path.
    MissingPath(&'static str),
    /// A model input window is invalid.
    Window(String),
    /// A report command is invalid.
    Report(String),
    /// A policy command is invalid.
    Policy(String),
    /// A strategy command is invalid.
    Strategy(String),
    /// A fetch command is invalid.
    Fetch(String),
    /// A help topic is invalid.
    Help(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown(name) => {
                write!(formatter, "unknown command '{name}'; type help")
            }
            Self::Usage(message) => formatter.write_str(message),
            Self::UnexpectedArguments(name) => write!(formatter, "{name} takes no arguments"),
            Self::MissingPath(name) => write!(formatter, "usage: {name} <csv-path>"),
            Self::Window(message)
            | Self::Report(message)
            | Self::Policy(message)
            | Self::Strategy(message)
            | Self::Fetch(message)
            | Self::Help(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for ParseError {}

/// A report-specific failure.
#[derive(Debug)]
pub(crate) enum ReportError {
    /// The requested period selected no candle.
    EmptyPeriod {
        /// The requested period as written.
        period: String,
    },
    /// The spec file could not be read.
    SpecRead {
        /// The spec path.
        path: PathBuf,
        /// The underlying error.
        source: io::Error,
    },
    /// The spec is invalid.
    Spec(String),
    /// A spec timeframe is not in the data ladder.
    SpecTimeframe {
        /// The requested timeframe.
        timeframe_secs: u64,
    },
}

impl fmt::Display for ReportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPeriod { period } => {
                write!(
                    formatter,
                    "no candle falls inside the requested period {period}"
                )
            }
            Self::SpecRead { path, source } => {
                write!(formatter, "cannot read {}: {source}", path.display())
            }
            Self::Spec(message) => formatter.write_str(message),
            Self::SpecTimeframe { timeframe_secs } => write!(
                formatter,
                "the spec timeframe {timeframe_secs}s is not in the run's data ladder"
            ),
        }
    }
}

impl std::error::Error for ReportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::SpecRead { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// A command failed while executing.
#[derive(Debug)]
pub(crate) enum RunError {
    /// A configuration file is invalid.
    Config(ConfigError),
    /// The source or data engine rejected the candles.
    Source(SourceError),
    /// The report could not be built.
    Report(ReportError),
    /// The strategy command failed.
    Strategy(StrategyRunError),
    /// The bundle could not be written.
    Bundle(BundleError),
    /// The fetch failed.
    Fetch(FetchRunError),
    /// A help topic is unknown.
    Help(String),
    /// The writer failed.
    Output(io::Error),
}

impl fmt::Display for RunError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(error) => error.fmt(formatter),
            Self::Source(error) => error.fmt(formatter),
            Self::Report(error) => error.fmt(formatter),
            Self::Strategy(error) => error.fmt(formatter),
            Self::Bundle(error) => error.fmt(formatter),
            Self::Fetch(error) => error.fmt(formatter),
            Self::Help(message) => formatter.write_str(message),
            Self::Output(error) => write!(formatter, "cannot write output: {error}"),
        }
    }
}

impl std::error::Error for RunError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Config(error) => Some(error),
            Self::Source(error) => Some(error),
            Self::Report(error) => Some(error),
            Self::Strategy(error) => Some(error),
            Self::Bundle(error) => Some(error),
            Self::Fetch(error) => Some(error),
            Self::Output(error) => Some(error),
            Self::Help(_) => None,
        }
    }
}
