//! The command vocabulary shared by every command family.
//!
//! The types in this module describe the command envelope and its failures. The
//! parsed argument structs live in `crate::request`; this module owns `Command`,
//! the help topic, and the failure types. It contains no parsing or rendering
//! behavior, so a command family can depend on the vocabulary without depending
//! on another family.

use std::fmt;
use std::io;
use std::path::PathBuf;

use crate::analytics::AnalyticsRunError;
use crate::backtest::BacktestRunError;
use crate::config::ConfigError;
use crate::csv::SourceError;
use crate::execution::ExecutionRunError;
use crate::fetch::FetchRunError;
use crate::model_bundle::ModelBundleError;
use crate::portfolio::PortfolioRunError;
use crate::qualification::QualificationRunError;
use crate::request::{
    AnalyticsEvaluateArgs, BacktestRunArgs, ExecutionRunArgs, FetchArgs, ModelArgs,
    PolicyCheckArgs, PortfolioReplayArgs, QualifyArgs, ReportArgs, StrategyExplainArgs,
    StrategyRunArgs,
};
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
    /// Turn a strategy decisions bundle into orders, fills, and rejections.
    ExecutionRun(ExecutionRunArgs),
    /// Replay an execution fills bundle into portfolio accounting.
    PortfolioReplay(PortfolioReplayArgs),
    /// Run the coupled model, strategy, execution, and portfolio stages.
    BacktestRun(BacktestRunArgs),
    /// Evaluate an evidence bundle against human-owned qualification criteria.
    Qualify(QualifyArgs),
    /// Publish descriptive analytics metrics and evaluation tables.
    AnalyticsEvaluate(AnalyticsEvaluateArgs),
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
    /// An execution command is invalid.
    Execution(String),
    /// A portfolio command is invalid.
    Portfolio(String),
    /// A backtest command is invalid.
    Backtest(String),
    /// A qualification command is invalid.
    Qualification(String),
    /// An analytics command is invalid.
    Analytics(String),
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
            | Self::Execution(message)
            | Self::Portfolio(message)
            | Self::Backtest(message)
            | Self::Qualification(message)
            | Self::Analytics(message)
            | Self::Fetch(message)
            | Self::Help(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for ParseError {}

/// A report spec failure.
#[derive(Debug)]
pub(crate) enum ReportError {
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
    /// The report spec could not be read or parsed.
    Report(ReportError),
    /// The model bundle could not be built.
    Model(ModelBundleError),
    /// The strategy command failed.
    Strategy(StrategyRunError),
    /// The execution command failed.
    Execution(ExecutionRunError),
    /// The portfolio command failed.
    Portfolio(PortfolioRunError),
    /// The backtest command failed.
    Backtest(BacktestRunError),
    /// The qualification command failed.
    Qualification(QualificationRunError),
    /// The analytics command failed.
    Analytics(AnalyticsRunError),
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
            Self::Model(error) => error.fmt(formatter),
            Self::Strategy(error) => error.fmt(formatter),
            Self::Execution(error) => error.fmt(formatter),
            Self::Portfolio(error) => error.fmt(formatter),
            Self::Backtest(error) => error.fmt(formatter),
            Self::Qualification(error) => error.fmt(formatter),
            Self::Analytics(error) => error.fmt(formatter),
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
            Self::Model(error) => Some(error),
            Self::Strategy(error) => Some(error),
            Self::Execution(error) => Some(error),
            Self::Portfolio(error) => Some(error),
            Self::Backtest(error) => Some(error),
            Self::Qualification(error) => Some(error),
            Self::Analytics(error) => Some(error),
            Self::Fetch(error) => Some(error),
            Self::Output(error) => Some(error),
            Self::Help(_) => None,
        }
    }
}
