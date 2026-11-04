//! The event/state snapshot a v8 strategy may read.
//!
//! The snapshot carries one closed model bar per active ladder timeframe: the
//! bar identity, the v3 discrete states, and the events that became knowable
//! within the configured window. It does not carry a raw indicator value, an
//! axis value, a percentile, a readiness code, a fill, a position, or P&L.
//!
//! The module is pure: it only projects a completed model run and the selected
//! as-of context. It performs no I/O, reads no clock, and uses no randomness.

use quantrule_model::context::ContextBar;
use quantrule_model::events::EventRecord;
use quantrule_model::run::ModelRun;
use quantrule_model::timeframe::ModelStates;

use crate::context::LadderContext;

/// One event as the v8 strategy sees it.
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
    /// The execution timeframe first, then context timeframes, fastest first.
    pub timeframes: Vec<EventTimeframeView>,
}

impl EventStrategyInput {
    /// Projects a completed model run and the as-of context onto the v8 input.
    ///
    /// `window_bars` is at least one. An event is included when its
    /// `available_bar_index` is at or before the selected bar and no more than
    /// `window_bars` behind it.
    #[must_use]
    pub fn from_model_run(
        run: &ModelRun,
        execution_timeframe_secs: u64,
        execution_bar_index: usize,
        context: &LadderContext,
        window_bars: usize,
    ) -> Self {
        let mut timeframes = Vec::with_capacity(context.context.len() + 1);
        if let Some(timeframe) = run.timeframe(execution_timeframe_secs)
            && let Some(bar) = timeframe.bar(execution_bar_index)
        {
            timeframes.push(view(
                timeframe.timeframe_secs(),
                &bar,
                timeframe.events(),
                window_bars,
                timeframe.context().get(bar.bar_index),
            ));
        }
        for selected in &context.context {
            if let Some(timeframe) = run.timeframe(selected.timeframe_secs)
                && let Some(bar) = timeframe.bar(selected.bar_index)
            {
                timeframes.push(view(
                    timeframe.timeframe_secs(),
                    &bar,
                    timeframe.events(),
                    window_bars,
                    timeframe.context().get(bar.bar_index),
                ));
            }
        }
        Self {
            execution_timeframe_secs,
            execution_bar_index,
            execution_open_time: context.execution_open_time,
            execution_contiguous: false,
            timeframes,
        }
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
    timeframe_secs: u64,
    bar: &quantrule_model::timeframe::ModelBar,
    events: &[EventRecord],
    window_bars: usize,
    context: Option<ContextBar>,
) -> EventTimeframeView {
    let window_start = bar.bar_index.saturating_sub(window_bars.saturating_sub(1));
    EventTimeframeView {
        timeframe_secs,
        bar_index: bar.bar_index,
        open_time: bar.candle.timestamp(),
        states: bar.states,
        progression_stage: bar.progression.stage_code,
        progression_leg_direction: bar.progression.leg_direction,
        events: events
            .iter()
            .filter(|event| {
                event.available_bar_index <= bar.bar_index
                    && event.available_bar_index >= window_start
            })
            .map(event_view)
            .collect(),
        context,
    }
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
    use crate::context::LadderContext;
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
        let context = LadderContext {
            execution_timeframe_secs: 300,
            execution_bar_index: 20,
            execution_open_time: 20 * 300,
            execution_close_time: 21 * 300,
            context: Vec::new(),
            missing_timeframes: Vec::new(),
            direction: crate::context::AxisContext::unavailable(),
            momentum: crate::context::AxisContext::unavailable(),
            structure: crate::context::StructureContext::unavailable(),
            progression: crate::context::ProgressionContext::unavailable(),
            levels: crate::context::LevelContext::unavailable(),
            agreement: crate::context::AgreementContext::unavailable(),
        };
        let input = EventStrategyInput::from_model_run(&run, 300, 20, &context, 5);
        assert_eq!(input.execution_timeframe_secs, 300);
        assert_eq!(input.execution_bar_index, 20);
        let view = input.timeframe(300).expect("the execution timeframe");
        assert_eq!(view.bar_index, 20);
        assert_eq!(view.open_time, 20 * 300);
        assert!(view.states.ema_stack.is_none());
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
