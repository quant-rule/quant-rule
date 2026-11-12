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
use crate::execution::ExecutionRunError;
use crate::fetch::FetchRunError;
use crate::model_bundle::ModelBundleError;
use crate::portfolio::PortfolioRunError;
use crate::qualification::QualificationRunError;
use crate::request::{
    AnalyticsEvaluateArgs, BacktestRunArgs, ExecutionRunArgs, FetchArgs, ModelArgs,
    PortfolioReplayArgs, QualifyArgs, ReportArgs, RunCheckArgs, StrategyExplainArgs,
    StrategyRunArgs,
};
use crate::strategy::StrategyRunError;
use quantrule_app::config::ConfigError;
use quantrule_app::csv::SourceError;
use quantrule_app::error::AppError;
use quantrule_app::memory::MemoryError;

/// A parsed shell command.
#[derive(Debug, PartialEq)]
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
    /// Validate the run and every engine config against it.
    RunCheck(RunCheckArgs),
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
    /// Read or write the local research memory.
    Memory(MemoryArgs),
    /// Print the command reference, one command, or a reference topic.
    Help(HelpTopic),
    /// Exit the shell.
    Quit,
}

/// One typed metric value for `memory metric set`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum MetricArg {
    /// A finite real number.
    Real(f64),
    /// A signed integer.
    Integer(i64),
    /// Text.
    Text(String),
    /// An unavailable measurement.
    Null,
}

/// The parsed `memory` command family.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum MemoryArgs {
    /// Print the schema, version, and path.
    Status {
        /// The memory database.
        db: PathBuf,
    },
    /// Record a validated run and its computed model counts.
    Record {
        /// The memory database.
        db: PathBuf,
        /// The run configuration.
        config: PathBuf,
        /// The stable run key.
        run_key: String,
        /// The status code.
        status: String,
        /// The caller-supplied start second.
        started_at: i64,
        /// The caller-supplied finish second.
        finished_at: Option<i64>,
    },
    /// List workspaces.
    Workspaces {
        /// The memory database.
        db: PathBuf,
    },
    /// List runs in one workspace.
    Runs {
        /// The memory database.
        db: PathBuf,
        /// The workspace ID.
        workspace_id: i64,
    },
    /// List metrics for one run.
    Metrics {
        /// The memory database.
        db: PathBuf,
        /// The run ID.
        run_id: i64,
    },
    /// Insert or update one metric.
    MetricSet {
        /// The memory database.
        db: PathBuf,
        /// The run ID.
        run_id: i64,
        /// The metric name.
        name: String,
        /// The typed value.
        value: MetricArg,
    },
    /// Delete one metric.
    MetricDelete {
        /// The memory database.
        db: PathBuf,
        /// The run ID.
        run_id: i64,
        /// The metric name.
        name: String,
    },
    /// List claims for one run.
    Claims {
        /// The memory database.
        db: PathBuf,
        /// The run ID.
        run_id: i64,
    },
    /// Insert one claim.
    ClaimAdd {
        /// The memory database.
        db: PathBuf,
        /// The run ID.
        run_id: i64,
        /// The claim type.
        claim_type: String,
        /// The claim statement.
        statement: String,
        /// The claim status.
        status: String,
        /// The caller-supplied creation second.
        created_at: i64,
    },
    /// Delete one claim.
    ClaimDelete {
        /// The memory database.
        db: PathBuf,
        /// The claim ID.
        id: i64,
    },
    /// List notes for one workspace or one run.
    Notes {
        /// The memory database.
        db: PathBuf,
        /// The workspace ID.
        workspace_id: Option<i64>,
        /// The run ID.
        run_id: Option<i64>,
    },
    /// Insert one note.
    NoteAdd {
        /// The memory database.
        db: PathBuf,
        /// The workspace ID.
        workspace_id: i64,
        /// The optional run ID.
        run_id: Option<i64>,
        /// The note body.
        body: String,
        /// The caller-supplied creation second.
        created_at: i64,
        /// The caller-supplied update second.
        updated_at: i64,
    },
    /// Update one note.
    NoteUpdate {
        /// The memory database.
        db: PathBuf,
        /// The note ID.
        id: i64,
        /// The note body.
        body: String,
        /// The caller-supplied update second.
        updated_at: i64,
    },
    /// Delete one note.
    NoteDelete {
        /// The memory database.
        db: PathBuf,
        /// The note ID.
        id: i64,
    },
    /// Write the logical export.
    Export {
        /// The memory database.
        db: PathBuf,
        /// The export path.
        out: PathBuf,
    },
    /// Import a logical export into an empty memory.
    Import {
        /// The memory database.
        db: PathBuf,
        /// The export path.
        from: PathBuf,
    },
    /// Write a SQLite backup.
    Backup {
        /// The memory database.
        db: PathBuf,
        /// The backup path.
        out: PathBuf,
    },
    /// Restore a SQLite backup to a new path.
    Restore {
        /// The backup path.
        backup: PathBuf,
        /// The new database path.
        target: PathBuf,
    },
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
    /// The workflow selection guide.
    Workflows,
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
    /// A run-check command is invalid.
    RunCheck(String),
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
    /// A memory command is invalid.
    Memory(String),
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
            | Self::RunCheck(message)
            | Self::Strategy(message)
            | Self::Execution(message)
            | Self::Portfolio(message)
            | Self::Backtest(message)
            | Self::Qualification(message)
            | Self::Analytics(message)
            | Self::Fetch(message)
            | Self::Memory(message)
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
    /// The application service failed.
    App(AppError),
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
    /// The memory operation failed.
    Memory(MemoryError),
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
            Self::App(error) => error.fmt(formatter),
            Self::Report(error) => error.fmt(formatter),
            Self::Model(error) => error.fmt(formatter),
            Self::Strategy(error) => error.fmt(formatter),
            Self::Execution(error) => error.fmt(formatter),
            Self::Portfolio(error) => error.fmt(formatter),
            Self::Backtest(error) => error.fmt(formatter),
            Self::Qualification(error) => error.fmt(formatter),
            Self::Analytics(error) => error.fmt(formatter),
            Self::Fetch(error) => error.fmt(formatter),
            Self::Memory(error) => error.fmt(formatter),
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
            Self::App(error) => Some(error),
            Self::Report(error) => Some(error),
            Self::Model(error) => Some(error),
            Self::Strategy(error) => Some(error),
            Self::Execution(error) => Some(error),
            Self::Portfolio(error) => Some(error),
            Self::Backtest(error) => Some(error),
            Self::Qualification(error) => Some(error),
            Self::Analytics(error) => Some(error),
            Self::Fetch(error) => Some(error),
            Self::Memory(error) => Some(error),
            Self::Output(error) => Some(error),
            Self::Help(_) => None,
        }
    }
}
