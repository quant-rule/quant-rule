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
    /// The stable registry event code.
    pub event_code: u16,
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
    /// The execution-bar window is converted to the context timeframe's bars so
    /// one published window means the same span on every timeframe.
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
        let context_window =
            context_window_bars(window_bars, self.execution_timeframe_secs, timeframe_secs);
        if let Some(timeframe) = run.timeframe(timeframe_secs)
            && let Some(view) = view(timeframe, bar_index, context_window)
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

/// Converts an execution-bar window to a context timeframe's bar count.
///
/// `execution_bars` is the published window in execution bars. A context bar
/// covers `context_secs`, so the count is rounded up to cover the whole window.
/// A zero timeframe or window still produces at least one bar.
#[must_use]
pub fn context_window_bars(execution_bars: usize, execution_secs: u64, context_secs: u64) -> usize {
    let bars = (execution_bars as u64)
        .saturating_mul(execution_secs)
        .div_ceil(context_secs.max(1))
        .max(1);
    usize::try_from(bars).unwrap_or(usize::MAX)
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
            .all_events()
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
        event_code: event.event_code,
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
    use super::{EventStrategyInput, context_window_bars};
    use crate::intent::{RiskPolicy, StrategyPosition};
    use crate::strategy::{
        CompressionBracketConfig, CompressionBracketInput, CompressionBracketStrategy, Condition,
        Decision, EventCompositeConfig, EventCompositeStrategy,
    };
    use quantrule_data::Candle;
    use quantrule_model::input::{MarketStateConfig, TimeframeInput};
    use quantrule_model::run::ModelRun;

    fn run_with(len: usize) -> ModelRun {
        let candles = (0..len)
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

    fn run_with_changed_later_bar(len: usize, changed: usize) -> ModelRun {
        let candles = (0..len)
            .map(|index| {
                let close = if index == changed {
                    200.0
                } else {
                    100.0 + index as f64 * 0.5
                };
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

    fn run() -> ModelRun {
        run_with(80)
    }

    #[test]
    fn the_context_window_is_expressed_in_execution_bars() {
        assert_eq!(context_window_bars(20, 300, 3_600), 2);
        assert_eq!(context_window_bars(20, 300, 900), 7);
        assert_eq!(context_window_bars(20, 300, 60), 100);
        assert_eq!(context_window_bars(60, 60, 3_600), 1);
        assert_eq!(context_window_bars(0, 300, 900), 1);
    }

    #[test]
    fn later_bars_do_not_change_an_earlier_snapshot() {
        let short = run_with(60);
        let full = run_with(80);
        let changed = run_with_changed_later_bar(80, 60);
        let short_snapshot = EventStrategyInput::from_model_run(&short, 300, 40, 20);
        assert_eq!(
            short_snapshot,
            EventStrategyInput::from_model_run(&full, 300, 40, 20)
        );
        assert_eq!(
            short_snapshot,
            EventStrategyInput::from_model_run(&changed, 300, 40, 20)
        );
    }

    #[test]
    fn the_snapshot_boundary_keeps_both_engine_decisions_causal() {
        fn event_decision(snapshot: &EventStrategyInput) -> Decision {
            let config = EventCompositeConfig::new(
                1,
                0,
                &[Condition::Event {
                    timeframe_secs: 300,
                    event_type: 1,
                    subtype: 3,
                    source: 19,
                    direction: 1,
                }],
                &[],
            )
            .expect("the event-composite config is valid");
            EventCompositeStrategy::new(config, RiskPolicy::default(), true)
                .decide(snapshot, StrategyPosition::flat())
        }

        fn bracket_decision(snapshot: &EventStrategyInput) -> Decision {
            let view = snapshot
                .timeframe(300)
                .expect("the execution timeframe is present");
            let input = CompressionBracketInput {
                execution_timeframe_secs: 300,
                execution_bar_index: snapshot.execution_bar_index,
                execution_open_time: snapshot.execution_open_time,
                execution_contiguous: true,
                events: &view.events,
            };
            let config = CompressionBracketConfig::new(300, 3, 8, 5, 1, 20)
                .expect("the compression-range config is valid");
            CompressionBracketStrategy::new(config, RiskPolicy::default(), true)
                .decide(&input, StrategyPosition::flat())
        }

        let short =
            EventStrategyInput::from_model_run(&run_with(60), 300, 40, 20).with_contiguous(true);
        let full =
            EventStrategyInput::from_model_run(&run_with(80), 300, 40, 20).with_contiguous(true);
        let changed =
            EventStrategyInput::from_model_run(&run_with_changed_later_bar(80, 60), 300, 40, 20)
                .with_contiguous(true);
        let decisions = [
            (event_decision(&short), bracket_decision(&short)),
            (event_decision(&full), bracket_decision(&full)),
            (event_decision(&changed), bracket_decision(&changed)),
        ];
        assert!(matches!(decisions[0].0, Decision::Enter(_)));
        // The fixture publishes no compression-range event, so the bracket arm
        // is Hold; the equality below still proves its decision is invariant.
        assert!(matches!(decisions[0].1, Decision::Hold));
        assert_eq!(decisions[0], decisions[1]);
        assert_eq!(decisions[0], decisions[2]);
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

    fn context_run() -> ModelRun {
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
        ModelRun::compute(
            &[
                TimeframeInput::new(300, &candles_300),
                TimeframeInput::new(900, &candles_900),
            ],
            &MarketStateConfig::default(),
        )
    }

    #[test]
    fn an_explicit_context_view_is_sorted_by_timeframe() {
        let run = context_run();
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
    fn the_context_view_window_is_converted_from_execution_bars() {
        let run = context_run();
        // 20 execution bars at 5m is 6,000 seconds: 7 context bars at 15m,
        // rounded up. At context bar 25 the window therefore starts at bar 19.
        let input =
            EventStrategyInput::from_model_run(&run, 300, 25, 1).with_view(&run, 900, 25, 20);
        let view = input
            .timeframe(900)
            .expect("the context timeframe is present");
        let oldest = view
            .events
            .iter()
            .map(|event| event.available_bar_index)
            .min()
            .expect("the converted context window contains events");
        assert!(
            oldest >= 19,
            "the 20-bar 5m window must reach back only 7 15m bars, not 20; oldest event: {oldest}"
        );
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
            "pub atr",
            "pub rsi",
            "pub ema",
            "pub rvol",
            "pub cash",
            "pub equity",
            "pub pnl",
            "pub fill",
            "pub quantity",
            "pub order",
        ] {
            assert!(!source.contains(forbidden), "the input exposes {forbidden}");
        }
    }
}
