//! The per-bar execution snapshot a strategy may read.
//!
//! The snapshot carries the execution timeframe's primary axis values and bar
//! identity, plus the previous Structure state needed by the configured shift.
//! The as-of [`crate::context::LadderContext`] is passed separately; it is built
//! by the strategy layer and is not part of the model bar.
//!
//! The snapshot carries no candle price, fill, cash, P&L, percentile,
//! readiness, event, or raw indicator value, so a strategy cannot accidentally
//! read one.

use quantrule_model::timeframe::ModelBar;

/// One closed execution bar as the strategy sees it.
///
/// `None` means the axis did not publish a value; it is not a neutral reading.
/// `execution_contiguous` says whether this bar opened exactly one execution
/// timeframe after the previous bar.
///
/// # Examples
///
/// ```
/// use quantrule_backtest::strategy::StrategyInput;
///
/// let bar = StrategyInput::new(300, 12, 1_700_000_000, Some(0.4), Some(0.2), Some(1.0));
/// assert_eq!(bar.bar_index, 12);
/// assert_eq!(bar.direction, Some(0.4));
/// assert!(bar.execution_contiguous);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StrategyInput {
    /// The execution timeframe in seconds.
    pub timeframe_secs: u64,
    /// The zero-based bar position in the execution timeframe series.
    pub bar_index: usize,
    /// The bar's open time as UTC Unix seconds.
    pub open_time: i64,
    /// The Direction axis value, or `None` when it is unavailable.
    pub direction: Option<f64>,
    /// The Momentum force, or `None` when it is unavailable.
    pub momentum_force: Option<f64>,
    /// The Structure score, or `None` when it is unavailable.
    pub structure_score: Option<f64>,
    /// The compact Structure state code in `[-2, 2]`, or `None`.
    pub structure_state_code: Option<i8>,
    /// The previous bar's compact Structure state code, or `None`.
    pub previous_structure_state_code: Option<i8>,
    /// The published Progression leg direction, or `None`.
    pub progression_leg_direction: Option<i8>,
    /// The published Progression stage code in `0..=5`, or `None`.
    pub progression_stage_code: Option<i8>,
    /// Whether this execution bar opened one timeframe after the previous one.
    pub execution_contiguous: bool,
}

impl StrategyInput {
    /// Builds a snapshot from the three primary values and bar identity.
    ///
    /// The Structure state, Progression fields, and contiguity default to
    /// unavailable/contiguous. Use [`Self::with_structure`],
    /// [`Self::with_progression`], and [`Self::with_contiguous`] to add them.
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
            structure_state_code: None,
            previous_structure_state_code: None,
            progression_leg_direction: None,
            progression_stage_code: None,
            execution_contiguous: true,
        }
    }

    /// Adds the current and previous Structure state codes.
    #[must_use]
    pub const fn with_structure(
        mut self,
        structure_state_code: Option<i8>,
        previous_structure_state_code: Option<i8>,
    ) -> Self {
        self.structure_state_code = structure_state_code;
        self.previous_structure_state_code = previous_structure_state_code;
        self
    }

    /// Adds the published Progression fields.
    #[must_use]
    pub const fn with_progression(
        mut self,
        progression_leg_direction: Option<i8>,
        progression_stage_code: Option<i8>,
    ) -> Self {
        self.progression_leg_direction = progression_leg_direction;
        self.progression_stage_code = progression_stage_code;
        self
    }

    /// Sets whether the execution bar is contiguous.
    #[must_use]
    pub const fn with_contiguous(mut self, execution_contiguous: bool) -> Self {
        self.execution_contiguous = execution_contiguous;
        self
    }

    /// Projects a published execution model bar onto the snapshot.
    ///
    /// The candle, percentiles, readiness, events, and cross-timeframe
    /// aggregates are deliberately dropped.
    #[must_use]
    pub fn from_model_bar(
        bar: &ModelBar,
        previous: Option<&ModelBar>,
        execution_contiguous: bool,
    ) -> Self {
        Self {
            timeframe_secs: bar.timeframe_secs,
            bar_index: bar.bar_index,
            open_time: bar.candle.timestamp(),
            direction: bar.direction.direction,
            momentum_force: bar.momentum.force,
            structure_score: bar.structure.score,
            structure_state_code: bar.structure.state_code,
            previous_structure_state_code: previous.and_then(|bar| bar.structure.state_code),
            progression_leg_direction: bar.progression.leg_direction,
            progression_stage_code: bar.progression.stage_code,
            execution_contiguous,
        }
    }
}

#[cfg(test)]
mod boundary_tests {
    /// The strategy input must stay a narrow projection of the model bar.
    ///
    /// This is a source-level guard against a future field silently exposing a
    /// statistical diagnostic, an event, context, execution state, or
    /// portfolio data to the strategy. The compiler enforces the same shape
    /// for every field that is actually present.
    #[test]
    fn the_strategy_input_exposes_only_primary_values_and_identity() {
        let source = include_str!("input.rs")
            .split("#[cfg(test)]")
            .next()
            .expect("the source has a test marker");
        for forbidden in [
            "pub percentile",
            "pub readiness",
            "pub event",
            "pub context",
            "pub execution_config",
            "pub portfolio",
            "pub atr",
            "pub rsi",
            "pub ema",
            "pub rvol",
            "pub cash",
            "pub equity",
            "pub pnl",
            "pub fill",
            "pub order",
            "pub quantity",
        ] {
            assert!(
                !source.contains(forbidden),
                "StrategyInput must not expose `{forbidden}`"
            );
        }
    }
}
