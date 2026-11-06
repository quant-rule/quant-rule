//! The event/state snapshot a strategy may read.
//!
//! The snapshot carries one closed model bar per included timeframe: the bar
//! identity, the discrete states, the events that became knowable within the
//! configured window, and the published progression and context values. It does
//! not carry a raw indicator value, an axis score, a percentile, a readiness
//! code, a fill, a position, or P&L.
//!
//! The module is pure: it only projects a completed model run. It performs no
//! I/O, reads no clock, and uses no randomness.

use quantrule_model::context::ContextBar;
use quantrule_model::events::EventRecord;
use quantrule_model::run::ModelRun;
use quantrule_model::timeframe::ModelStates;

/// One event as the strategy sees it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EventView {
    /// The event type code: 1 Level, 2 Divergence, 3 Signal.
    pub event_type: u8,
    /// The event subtype code.
    pub subtype: u8,
    /// The stable source code.
    pub source: u16,
    /// -1 bearish, 0 neutral, +1 bullish.
    pub direction: i8,
    /// The bar the relationship describes.
    pub occurred_bar_index: usize,
    /// The first bar on which it became knowable.
    pub available_bar_index: usize,
    /// The identified EMA or confirmed swing level, when applicable.
    pub reference_code: Option<u32>,
    /// The main measured value at the occurrence.
    pub primary_value: Option<f64>,
    /// The compared level or value, when applicable.
    pub reference_value: Option<f64>,
}

/// One timeframe's event/state snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct EventTimeframeView {
    /// The timeframe in seconds.
    pub timeframe_secs: u64,
    /// The zero-based bar position.
    pub bar_index: usize,
    /// The bar's UTC open time.
    pub open_time: i64,
    /// The v3 discrete states at that bar.
    pub states: ModelStates,
    /// The progression stage code, or `None`.
    pub progression_stage: Option<i8>,
    /// The progression leg direction, or `None`.
    pub progression_leg_direction: Option<i8>,
    /// Events that became knowable within the configured window.
    pub events: Vec<EventView>,
    /// The latest context values, for presentation and diagnostics only.
    pub context: Option<ContextBar>,
}

/// The complete event/state snapshot for one execution bar.
#[derive(Debug, Clone, PartialEq)]
pub struct EventStrategyInput {
    /// The execution timeframe in seconds.
    pub execution_timeframe_secs: u64,
    /// The zero-based execution bar.
    pub execution_bar_index: usize,
    /// The execution bar's UTC open time.
    pub execution_open_time: i64,
    /// Whether this execution bar opened one timeframe after the previous one.
    pub execution_contiguous: bool,
    /// The execution timeframe first, then any explicitly added context
    /// timeframes, fastest first.
    pub timeframes: Vec<EventTimeframeView>,
}

impl EventStrategyInput {
    /// Projects the execution timeframe of a completed model run.
    ///
    /// `window_bars` is at least one. An event is included when its
    /// `available_bar_index` is at or before the selected bar and no more than
    /// `window_bars` behind it. Higher-timeframe views are added explicitly with
    /// [`Self::with_view`]; the model run does not infer a cross-timeframe
    /// alignment for the strategy.
    #[must_use]
    pub fn from_model_run(
        run: &ModelRun,
        execution_timeframe_secs: u64,
        execution_bar_index: usize,
        window_bars: usize,
    ) -> Self {
        let mut timeframes = Vec::new();
        let execution_open_time = run
            .timeframe(execution_timeframe_secs)
            .and_then(|timeframe| timeframe.candle(execution_bar_index))
            .map_or(0, |candle| candle.timestamp());
        if let Some(timeframe) = run.timeframe(execution_timeframe_secs)
            && let Some(view) = view(timeframe, execution_bar_index, window_bars)
        {
            timeframes.push(view);
        }
        Self {
            execution_timeframe_secs,
            execution_bar_index,
            execution_open_time,
            execution_contiguous: false,
            timeframes,
        }
    }

    /// Appends one explicitly selected timeframe view.
    ///
    /// Returns `self` unchanged when the timeframe or bar is absent. Views stay
    /// ordered by timeframe, so the result does not depend on insertion order.
    #[must_use]
    pub fn with_view(
        mut self,
        run: &ModelRun,
        timeframe_secs: u64,
        bar_index: usize,
        window_bars: usize,
    ) -> Self {
        if self
            .timeframes
            .iter()
            .any(|view| view.timeframe_secs == timeframe_secs)
        {
            return self;
        }
        if let Some(timeframe) = run.timeframe(timeframe_secs)
            && let Some(view) = view(timeframe, bar_index, window_bars)
        {
            self.timeframes.push(view);
            self.timeframes.sort_by_key(|view| view.timeframe_secs);
        }
        self
    }

    /// One timeframe's view, or `None` when it is absent.
    #[must_use]
    pub fn timeframe(&self, timeframe_secs: u64) -> Option<&EventTimeframeView> {
        self.timeframes
            .iter()
            .find(|view| view.timeframe_secs == timeframe_secs)
    }

    /// Marks whether the execution bar is contiguous.
    #[must_use]
    pub fn with_contiguous(mut self, contiguous: bool) -> Self {
        self.execution_contiguous = contiguous;
        self
    }
}

fn view(
    timeframe: &quantrule_model::timeframe::TimeframeModel,
    bar_index: usize,
    window_bars: usize,
) -> Option<EventTimeframeView> {
    let bar = timeframe.bar(bar_index)?;
    let window_start = bar_index.saturating_sub(window_bars.saturating_sub(1));
    Some(EventTimeframeView {
        timeframe_secs: timeframe.timeframe_secs(),
        bar_index,
        open_time: bar.candle.timestamp(),
        states: bar.states,
        progression_stage: bar.progression.stage_code,
        progression_leg_direction: bar.progression.leg_direction,
        events: timeframe
            .events()
            .iter()
            .filter(|event| {
                event.available_bar_index <= bar_index && event.available_bar_index >= window_start
            })
            .map(event_view)
            .collect(),
        context: timeframe.context().get(bar_index),
    })
}

fn event_view(event: &EventRecord) -> EventView {
    EventView {
        event_type: event.event_type.code(),
        subtype: event.event_subtype_code,
        source: event.source_code,
        direction: event.direction_code,
        occurred_bar_index: event.occurred_bar_index,
        available_bar_index: event.available_bar_index,
        reference_code: event.reference_code,
        primary_value: event.primary_value,
        reference_value: event.reference_value,
    }
}

#[cfg(test)]
mod tests {
    use super::EventStrategyInput;
    use quantrule_data::Candle;
    use quantrule_model::input::{MarketStateConfig, TimeframeInput};
    use quantrule_model::run::ModelRun;

    fn run() -> ModelRun {
        let candles = (0..80)
            .map(|index| {
                let close = 100.0 + index as f64 * 0.5;
                Candle::new(
                    index as u64,
                    index as i64 * 300,
                    close,
                    close + 1.0,
                    close - 1.0,
                    close,
                    Some(1.0),
                )
                .expect("the test candle is valid")
            })
            .collect::<Vec<_>>();
        ModelRun::compute(
            &[TimeframeInput::new(300, &candles)],
            &MarketStateConfig::default(),
        )
    }

    #[test]
    fn the_snapshot_carries_identity_states_and_events() {
        let run = run();
        let input = EventStrategyInput::from_model_run(&run, 300, 20, 5);
        assert_eq!(input.execution_timeframe_secs, 300);
        assert_eq!(input.execution_bar_index, 20);
        let view = input.timeframe(300).expect("the execution timeframe");
        assert_eq!(view.bar_index, 20);
        assert_eq!(view.open_time, 20 * 300);
        assert!(view.states.ema_stack.is_none());
    }

    #[test]
    fn an_explicit_context_view_is_sorted_by_timeframe() {
        let candles_300 = (0..80)
            .map(|index| {
                let close = 100.0 + index as f64 * 0.5;
                Candle::new(
                    index as u64,
                    index as i64 * 300,
                    close,
                    close + 1.0,
                    close - 1.0,
                    close,
                    Some(1.0),
                )
                .expect("the test candle is valid")
            })
            .collect::<Vec<_>>();
        let candles_900 = (0..30)
            .map(|index| {
                let close = 100.0 + index as f64;
                Candle::new(
                    index as u64,
                    index as i64 * 900,
                    close,
                    close + 1.0,
                    close - 1.0,
                    close,
                    Some(1.0),
                )
                .expect("the test candle is valid")
            })
            .collect::<Vec<_>>();
        let run = ModelRun::compute(
            &[
                TimeframeInput::new(300, &candles_300),
                TimeframeInput::new(900, &candles_900),
            ],
            &MarketStateConfig::default(),
        );
        let input = EventStrategyInput::from_model_run(&run, 300, 20, 5)
            .with_view(&run, 900, 6, 5)
            .with_view(&run, 300, 20, 5);
        let timeframes: Vec<u64> = input
            .timeframes
            .iter()
            .map(|view| view.timeframe_secs)
            .collect();
        assert_eq!(timeframes, vec![300, 900]);
    }

    #[test]
    fn the_snapshot_has_no_axis_or_statistical_field() {
        let source = include_str!("event_input.rs")
            .split("#[cfg(test)]")
            .next()
            .expect("the source has a test marker");
        for forbidden in [
            "pub axis",
            "pub percentile",
            "pub readiness",
            "pub momentum",
            "pub cross_timeframe",
        ] {
            assert!(!source.contains(forbidden), "the input exposes {forbidden}");
        }
    }
}
