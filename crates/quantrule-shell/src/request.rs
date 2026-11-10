//! The parsed request types shared by the command parsers and the stage runs.
//!
//! A request describes what a command asks a stage to do. The structs carry
//! paths, options, and already-validated identifiers; they contain no parsing,
//! rendering, or engine behavior. The module lives at the crate root so a stage
//! module can accept its input without importing the command adapter.

use std::path::PathBuf;

use quantrule_backtest::intent::StrategyPosition;
use quantrule_data::Timeframe;

use crate::date::DateRange;
use crate::pipeline::WindowMode;
use crate::source::Symbol;

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
#[derive(Debug, Clone, PartialEq)]
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

/// A parsed `execution run` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExecutionRunArgs {
    /// The strategy decisions bundle.
    pub(crate) decisions: PathBuf,
    /// The run configuration.
    pub(crate) config: PathBuf,
    /// The output root, or the decisions bundle's parent.
    pub(crate) out: Option<PathBuf>,
    /// Whether to print a JSON envelope of the artifact paths.
    pub(crate) json: bool,
}

/// A parsed `portfolio replay` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PortfolioReplayArgs {
    /// The execution fills bundle.
    pub(crate) fills: PathBuf,
    /// The run configuration.
    pub(crate) config: PathBuf,
    /// The output root, or the fills bundle's parent.
    pub(crate) out: Option<PathBuf>,
    /// Whether to print a JSON envelope of the artifact paths.
    pub(crate) json: bool,
}

/// A parsed `backtest run` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BacktestRunArgs {
    /// The run configuration.
    pub(crate) config: PathBuf,
    /// The strategy configuration.
    pub(crate) strategy: PathBuf,
    /// The output root, or the run configuration's root.
    pub(crate) out: Option<PathBuf>,
    /// Whether to print a JSON envelope of the artifact paths.
    pub(crate) json: bool,
}

/// A parsed `qualify` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct QualifyArgs {
    /// The backtest evidence bundle.
    pub(crate) evidence: PathBuf,
    /// The human-owned qualification criteria.
    pub(crate) criteria: PathBuf,
    /// The output root, or the evidence bundle's parent.
    pub(crate) out: Option<PathBuf>,
    /// Whether to print a JSON envelope.
    pub(crate) json: bool,
}

/// A parsed `analytics evaluate` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AnalyticsEvaluateArgs {
    /// The backtest evidence bundle.
    pub(crate) evidence: PathBuf,
    /// The output root, or the evidence bundle's parent.
    pub(crate) out: Option<PathBuf>,
    /// Whether to print a JSON envelope.
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
