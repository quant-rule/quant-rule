//! The v10 compression-bracket strategy.
//!
//! The rule reads the neutral compression-range event on the execution
//! timeframe and places a two-sided stop-entry bracket at the published range
//! boundaries. The first fill chooses the side; execution owns the fill, the
//! OCO cancellation, and the stop/target bracket. The strategy owns the time
//! stop and the entry window.
//!
//! The module is pure: it reads the event snapshot and the position it is
//! given. It performs no I/O, reads no clock, and uses no randomness.

use crate::intent::{BracketIntent, RiskPolicy, StopSpec, StrategyPosition, TargetSpec};

use super::config::StrategyConfigError;
use super::decision::{BracketEntry, Decision, Exit, ExitReason};
use super::event_input::EventView;
use super::input::StrategyInput;

/// The v10 compression-bracket configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompressionBracketConfig {
    event_timeframe_secs: u64,
    event_type: u8,
    event_subtype: u8,
    event_source: u16,
    entry_valid_bars: u32,
    time_stop_bars: u64,
}

impl CompressionBracketConfig {
    /// Builds a compression-bracket configuration.
    ///
    /// # Errors
    ///
    /// Returns [`StrategyConfigError`] when the timeframe is zero, the event
    /// code is outside its registered range, the entry window is zero, or the
    /// time stop is zero.
    pub fn new(
        event_timeframe_secs: u64,
        event_type: u8,
        event_subtype: u8,
        event_source: u16,
        entry_valid_bars: u32,
        time_stop_bars: u64,
    ) -> Result<Self, StrategyConfigError> {
        if event_timeframe_secs == 0 {
            return Err(StrategyConfigError::EventTimeframe);
        }
        if !(1..=3).contains(&event_type) {
            return Err(StrategyConfigError::EventType);
        }
        if !(1..=9).contains(&event_subtype) {
            return Err(StrategyConfigError::EventSubtype);
        }
        if !(1..=18).contains(&event_source) {
            return Err(StrategyConfigError::EventSource);
        }
        if entry_valid_bars != 1 {
            // The pre-registered rule places the bracket for the next bar
            // only. A longer window would leave a pending bracket while the
            // runner submits the next decision, so it is refused here rather
            // than failing mid-run.
            return Err(StrategyConfigError::CompressionEntryWindow);
        }
        if time_stop_bars == 0 {
            return Err(StrategyConfigError::CompressionTimeStop);
        }
        Ok(Self {
            event_timeframe_secs,
            event_type,
            event_subtype,
            event_source,
            entry_valid_bars,
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

    /// The entry bracket's validity in bars.
    #[must_use]
    pub const fn entry_valid_bars(self) -> u32 {
        self.entry_valid_bars
    }

    /// The maximum holding period in bars.
    #[must_use]
    pub const fn time_stop_bars(self) -> u64 {
        self.time_stop_bars
    }
}

/// The per-bar compression-bracket input.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompressionBracketInput<'a> {
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

/// The v10 compression-bracket strategy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompressionBracketStrategy {
    config: CompressionBracketConfig,
    risk: RiskPolicy,
    require_contiguous: bool,
}

impl CompressionBracketStrategy {
    /// Builds the strategy.
    #[must_use]
    pub const fn new(
        config: CompressionBracketConfig,
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
    pub const fn config(self) -> CompressionBracketConfig {
        self.config
    }

    /// Decides for one closed execution bar.
    #[must_use]
    pub fn decide(
        &self,
        input: &CompressionBracketInput<'_>,
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
        let Some((upper, lower)) = self.range(input) else {
            return Decision::Hold;
        };
        Decision::EnterBracket(BracketEntry {
            bar: identity(input),
            intent: BracketIntent {
                upper_price: upper,
                lower_price: lower,
                stop: StopSpec::AtrMultiple {
                    multiple: self.risk.stop_atr_multiple(),
                },
                target: TargetSpec::RewardToRisk {
                    multiple: self.risk.reward_to_risk(),
                },
                valid_for_bars: self.config.entry_valid_bars,
                max_slippage_bps: None,
            },
        })
    }

    /// The range boundaries when the current bar carries one unambiguous event.
    fn range(&self, input: &CompressionBracketInput<'_>) -> Option<(f64, f64)> {
        let mut range = None;
        for event in input.events {
            if event.event_type != self.config.event_type
                || event.subtype != self.config.event_subtype
                || event.source != self.config.event_source
                || event.available_bar_index != input.execution_bar_index
                || event.direction != 0
            {
                continue;
            }
            let (Some(upper), Some(lower)) = (event.primary_value, event.reference_value) else {
                continue;
            };
            if !upper.is_finite() || !lower.is_finite() || upper <= lower || lower <= 0.0 {
                continue;
            }
            match range {
                None => range = Some((upper, lower)),
                Some(existing) if existing != (upper, lower) => return None,
                Some(_) => {}
            }
        }
        range
    }
}

fn identity(input: &CompressionBracketInput<'_>) -> StrategyInput {
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
    use super::{CompressionBracketConfig, CompressionBracketInput, CompressionBracketStrategy};
    use crate::intent::{RiskPolicy, StrategyPosition};
    use crate::side::Side;
    use crate::strategy::decision::{Decision, ExitReason};
    use crate::strategy::event_input::EventView;

    fn config() -> CompressionBracketConfig {
        CompressionBracketConfig::new(60, 3, 8, 5, 1, 20).expect("a valid config")
    }

    fn event(upper: Option<f64>, lower: Option<f64>, bar: usize) -> EventView {
        EventView {
            event_type: 3,
            subtype: 8,
            source: 5,
            direction: 0,
            occurred_bar_index: bar,
            available_bar_index: bar,
            reference_code: None,
            primary_value: upper,
            reference_value: lower,
        }
    }

    fn input(events: &'static [EventView]) -> CompressionBracketInput<'static> {
        CompressionBracketInput {
            execution_timeframe_secs: 60,
            execution_bar_index: 10,
            execution_open_time: 600,
            execution_contiguous: true,
            events,
        }
    }

    fn flat() -> StrategyPosition {
        StrategyPosition::flat()
    }

    #[test]
    fn a_range_event_places_a_two_sided_bracket() {
        let strategy = CompressionBracketStrategy::new(config(), RiskPolicy::default(), true);
        let events = Box::leak(Box::new([event(Some(105.0), Some(95.0), 10)]));
        let decision = strategy.decide(&input(events), flat());
        let Decision::EnterBracket(entry) = decision else {
            panic!("a bracket entry")
        };
        assert_eq!(entry.intent.upper_price, 105.0);
        assert_eq!(entry.intent.lower_price, 95.0);
        assert_eq!(
            entry.intent.stop,
            crate::intent::StopSpec::AtrMultiple { multiple: 2.0 }
        );
        assert_eq!(
            entry.intent.target,
            crate::intent::TargetSpec::RewardToRisk { multiple: 1.5 }
        );
        assert_eq!(entry.intent.valid_for_bars, 1);
    }

    #[test]
    fn no_event_holds() {
        let strategy = CompressionBracketStrategy::new(config(), RiskPolicy::default(), true);
        assert_eq!(strategy.decide(&input(&[]), flat()), Decision::Hold);
    }

    #[test]
    fn an_event_on_a_previous_bar_holds() {
        let strategy = CompressionBracketStrategy::new(config(), RiskPolicy::default(), true);
        let events = Box::leak(Box::new([event(Some(105.0), Some(95.0), 9)]));
        assert_eq!(strategy.decide(&input(events), flat()), Decision::Hold);
    }

    #[test]
    fn a_missing_or_invalid_range_holds() {
        let strategy = CompressionBracketStrategy::new(config(), RiskPolicy::default(), true);
        for event in [
            event(None, Some(95.0), 10),
            event(Some(105.0), None, 10),
            event(Some(95.0), Some(105.0), 10),
            event(Some(105.0), Some(105.0), 10),
        ] {
            let events = Box::leak(Box::new([event]));
            assert_eq!(strategy.decide(&input(events), flat()), Decision::Hold);
        }
    }

    #[test]
    fn a_directional_event_does_not_place_a_bracket() {
        let strategy = CompressionBracketStrategy::new(config(), RiskPolicy::default(), true);
        let mut event = event(Some(105.0), Some(95.0), 10);
        event.direction = 1;
        let events = Box::leak(Box::new([event]));
        assert_eq!(strategy.decide(&input(events), flat()), Decision::Hold);
    }

    #[test]
    fn the_time_stop_exits_an_open_position() {
        let strategy = CompressionBracketStrategy::new(config(), RiskPolicy::default(), true);
        let position = StrategyPosition::opened(Side::Short, 0);
        let position = StrategyPosition {
            bars_held: 20,
            ..position
        };
        let decision = strategy.decide(&input(&[]), position);
        let Decision::Exit(exit) = decision else {
            panic!("an exit")
        };
        assert_eq!(exit.reason, ExitReason::TimeStop);
        assert_eq!(exit.side, Side::Short);
    }

    #[test]
    fn a_non_contiguous_bar_blocks_a_bracket() {
        let strategy = CompressionBracketStrategy::new(config(), RiskPolicy::default(), true);
        let events = Box::leak(Box::new([event(Some(105.0), Some(95.0), 10)]));
        let input = CompressionBracketInput {
            execution_contiguous: false,
            ..input(events)
        };
        assert_eq!(strategy.decide(&input, flat()), Decision::Hold);
    }

    #[test]
    fn the_strategy_is_deterministic() {
        let strategy = CompressionBracketStrategy::new(config(), RiskPolicy::default(), true);
        let events = Box::leak(Box::new([event(Some(105.0), Some(95.0), 10)]));
        assert_eq!(
            strategy.decide(&input(events), flat()),
            strategy.decide(&input(events), flat())
        );
    }
}
