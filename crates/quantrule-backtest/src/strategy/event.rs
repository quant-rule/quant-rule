//! The v8 event-composite strategy and its bounded condition language.
//!
//! A condition names one published event or one v3 discrete state on one
//! timeframe. The entry is the conjunction of its conditions; at least one
//! condition must be a directional event, and every entry event must agree on
//! the side. The exit is a separate disjunction of conditions. There is no
//! weighted score, no axis input, and no expression language.
//!
//! The module is pure: it reads the event/state snapshot and the position it is
//! given. It performs no I/O, reads no clock, and uses no randomness.

use crate::intent::{RiskPolicy, StrategyPosition};
use crate::side::Side;

use super::decision::{Decision, Entry, Exit, ExitReason};
use super::event_input::{EventStrategyInput, EventTimeframeView};

/// The maximum number of conditions in one entry or exit list.
pub const MAX_CONDITIONS: usize = 6;

/// A v3 state a v8 condition may read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateField {
    /// EMA stack state.
    EmaStack,
    /// RSI zone.
    RsiZone,
    /// MACD line/signal state.
    MacdState,
    /// MACD histogram sign.
    MacdHistogramState,
    /// ADX regime.
    AdxRegime,
    /// ADX slope.
    AdxSlope,
    /// DI direction.
    DiState,
    /// Progression stage.
    ProgressionStage,
    /// Progression leg direction.
    ProgressionLegDirection,
}

impl StateField {
    /// Parses the stable snake_case name.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "ema_stack" => Some(Self::EmaStack),
            "rsi_zone" => Some(Self::RsiZone),
            "macd_state" => Some(Self::MacdState),
            "macd_histogram_state" => Some(Self::MacdHistogramState),
            "adx_regime" => Some(Self::AdxRegime),
            "adx_slope" => Some(Self::AdxSlope),
            "di_state" => Some(Self::DiState),
            "progression_stage" => Some(Self::ProgressionStage),
            "progression_leg_direction" => Some(Self::ProgressionLegDirection),
            _ => None,
        }
    }

    /// The valid value range for the field.
    #[must_use]
    pub const fn accepts(self, value: i8) -> bool {
        match self {
            Self::EmaStack
            | Self::RsiZone
            | Self::MacdState
            | Self::MacdHistogramState
            | Self::AdxSlope
            | Self::DiState
            | Self::ProgressionLegDirection => value >= -1 && value <= 1,
            Self::AdxRegime => value >= 0 && value <= 1,
            Self::ProgressionStage => value >= 0 && value <= 5,
        }
    }
}

/// One v8 condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Condition {
    /// A published event with the exact type, subtype, source, and direction.
    Event {
        /// The source timeframe in seconds.
        timeframe_secs: u32,
        /// The event type code.
        event_type: u8,
        /// The event subtype code.
        subtype: u8,
        /// The stable source code.
        source: u8,
        /// The required direction.
        direction: i8,
    },
    /// A v3 state with the exact value.
    State {
        /// The source timeframe in seconds.
        timeframe_secs: u32,
        /// The state field.
        field: StateField,
        /// The required value.
        value: i8,
    },
}

impl Condition {
    fn timeframe_secs(self) -> u64 {
        match self {
            Self::Event { timeframe_secs, .. } | Self::State { timeframe_secs, .. } => {
                u64::from(timeframe_secs)
            }
        }
    }

    fn matches(self, input: &EventStrategyInput) -> bool {
        let Some(view) = input.timeframe(self.timeframe_secs()) else {
            return false;
        };
        match self {
            Self::Event {
                event_type,
                subtype,
                source,
                direction,
                ..
            } => view.events.iter().any(|event| {
                event.event_type == event_type
                    && event.subtype == subtype
                    && event.source == u16::from(source)
                    && event.direction == direction
            }),
            Self::State { field, value, .. } => state_value(field, view) == Some(value),
        }
    }
}

/// The v8 event-composite configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EventCompositeConfig {
    window_bars: usize,
    entry: [Condition; MAX_CONDITIONS],
    entry_len: usize,
    exit: [Condition; MAX_CONDITIONS],
    exit_len: usize,
}

impl EventCompositeConfig {
    /// Builds a bounded event-composite configuration.
    ///
    /// # Errors
    ///
    /// Returns [`super::config::StrategyConfigError`] when a list is empty
    /// where required, too long, contains an invalid condition, contains a
    /// duplicate, or contains entry events that do not agree on direction.
    pub fn new(
        window_bars: usize,
        entry: &[Condition],
        exit: &[Condition],
    ) -> Result<Self, super::config::StrategyConfigError> {
        use super::config::StrategyConfigError;

        if window_bars == 0 {
            return Err(StrategyConfigError::EventWindowBars);
        }
        if entry.is_empty() {
            return Err(StrategyConfigError::EventEmptyEntry);
        }
        if entry.len() > MAX_CONDITIONS || exit.len() > MAX_CONDITIONS {
            return Err(StrategyConfigError::EventTooManyConditions);
        }
        let mut entry_conditions = [Condition::State {
            timeframe_secs: 1,
            field: StateField::EmaStack,
            value: 0,
        }; MAX_CONDITIONS];
        let mut exit_conditions = entry_conditions;
        let mut entry_side = 0_i8;
        for (index, condition) in entry.iter().copied().enumerate() {
            validate_condition(condition)?;
            if entry[..index].contains(&condition) {
                return Err(StrategyConfigError::EventDuplicateCondition);
            }
            if let Condition::Event { direction, .. } = condition {
                if direction == 0 {
                    return Err(StrategyConfigError::EventEntryDirection);
                }
                if entry_side == 0 {
                    entry_side = direction;
                } else if entry_side != direction {
                    return Err(StrategyConfigError::EventDirectionConflict);
                }
            }
            entry_conditions[index] = condition;
        }
        if entry_side == 0 {
            return Err(StrategyConfigError::EventEntryDirection);
        }
        for (index, condition) in exit.iter().copied().enumerate() {
            validate_condition(condition)?;
            if exit[..index].contains(&condition) {
                return Err(StrategyConfigError::EventDuplicateCondition);
            }
            exit_conditions[index] = condition;
        }
        Ok(Self {
            window_bars,
            entry: entry_conditions,
            entry_len: entry.len(),
            exit: exit_conditions,
            exit_len: exit.len(),
        })
    }

    /// The event window in bars.
    #[must_use]
    pub const fn window_bars(self) -> usize {
        self.window_bars
    }

    /// The entry conditions.
    #[must_use]
    pub fn entry_conditions(&self) -> &[Condition] {
        &self.entry[..self.entry_len]
    }

    /// The exit conditions.
    #[must_use]
    pub fn exit_conditions(&self) -> &[Condition] {
        &self.exit[..self.exit_len]
    }
}

fn validate_condition(condition: Condition) -> Result<(), super::config::StrategyConfigError> {
    use super::config::StrategyConfigError;

    match condition {
        Condition::Event {
            timeframe_secs,
            event_type,
            subtype,
            source,
            direction,
        } => {
            if timeframe_secs == 0 {
                return Err(StrategyConfigError::EventTimeframe);
            }
            if !(1..=3).contains(&event_type) {
                return Err(StrategyConfigError::EventType);
            }
            if !(1..=7).contains(&subtype) {
                return Err(StrategyConfigError::EventSubtype);
            }
            if !(1..=18).contains(&source) {
                return Err(StrategyConfigError::EventSource);
            }
            if !(-1..=1).contains(&direction) {
                return Err(StrategyConfigError::EventDirection);
            }
        }
        Condition::State {
            timeframe_secs,
            field,
            value,
        } => {
            if timeframe_secs == 0 {
                return Err(StrategyConfigError::EventTimeframe);
            }
            if !field.accepts(value) {
                return Err(StrategyConfigError::EventStateValue);
            }
        }
    }
    Ok(())
}

fn state_value(field: StateField, view: &EventTimeframeView) -> Option<i8> {
    match field {
        StateField::EmaStack => view.states.ema_stack,
        StateField::RsiZone => view.states.rsi_zone,
        StateField::MacdState => view.states.macd_state,
        StateField::MacdHistogramState => view.states.macd_histogram_state,
        StateField::AdxRegime => view.states.adx_regime,
        StateField::AdxSlope => view.states.adx_slope,
        StateField::DiState => view.states.di_state,
        StateField::ProgressionStage => view.progression_stage,
        StateField::ProgressionLegDirection => view.progression_leg_direction,
    }
}

/// The v8 event-composite strategy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EventCompositeStrategy {
    config: EventCompositeConfig,
    risk: RiskPolicy,
    require_contiguous: bool,
}

impl EventCompositeStrategy {
    /// Builds the strategy.
    #[must_use]
    pub const fn new(
        config: EventCompositeConfig,
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
    pub const fn config(&self) -> EventCompositeConfig {
        self.config
    }

    /// Decides for one closed execution bar.
    #[must_use]
    pub fn decide(&self, input: &EventStrategyInput, position: StrategyPosition) -> Decision {
        if position.is_open() {
            if self
                .config
                .exit_conditions()
                .iter()
                .any(|condition| condition.matches(input))
            {
                let Some(side) = position.side else {
                    return Decision::Hold;
                };
                return Decision::Exit(Exit {
                    side,
                    bar: identity(input),
                    reason: ExitReason::EventCondition,
                });
            }
            return Decision::Hold;
        }
        if self.require_contiguous && !input.execution_contiguous {
            return Decision::Hold;
        }
        if !self
            .config
            .entry_conditions()
            .iter()
            .all(|condition| condition.matches(input))
        {
            return Decision::Hold;
        }
        let side = self
            .config
            .entry_conditions()
            .iter()
            .find_map(|condition| match condition {
                Condition::Event { direction, .. } if *direction > 0 => Some(Side::Long),
                Condition::Event { direction, .. } if *direction < 0 => Some(Side::Short),
                _ => None,
            });
        let Some(side) = side else {
            return Decision::Hold;
        };
        Decision::Enter(Entry {
            bar: identity(input),
            intent: self.risk.intent(side),
        })
    }
}

fn identity(input: &EventStrategyInput) -> super::input::StrategyInput {
    super::input::StrategyInput::new(
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
    use super::{Condition, EventCompositeConfig, EventCompositeStrategy, StateField};
    use crate::intent::{RiskPolicy, StrategyPosition};
    use crate::side::Side;
    use crate::strategy::decision::Decision;
    use crate::strategy::event_input::{EventStrategyInput, EventTimeframeView};
    use quantrule_model::timeframe::ModelStates;

    fn states() -> ModelStates {
        ModelStates {
            ema_stack: Some(1),
            rsi_zone: Some(0),
            macd_state: Some(1),
            macd_histogram_state: Some(1),
            adx_regime: Some(1),
            adx_slope: Some(1),
            di_state: Some(1),
        }
    }

    fn input() -> EventStrategyInput {
        EventStrategyInput {
            execution_timeframe_secs: 900,
            execution_bar_index: 10,
            execution_open_time: 9_000,
            execution_contiguous: true,
            timeframes: vec![EventTimeframeView {
                timeframe_secs: 900,
                bar_index: 10,
                open_time: 9_000,
                states: states(),
                progression_stage: None,
                progression_leg_direction: None,
                events: Vec::new(),
                context: None,
            }],
        }
    }

    #[test]
    fn a_state_only_entry_is_rejected() {
        let result = EventCompositeConfig::new(
            3,
            &[Condition::State {
                timeframe_secs: 900,
                field: StateField::EmaStack,
                value: 1,
            }],
            &[],
        );
        assert!(result.is_err());
    }

    #[test]
    fn a_directional_event_entry_opens_the_side() {
        let config = EventCompositeConfig::new(
            3,
            &[Condition::Event {
                timeframe_secs: 900,
                event_type: 3,
                subtype: 6,
                source: 16,
                direction: 1,
            }],
            &[],
        )
        .expect("the config is valid");
        let mut input = input();
        input.timeframes[0].events = vec![crate::strategy::event_input::EventView {
            event_type: 3,
            subtype: 6,
            source: 16,
            direction: 1,
            occurred_bar_index: 10,
            available_bar_index: 10,
        }];
        let strategy = EventCompositeStrategy::new(config, RiskPolicy::default(), true);
        let Decision::Enter(entry) = strategy.decide(&input, StrategyPosition::flat()) else {
            panic!("the entry must fire");
        };
        assert_eq!(entry.side(), Side::Long);
    }

    #[test]
    fn a_conflicting_entry_is_rejected() {
        let result = EventCompositeConfig::new(
            3,
            &[
                Condition::Event {
                    timeframe_secs: 900,
                    event_type: 3,
                    subtype: 6,
                    source: 16,
                    direction: 1,
                },
                Condition::Event {
                    timeframe_secs: 900,
                    event_type: 3,
                    subtype: 2,
                    source: 17,
                    direction: -1,
                },
            ],
            &[],
        );
        assert!(result.is_err());
    }

    #[test]
    fn an_exit_condition_closes_the_position() {
        let config = EventCompositeConfig::new(
            3,
            &[Condition::Event {
                timeframe_secs: 900,
                event_type: 3,
                subtype: 6,
                source: 16,
                direction: 1,
            }],
            &[Condition::State {
                timeframe_secs: 900,
                field: StateField::EmaStack,
                value: -1,
            }],
        )
        .expect("the config is valid");
        let mut input = input();
        input.timeframes[0].states.ema_stack = Some(-1);
        let strategy = EventCompositeStrategy::new(config, RiskPolicy::default(), true);
        let Decision::Exit(exit) = strategy.decide(&input, StrategyPosition::with_side(Side::Long))
        else {
            panic!("the exit must fire");
        };
        assert_eq!(
            exit.reason,
            crate::strategy::decision::ExitReason::EventCondition
        );
    }
}
