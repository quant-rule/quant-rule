//! The v9 compression-breakout strategy.
//!
//! The rule reads the published compression-breakout event on the execution
//! timeframe and takes its direction at the next open. Execution resolves the
//! ATR stop and the reward-to-risk target; the strategy owns only the side, the
//! risk intent, and the time stop. It never computes an indicator or reads a
//! price.
//!
//! The module is pure: it reads the event/state snapshot and the position it is
//! given. It performs no I/O, reads no clock, and uses no randomness.

use crate::intent::{RiskPolicy, StrategyPosition};
use crate::side::Side;

use super::config::StrategyConfigError;
use super::decision::{Decision, Entry, Exit, ExitReason};
use super::event_input::EventView;
use super::input::StrategyInput;

/// The v9 compression-breakout configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompressionBreakoutConfig {
    event_timeframe_secs: u64,
    event_type: u8,
    event_subtype: u8,
    event_source: u16,
    window_bars: usize,
    time_stop_bars: u64,
}

impl CompressionBreakoutConfig {
    /// Builds a compression-breakout configuration.
    ///
    /// # Errors
    ///
    /// Returns [`StrategyConfigError`] when the timeframe is zero, the event
    /// code is outside its registered range, the window is zero, or the time
    /// stop is zero.
    pub fn new(
        event_timeframe_secs: u64,
        event_type: u8,
        event_subtype: u8,
        event_source: u16,
        window_bars: usize,
        time_stop_bars: u64,
    ) -> Result<Self, StrategyConfigError> {
        if event_timeframe_secs == 0 {
            return Err(StrategyConfigError::EventTimeframe);
        }
        if !(1..=3).contains(&event_type) {
            return Err(StrategyConfigError::EventType);
        }
        if !(1..=7).contains(&event_subtype) {
            return Err(StrategyConfigError::EventSubtype);
        }
        if !(1..=54).contains(&event_source) {
            return Err(StrategyConfigError::EventSource);
        }
        if window_bars == 0 {
            return Err(StrategyConfigError::EventWindowBars);
        }
        if time_stop_bars == 0 {
            return Err(StrategyConfigError::CompressionTimeStop);
        }
        Ok(Self {
            event_timeframe_secs,
            event_type,
            event_subtype,
            event_source,
            window_bars,
            time_stop_bars,
        })
    }

    /// The event timeframe in seconds.
    #[must_use]
    pub const fn event_timeframe_secs(self) -> u64 {
        self.event_timeframe_secs
    }

    /// The event type code.
    #[must_use]
    pub const fn event_type(self) -> u8 {
        self.event_type
    }

    /// The event subtype code.
    #[must_use]
    pub const fn event_subtype(self) -> u8 {
        self.event_subtype
    }

    /// The event source code.
    #[must_use]
    pub const fn event_source(self) -> u16 {
        self.event_source
    }

    /// The event window in bars.
    #[must_use]
    pub const fn window_bars(self) -> usize {
        self.window_bars
    }

    /// The maximum holding period in bars.
    #[must_use]
    pub const fn time_stop_bars(self) -> u64 {
        self.time_stop_bars
    }
}

/// The per-bar compression-breakout input.
///
/// The runner builds a per-bar index of the configured event once, so the
/// strategy never scans the full event batch on a 1m series.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompressionBreakoutInput<'a> {
    /// The execution timeframe in seconds.
    pub execution_timeframe_secs: u64,
    /// The zero-based execution bar.
    pub execution_bar_index: usize,
    /// The execution bar's UTC open time.
    pub execution_open_time: i64,
    /// Whether this execution bar opened one timeframe after the previous one.
    pub execution_contiguous: bool,
    /// The configured events available on this bar.
    pub events: &'a [EventView],
}

/// The v9 compression-breakout strategy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompressionBreakoutStrategy {
    config: CompressionBreakoutConfig,
    risk: RiskPolicy,
    require_contiguous: bool,
}

impl CompressionBreakoutStrategy {
    /// Builds the strategy.
    #[must_use]
    pub const fn new(
        config: CompressionBreakoutConfig,
        risk: RiskPolicy,
        require_contiguous: bool,
    ) -> Self {
        Self {
            config,
            risk,
            require_contiguous,
        }
    }

    /// The configuration.
    #[must_use]
    pub const fn config(self) -> CompressionBreakoutConfig {
        self.config
    }

    /// Decides for one closed execution bar.
    #[must_use]
    pub fn decide(
        &self,
        input: &CompressionBreakoutInput<'_>,
        position: StrategyPosition,
    ) -> Decision {
        if position.is_open() {
            if position.bars_held >= self.config.time_stop_bars {
                let Some(side) = position.side else {
                    return Decision::Hold;
                };
                return Decision::Exit(Exit {
                    side,
                    bar: identity(input),
                    reason: ExitReason::TimeStop,
                });
            }
            return Decision::Hold;
        }
        if self.require_contiguous && !input.execution_contiguous {
            return Decision::Hold;
        }
        let Some(side) = self.event_side(input) else {
            return Decision::Hold;
        };
        Decision::Enter(Entry {
            bar: identity(input),
            intent: self.risk.intent(side),
        })
    }

    /// The event's side when the current bar carries an unambiguous event.
    fn event_side(&self, input: &CompressionBreakoutInput<'_>) -> Option<Side> {
        let mut side = None;
        for event in input.events {
            if event.event_type != self.config.event_type
                || event.subtype != self.config.event_subtype
                || event.source != self.config.event_source
                || event.available_bar_index != input.execution_bar_index
                || event.direction == 0
            {
                continue;
            }
            let candidate = if event.direction > 0 {
                Side::Long
            } else {
                Side::Short
            };
            match side {
                None => side = Some(candidate),
                Some(existing) if existing != candidate => return None,
                Some(_) => {}
            }
        }
        side
    }
}

fn identity(input: &CompressionBreakoutInput<'_>) -> StrategyInput {
    StrategyInput::new(
        input.execution_timeframe_secs,
        input.execution_bar_index,
        input.execution_open_time,
        None,
        None,
        None,
    )
    .with_contiguous(input.execution_contiguous)
}

#[cfg(test)]
mod tests {
    use super::CompressionBreakoutConfig;
    use super::CompressionBreakoutStrategy;
    use crate::intent::{RiskPolicy, StrategyPosition};
    use crate::side::Side;
    use crate::strategy::compression::CompressionBreakoutInput;
    use crate::strategy::decision::{Decision, ExitReason};
    use crate::strategy::event_input::EventView;

    fn config() -> CompressionBreakoutConfig {
        CompressionBreakoutConfig::new(60, 3, 7, 5, 1, 20).expect("a valid config")
    }

    fn event(direction: i8, bar: usize) -> EventView {
        EventView {
            event_type: 3,
            subtype: 7,
            source: 5,
            direction,
            occurred_bar_index: bar,
            available_bar_index: bar,
            reference_code: None,
            primary_value: None,
            reference_value: None,
        }
    }

    fn input(direction: i8) -> CompressionBreakoutInput<'static> {
        CompressionBreakoutInput {
            execution_timeframe_secs: 60,
            execution_bar_index: 10,
            execution_open_time: 600,
            execution_contiguous: true,
            events: Box::leak(Box::new([event(direction, 10)])),
        }
    }

    fn flat() -> StrategyPosition {
        StrategyPosition::flat()
    }

    #[test]
    fn a_long_event_enters_long() {
        let strategy = CompressionBreakoutStrategy::new(config(), RiskPolicy::default(), true);
        let decision = strategy.decide(&input(1), flat());
        let Decision::Enter(entry) = decision else {
            panic!("an entry")
        };
        assert_eq!(entry.side(), Side::Long);
        assert_eq!(
            entry.intent.stop,
            crate::intent::StopSpec::AtrMultiple { multiple: 2.0 }
        );
    }

    #[test]
    fn a_short_event_enters_short() {
        let strategy = CompressionBreakoutStrategy::new(config(), RiskPolicy::default(), true);
        let decision = strategy.decide(&input(-1), flat());
        let Decision::Enter(entry) = decision else {
            panic!("an entry")
        };
        assert_eq!(entry.side(), Side::Short);
    }

    #[test]
    fn an_event_on_a_previous_bar_does_not_enter() {
        let strategy = CompressionBreakoutStrategy::new(config(), RiskPolicy::default(), true);
        let event = event(1, 9);
        let input = CompressionBreakoutInput {
            execution_timeframe_secs: 60,
            execution_bar_index: 10,
            execution_open_time: 600,
            execution_contiguous: true,
            events: &[event],
        };
        assert_eq!(strategy.decide(&input, flat()), Decision::Hold);
    }

    #[test]
    fn the_time_stop_exits_an_open_position() {
        let strategy = CompressionBreakoutStrategy::new(config(), RiskPolicy::default(), true);
        let position = StrategyPosition::opened(Side::Long, 0);
        let position = StrategyPosition {
            bars_held: 20,
            ..position
        };
        let decision = strategy.decide(&input(1), position);
        let Decision::Exit(exit) = decision else {
            panic!("an exit")
        };
        assert_eq!(exit.reason, ExitReason::TimeStop);
        assert_eq!(exit.side, Side::Long);
    }

    #[test]
    fn a_non_contiguous_bar_blocks_an_entry() {
        let strategy = CompressionBreakoutStrategy::new(config(), RiskPolicy::default(), true);
        let mut input = input(1);
        input.execution_contiguous = false;
        assert_eq!(strategy.decide(&input, flat()), Decision::Hold);
    }

    #[test]
    fn a_missing_execution_timeframe_holds() {
        let strategy = CompressionBreakoutStrategy::new(config(), RiskPolicy::default(), true);
        let input = CompressionBreakoutInput {
            execution_timeframe_secs: 60,
            execution_bar_index: 10,
            execution_open_time: 600,
            execution_contiguous: true,
            events: &[],
        };
        assert_eq!(strategy.decide(&input, flat()), Decision::Hold);
    }

    #[test]
    fn the_strategy_is_deterministic() {
        let strategy = CompressionBreakoutStrategy::new(config(), RiskPolicy::default(), true);
        let first = strategy.decide(&input(1), flat());
        let second = strategy.decide(&input(1), flat());
        assert_eq!(first, second);
    }
}
