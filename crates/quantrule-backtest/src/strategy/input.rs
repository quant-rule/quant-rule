//! The per-bar execution snapshot a strategy may read.
//!
//! The snapshot carries the execution timeframe's identity and its published
//! progression values. The event engine reads its own [`super::event_input`]
//! snapshot instead. The snapshot carries no candle price, fill, cash, P&L,
//! percentile, readiness, event, axis score, or raw indicator value, so a
//! strategy cannot accidentally read one.

use quantrule_model::timeframe::ModelBar;

/// One closed execution bar as the strategy sees it.
///
/// `None` means the model did not publish a value; it is not a neutral reading.
/// `execution_contiguous` says whether this bar opened exactly one execution
/// timeframe after the previous bar.
///
/// # Examples
///
/// ```
/// use quantrule_backtest::strategy::StrategyInput;
///
/// let bar = StrategyInput::new(300, 12, 1_700_000_000)
///     .with_progression(Some(1), Some(2));
/// assert_eq!(bar.bar_index, 12);
/// assert_eq!(bar.progression_stage_code, Some(2));
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
    /// The published Progression leg direction, or `None`.
    pub progression_leg_direction: Option<i8>,
    /// The published Progression stage code in `0..=5`, or `None`.
    pub progression_stage_code: Option<i8>,
    /// Whether this execution bar opened one timeframe after the previous one.
    pub execution_contiguous: bool,
}

impl StrategyInput {
    /// Builds a snapshot from the bar identity.
    ///
    /// The Progression fields default to unavailable; contiguity defaults to
    /// true. Use [`Self::with_progression`] and [`Self::with_contiguous`] to add
    /// them.
    #[must_use]
    pub const fn new(timeframe_secs: u64, bar_index: usize, open_time: i64) -> Self {
        Self {
            timeframe_secs,
            bar_index,
            open_time,
            progression_leg_direction: None,
            progression_stage_code: None,
            execution_contiguous: true,
        }
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
    /// The candle, readings, states, events, levels, context, and any
    /// cross-timeframe view are deliberately dropped.
    #[must_use]
    pub fn from_model_bar(bar: &ModelBar, execution_contiguous: bool) -> Self {
        Self {
            timeframe_secs: bar.timeframe_secs,
            bar_index: bar.bar_index,
            open_time: bar.candle.timestamp(),
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
    fn the_strategy_input_exposes_only_identity_and_progression() {
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
            "pub diagnosis",
            "diagnosis_",
            "pub qualification",
            "pub verdict",
            "pub analytics",
            "pub human_decision",
            "qualification_",
            "verdict_",
            "analytics_",
        ] {
            assert!(
                !source.contains(forbidden),
                "StrategyInput must not expose `{forbidden}`"
            );
        }
    }
}
