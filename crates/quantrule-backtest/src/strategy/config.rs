//! The unified ladder strategy configuration and its validation.
//!
//! The configuration is the human-owned rule: the active ladder, the per-axis
//! context weights, the entry thresholds, the exit reasons, and the risk
//! intent. It is a value object, not a global setting. The strategy reads it on
//! every bar; nothing in it changes during a run.

use std::fmt;

use crate::context::{LadderConfig, LadderWeights};
use crate::intent::RiskPolicy;

/// The control entry threshold for Direction.
pub const DEFAULT_DIRECTION_MIN: f64 = 0.1;

/// The control entry threshold for Momentum force.
pub const DEFAULT_MOMENTUM_MIN: f64 = 0.1;

/// The control entry threshold for Structure score.
pub const DEFAULT_STRUCTURE_MIN: f64 = 0.0;

/// The control context Direction share threshold.
pub const DEFAULT_DIRECTION_SHARE_MIN: f64 = 0.5;

/// The control context Momentum bearish-share limit.
pub const DEFAULT_MOMENTUM_BEARISH_SHARE_MAX: f64 = 0.5;

/// The control context Structure conflict limit.
pub const DEFAULT_CONFLICT_COUNT_MAX: u32 = 0;

/// The control context Progression late/terminal limit.
pub const DEFAULT_LATE_OR_TERMINAL_COUNT_MAX: u32 = 0;

/// The execution-timeframe entry conditions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExecutionEntryConfig {
    direction_min: f64,
    momentum_min: f64,
    structure_min: f64,
    structure_shift_from_max: i8,
    structure_shift_to_min: i8,
}

impl ExecutionEntryConfig {
    /// Builds the execution entry conditions.
    ///
    /// `direction_min` and `momentum_min` are in `[0, 1]`; `structure_min` is
    /// in `[-1, 1]`. The structure shift is a pair of native Structure state
    /// codes in `[-2, 2]` with `from_max < to_min`.
    ///
    /// # Errors
    ///
    /// Returns [`StrategyConfigError`] when a value is outside its range or
    /// the shift does not move to a strictly higher code.
    pub fn new(
        direction_min: f64,
        momentum_min: f64,
        structure_min: f64,
        structure_shift_from_max: i8,
        structure_shift_to_min: i8,
    ) -> Result<Self, StrategyConfigError> {
        if !direction_min.is_finite() || !(0.0..=1.0).contains(&direction_min) {
            return Err(StrategyConfigError::DirectionMin);
        }
        if !momentum_min.is_finite() || !(0.0..=1.0).contains(&momentum_min) {
            return Err(StrategyConfigError::MomentumMin);
        }
        if !structure_min.is_finite() || !(-1.0..=1.0).contains(&structure_min) {
            return Err(StrategyConfigError::StructureMin);
        }
        if !(-2..=2).contains(&structure_shift_from_max)
            || !(-2..=2).contains(&structure_shift_to_min)
        {
            return Err(StrategyConfigError::StructureShiftCode);
        }
        if structure_shift_from_max >= structure_shift_to_min {
            return Err(StrategyConfigError::StructureShiftOrder);
        }
        Ok(Self {
            direction_min,
            momentum_min,
            structure_min,
            structure_shift_from_max,
            structure_shift_to_min,
        })
    }

    /// The Direction threshold.
    #[must_use]
    pub const fn direction_min(&self) -> f64 {
        self.direction_min
    }

    /// The Momentum force threshold.
    #[must_use]
    pub const fn momentum_min(&self) -> f64 {
        self.momentum_min
    }

    /// The Structure score threshold.
    #[must_use]
    pub const fn structure_min(&self) -> f64 {
        self.structure_min
    }

    /// The highest previous Structure state code accepted by the shift.
    #[must_use]
    pub const fn structure_shift_from_max(&self) -> i8 {
        self.structure_shift_from_max
    }

    /// The lowest current Structure state code accepted by the shift.
    #[must_use]
    pub const fn structure_shift_to_min(&self) -> i8 {
        self.structure_shift_to_min
    }
}

impl Default for ExecutionEntryConfig {
    fn default() -> Self {
        Self {
            direction_min: DEFAULT_DIRECTION_MIN,
            momentum_min: DEFAULT_MOMENTUM_MIN,
            structure_min: DEFAULT_STRUCTURE_MIN,
            structure_shift_from_max: 0,
            structure_shift_to_min: 1,
        }
    }
}

/// The higher-timeframe context entry conditions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContextEntryConfig {
    direction_share_min: f64,
    momentum_bearish_share_max: f64,
    structure_conflict_count_max: u32,
    progression_late_or_terminal_count_max: u32,
}

impl ContextEntryConfig {
    /// Builds the context entry conditions.
    ///
    /// Both share values are in `[0, 1]`. The counts are non-negative.
    ///
    /// # Errors
    ///
    /// Returns [`StrategyConfigError`] when a share is outside `[0, 1]`.
    pub fn new(
        direction_share_min: f64,
        momentum_bearish_share_max: f64,
        structure_conflict_count_max: u32,
        progression_late_or_terminal_count_max: u32,
    ) -> Result<Self, StrategyConfigError> {
        if !direction_share_min.is_finite() || !(0.0..=1.0).contains(&direction_share_min) {
            return Err(StrategyConfigError::DirectionShareMin);
        }
        if !momentum_bearish_share_max.is_finite()
            || !(0.0..=1.0).contains(&momentum_bearish_share_max)
        {
            return Err(StrategyConfigError::MomentumBearishShareMax);
        }
        Ok(Self {
            direction_share_min,
            momentum_bearish_share_max,
            structure_conflict_count_max,
            progression_late_or_terminal_count_max,
        })
    }

    /// The minimum weighted Direction share for the entry side.
    #[must_use]
    pub const fn direction_share_min(&self) -> f64 {
        self.direction_share_min
    }

    /// The maximum weighted opposing Momentum share.
    #[must_use]
    pub const fn momentum_bearish_share_max(&self) -> f64 {
        self.momentum_bearish_share_max
    }

    /// The maximum Structure conflict count.
    #[must_use]
    pub const fn structure_conflict_count_max(&self) -> u32 {
        self.structure_conflict_count_max
    }

    /// The maximum late or terminal Progression leg count.
    #[must_use]
    pub const fn progression_late_or_terminal_count_max(&self) -> u32 {
        self.progression_late_or_terminal_count_max
    }
}

impl Default for ContextEntryConfig {
    fn default() -> Self {
        Self {
            direction_share_min: DEFAULT_DIRECTION_SHARE_MIN,
            momentum_bearish_share_max: DEFAULT_MOMENTUM_BEARISH_SHARE_MAX,
            structure_conflict_count_max: DEFAULT_CONFLICT_COUNT_MAX,
            progression_late_or_terminal_count_max: DEFAULT_LATE_OR_TERMINAL_COUNT_MAX,
        }
    }
}

/// The complete entry configuration.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct EntryConfig {
    execution: ExecutionEntryConfig,
    context: ContextEntryConfig,
}

impl EntryConfig {
    /// Builds the entry configuration.
    #[must_use]
    pub const fn new(execution: ExecutionEntryConfig, context: ContextEntryConfig) -> Self {
        Self { execution, context }
    }

    /// The execution-timeframe conditions.
    #[must_use]
    pub const fn execution(&self) -> ExecutionEntryConfig {
        self.execution
    }

    /// The higher-timeframe context conditions.
    #[must_use]
    pub const fn context(&self) -> ContextEntryConfig {
        self.context
    }
}

/// The configured exit reasons.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExitConfig {
    context_lost: bool,
    structure_lost: bool,
}

impl ExitConfig {
    /// Builds the exit configuration.
    ///
    /// # Errors
    ///
    /// Returns [`StrategyConfigError::EmptyExitReasons`] when both reasons are
    /// disabled, because the rule would then have no strategy exit.
    pub fn new(context_lost: bool, structure_lost: bool) -> Result<Self, StrategyConfigError> {
        if !context_lost && !structure_lost {
            return Err(StrategyConfigError::EmptyExitReasons);
        }
        Ok(Self {
            context_lost,
            structure_lost,
        })
    }

    /// Builds the control exit configuration: both reasons enabled.
    #[must_use]
    pub const fn both() -> Self {
        Self {
            context_lost: true,
            structure_lost: true,
        }
    }

    /// Whether the rule exits when the entry context is lost.
    #[must_use]
    pub const fn on_context_lost(&self) -> bool {
        self.context_lost
    }

    /// Whether the rule exits when execution structure flips against the
    /// position.
    #[must_use]
    pub const fn on_structure_lost(&self) -> bool {
        self.structure_lost
    }
}

impl Default for ExitConfig {
    fn default() -> Self {
        Self::both()
    }
}

/// The complete unified ladder strategy configuration.
///
/// # Examples
///
/// ```
/// use quantrule_backtest::strategy::StrategyConfig;
///
/// let config = StrategyConfig::default();
/// assert_eq!(config.timeframe_secs(), 300);
/// assert_eq!(config.ladder().context_timeframes(), &[900]);
/// assert_eq!(config.risk().stop_atr_multiple(), 2.0);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct StrategyConfig {
    ladder: LadderConfig,
    weights: LadderWeights,
    entry: EntryConfig,
    exit: ExitConfig,
    risk: RiskPolicy,
    require_contiguous_execution: bool,
}

impl StrategyConfig {
    /// Builds a unified ladder strategy configuration.
    ///
    /// # Errors
    ///
    /// Returns [`StrategyConfigError::WeightsLength`] when the weight set does
    /// not match the context timeframes. The component values are validated
    /// when they are built.
    pub fn new(
        ladder: LadderConfig,
        weights: LadderWeights,
        entry: EntryConfig,
        exit: ExitConfig,
        risk: RiskPolicy,
        require_contiguous_execution: bool,
    ) -> Result<Self, StrategyConfigError> {
        if weights.len() != ladder.context_timeframes().len() {
            return Err(StrategyConfigError::WeightsLength {
                expected: ladder.context_timeframes().len(),
                actual: weights.len(),
            });
        }
        Ok(Self {
            ladder,
            weights,
            entry,
            exit,
            risk,
            require_contiguous_execution,
        })
    }

    /// The active ladder. `ladder[0]` is the execution timeframe.
    #[must_use]
    pub const fn ladder(&self) -> &LadderConfig {
        &self.ladder
    }

    /// The execution timeframe in seconds.
    #[must_use]
    pub const fn timeframe_secs(&self) -> u64 {
        self.ladder.execution_timeframe_secs()
    }

    /// The per-axis, per-context-timeframe weights.
    #[must_use]
    pub const fn weights(&self) -> &LadderWeights {
        &self.weights
    }

    /// The entry conditions.
    #[must_use]
    pub const fn entry(&self) -> EntryConfig {
        self.entry
    }

    /// The exit reasons.
    #[must_use]
    pub const fn exit(&self) -> ExitConfig {
        self.exit
    }

    /// The risk intent attached to each entry.
    #[must_use]
    pub const fn risk(&self) -> RiskPolicy {
        self.risk
    }

    /// Whether a non-contiguous execution bar blocks a new entry.
    #[must_use]
    pub const fn require_contiguous_execution(&self) -> bool {
        self.require_contiguous_execution
    }
}

impl Default for StrategyConfig {
    fn default() -> Self {
        Self {
            ladder: LadderConfig::default(),
            weights: LadderWeights::equal(1),
            entry: EntryConfig::default(),
            exit: ExitConfig::default(),
            risk: RiskPolicy::default(),
            require_contiguous_execution: true,
        }
    }
}

/// A strategy configuration was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrategyConfigError {
    /// The weight count does not match the context timeframes.
    WeightsLength {
        /// The number of context timeframes.
        expected: usize,
        /// The number of weights.
        actual: usize,
    },
    /// The Direction minimum is outside `[0, 1]`.
    DirectionMin,
    /// The Momentum minimum is outside `[0, 1]`.
    MomentumMin,
    /// The Structure minimum is outside `[-1, 1]`.
    StructureMin,
    /// A Structure shift code is outside `[-2, 2]`.
    StructureShiftCode,
    /// The Structure shift does not move to a strictly higher code.
    StructureShiftOrder,
    /// The Direction context share minimum is outside `[0, 1]`.
    DirectionShareMin,
    /// The Momentum context bearish-share maximum is outside `[0, 1]`.
    MomentumBearishShareMax,
    /// Both configured exit reasons are disabled.
    EmptyExitReasons,
}

impl fmt::Display for StrategyConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WeightsLength { expected, actual } => write!(
                formatter,
                "the ladder has {expected} context timeframes but {actual} weights"
            ),
            Self::DirectionMin => {
                write!(formatter, "the Direction minimum must be in [0, 1]")
            }
            Self::MomentumMin => {
                write!(formatter, "the Momentum minimum must be in [0, 1]")
            }
            Self::StructureMin => {
                write!(formatter, "the Structure minimum must be in [-1, 1]")
            }
            Self::StructureShiftCode => {
                write!(formatter, "the Structure shift codes must be in [-2, 2]")
            }
            Self::StructureShiftOrder => write!(
                formatter,
                "the Structure shift must move to a strictly higher code"
            ),
            Self::DirectionShareMin => {
                write!(formatter, "the Direction share minimum must be in [0, 1]")
            }
            Self::MomentumBearishShareMax => write!(
                formatter,
                "the Momentum bearish-share maximum must be in [0, 1]"
            ),
            Self::EmptyExitReasons => {
                write!(formatter, "at least one exit reason must be configured")
            }
        }
    }
}

impl std::error::Error for StrategyConfigError {}
