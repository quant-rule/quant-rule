//! The per-bar strategy input and the agreement classification.
//!
//! The input is a snapshot of the three published readings a strategy is
//! allowed to read, together with the bar identity needed to report a decision.
//! It carries no candle price, so a strategy cannot accidentally treat a price
//! as a fill.

use quantrule_model::run::ModelBar;

use super::config::StrategyConfig;
use super::decision::Side;

/// One closed model bar as the strategy sees it.
///
/// The three readings are the primary values of the Direction, Momentum, and
/// Structure axes. `None` means the axis did not publish a value; it is not a
/// neutral reading.
///
/// # Examples
///
/// ```
/// use quantrule_backtest::strategy::StrategyInput;
///
/// let bar = StrategyInput::new(900, 12, 1_700_000_000, Some(0.4), Some(0.2), Some(1.0));
/// assert_eq!(bar.bar_index, 12);
/// assert_eq!(bar.direction, Some(0.4));
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StrategyInput {
    /// The source timeframe in seconds.
    pub timeframe_secs: u64,
    /// The zero-based bar position in the timeframe series.
    pub bar_index: usize,
    /// The bar's open time as UTC Unix seconds.
    pub open_time: i64,
    /// The Direction axis value, or `None` when it is unavailable.
    pub direction: Option<f64>,
    /// The Momentum force, or `None` when it is unavailable.
    pub momentum_force: Option<f64>,
    /// The Structure score, or `None` when it is unavailable.
    pub structure_score: Option<f64>,
}

impl StrategyInput {
    /// Builds an input snapshot from explicit values.
    #[must_use]
    pub const fn new(
        timeframe_secs: u64,
        bar_index: usize,
        open_time: i64,
        direction: Option<f64>,
        momentum_force: Option<f64>,
        structure_score: Option<f64>,
    ) -> Self {
        Self {
            timeframe_secs,
            bar_index,
            open_time,
            direction,
            momentum_force,
            structure_score,
        }
    }

    /// Projects a published model bar onto the three readings a strategy may
    /// read.
    ///
    /// The candle, the other readings, the events, and the cross-timeframe
    /// aggregates are deliberately dropped. A strategy that needs one of them
    /// is outside this rule's boundary.
    #[must_use]
    pub fn from_model_bar(bar: &ModelBar) -> Self {
        Self {
            timeframe_secs: bar.timeframe_secs,
            bar_index: bar.bar_index,
            open_time: bar.candle.timestamp(),
            direction: bar.direction.direction,
            momentum_force: bar.momentum.force,
            structure_score: bar.structure.score,
        }
    }
}

/// The agreement of the Direction, Momentum, and Structure readings.
///
/// `Neutral` is a measured non-agreement: the readings are available but do
/// not all cross the same threshold. A missing or non-finite reading produces
/// no agreement at all, which is distinct from `Neutral`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Agreement {
    /// All three readings are strictly above the long threshold.
    Long,
    /// All three readings are strictly below the short threshold.
    Short,
    /// The readings are available but do not agree in either direction.
    Neutral,
}

impl Agreement {
    /// Classifies one bar against the configured thresholds.
    ///
    /// Returns `None` when any reading is absent or non-finite. Otherwise
    /// returns `Long` when all three readings are strictly above the long
    /// threshold, `Short` when all three are strictly below the short
    /// threshold, and `Neutral` in every other case. Equality at either
    /// threshold is therefore neutral.
    ///
    /// # Examples
    ///
    /// ```
    /// use quantrule_backtest::strategy::{Agreement, StrategyConfig, StrategyInput};
    ///
    /// let config = StrategyConfig::default();
    /// let bar = StrategyInput::new(900, 0, 0, Some(0.4), Some(0.2), Some(1.0));
    /// assert_eq!(Agreement::classify(&bar, &config), Some(Agreement::Long));
    /// ```
    #[must_use]
    pub fn classify(input: &StrategyInput, config: &StrategyConfig) -> Option<Self> {
        let direction = input.direction.filter(|value| value.is_finite())?;
        let momentum_force = input.momentum_force.filter(|value| value.is_finite())?;
        let structure_score = input.structure_score.filter(|value| value.is_finite())?;

        if direction > config.long_threshold()
            && momentum_force > config.long_threshold()
            && structure_score > config.long_threshold()
        {
            Some(Self::Long)
        } else if direction < config.short_threshold()
            && momentum_force < config.short_threshold()
            && structure_score < config.short_threshold()
        {
            Some(Self::Short)
        } else {
            Some(Self::Neutral)
        }
    }

    /// The entry side this agreement implies, or `None` for a neutral
    /// agreement.
    #[must_use]
    pub const fn side(self) -> Option<Side> {
        match self {
            Self::Long => Some(Side::Long),
            Self::Short => Some(Side::Short),
            Self::Neutral => None,
        }
    }
}
