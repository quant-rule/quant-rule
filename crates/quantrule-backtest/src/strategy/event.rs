//! The event-composite strategy engine and its bounded condition language.
//!
//! A condition names one published event or one v3 discrete state on one
//! timeframe. The entry is the conjunction of its conditions; at least one
//! condition must be an event, and every nonzero entry event must agree on the
//! side. An event condition with `direction: 0` matches either side and takes
//! the side of the matched event; if the matched entry events disagree, the bar
//! produces no entry. The exit is a separate disjunction of conditions, and an
//! optional time stop closes a position after a fixed number of execution bars.
//! There is no weighted score, no axis input, and no expression language.
//!
//! The module is pure: it reads the event/state snapshot and the position it is
//! given. It performs no I/O, reads no clock, and uses no randomness.

use crate::intent::{RiskPolicy, StrategyPosition};
use crate::side::Side;

use super::decision::{Decision, Entry, Exit, ExitReason};
use super::event_input::{EventStrategyInput, EventTimeframeView};

/// The maximum number of conditions in one entry or exit list.
pub const MAX_CONDITIONS: usize = 6;

/// A state a condition may read.
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
    /// Relative-volume regime.
    RvolRegime,
    /// Volatility regime.
    VolRegime,
    /// Confirmed major structure state.
    StructureState,
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
            "rvol_regime" => Some(Self::RvolRegime),
            "vol_regime" => Some(Self::VolRegime),
            "structure_state" => Some(Self::StructureState),
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
            | Self::RvolRegime
            | Self::StructureState
            | Self::ProgressionLegDirection => value >= -1 && value <= 1,
            Self::AdxRegime => value >= 0 && value <= 1,
            Self::VolRegime => value >= 0 && value <= 2,
            Self::ProgressionStage => value >= 0 && value <= 5,
        }
    }
}

/// One condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Condition {
    /// A published event with the exact type, subtype, and source.
    Event {
        /// The source timeframe in seconds.
        timeframe_secs: u32,
        /// The event type code.
        event_type: u8,
        /// The event subtype code.
        subtype: u8,
        /// The stable source code.
        source: u8,
        /// The required direction. Zero matches either sign and takes the
        /// matched event's direction; a nonzero value filters exactly.
        direction: i8,
        /// Whether the entry takes the opposite side of the matched event.
        contrarian: bool,
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
        match self {
            Self::Event { .. } => !self.matched_event_directions(input).is_empty(),
            Self::State {
                timeframe_secs,
                field,
                value,
            } => {
                let Some(view) = input.timeframe(u64::from(timeframe_secs)) else {
                    return false;
                };
                state_value(field, view) == Some(value)
            }
        }
    }

    /// The directions of every event that satisfies this condition.
    ///
    /// A zero direction matches either sign, so the caller can take the side
    /// from the matched event. Neutral events never satisfy an event condition:
    /// they cannot choose a side.
    fn matched_event_directions(self, input: &EventStrategyInput) -> Vec<i8> {
        let Some(view) = input.timeframe(self.timeframe_secs()) else {
            return Vec::new();
        };
        let Self::Event {
            event_type,
            subtype,
            source,
            direction,
            contrarian,
            ..
        } = self
        else {
            return Vec::new();
        };
        view.events
            .iter()
            .filter(|event| {
                event.event_type == event_type
                    && event.subtype == subtype
                    && event.source == u16::from(source)
                    && event.direction != 0
                    && (direction == 0 || event.direction == direction)
            })
            .map(|event| {
                if contrarian {
                    -event.direction
                } else {
                    event.direction
                }
            })
            .collect()
    }
}

/// The event-composite strategy configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EventCompositeConfig {
    window_bars: usize,
    time_stop_bars: u64,
    entry: [Condition; MAX_CONDITIONS],
    entry_len: usize,
    exit: [Condition; MAX_CONDITIONS],
    exit_len: usize,
    giveback_atr: Option<f64>,
    giveback_arm_atr: f64,
}

impl EventCompositeConfig {
    /// Builds a bounded event-composite configuration.
    ///
    /// `time_stop_bars` closes a position after that many execution bars; zero
    /// disables the time stop. The event window must be at least one bar.
    ///
    /// # Errors
    ///
    /// Returns [`super::config::StrategyConfigError`] when a list is empty
    /// where required, too long, contains an invalid condition, contains a
    /// duplicate, has no entry event, or has nonzero entry events that
    /// disagree on direction.
    pub fn new(
        window_bars: usize,
        time_stop_bars: u64,
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
        let mut entry_events = 0_usize;
        for (index, condition) in entry.iter().copied().enumerate() {
            validate_condition(condition)?;
            if entry[..index].contains(&condition) {
                return Err(StrategyConfigError::EventDuplicateCondition);
            }
            if let Condition::Event { direction, .. } = condition {
                entry_events += 1;
                if direction != 0 {
                    if entry_side == 0 {
                        entry_side = direction;
                    } else if entry_side != direction {
                        return Err(StrategyConfigError::EventDirectionConflict);
                    }
                }
            }
            entry_conditions[index] = condition;
        }
        if entry_events == 0 {
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
            time_stop_bars,
            entry: entry_conditions,
            entry_len: entry.len(),
            exit: exit_conditions,
            exit_len: exit.len(),
            giveback_atr: None,
            giveback_arm_atr: 0.0,
        })
    }

    /// Enables the giveback exit with the given threshold in entry ATR.
    ///
    /// The exit fires when `giveback_atr >= threshold` and the position has
    /// reached `giveback_arm_atr` of favorable excursion. A non-finite or
    /// non-positive threshold is rejected.
    ///
    /// # Errors
    ///
    /// Returns [`super::config::StrategyConfigError::GivebackAtr`] when the
    /// threshold or arm is invalid.
    pub fn with_giveback_atr(
        mut self,
        giveback_atr: f64,
        giveback_arm_atr: f64,
    ) -> Result<Self, super::config::StrategyConfigError> {
        use super::config::StrategyConfigError;
        if !giveback_atr.is_finite()
            || giveback_atr <= 0.0
            || !giveback_arm_atr.is_finite()
            || giveback_arm_atr < 0.0
        {
            return Err(StrategyConfigError::GivebackAtr);
        }
        self.giveback_atr = Some(giveback_atr);
        self.giveback_arm_atr = giveback_arm_atr;
        Ok(self)
    }

    /// The giveback threshold in entry ATR, when enabled.
    #[must_use]
    pub const fn giveback_atr(self) -> Option<f64> {
        self.giveback_atr
    }

    /// The favorable excursion required before the giveback exit can fire.
    #[must_use]
    pub const fn giveback_arm_atr(self) -> f64 {
        self.giveback_arm_atr
    }

    /// The event window in bars.
    #[must_use]
    pub const fn window_bars(self) -> usize {
        self.window_bars
    }

    /// The maximum holding period in execution bars; zero disables it.
    #[must_use]
    pub const fn time_stop_bars(self) -> u64 {
        self.time_stop_bars
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
            contrarian: _,
        } => {
            if timeframe_secs == 0 {
                return Err(StrategyConfigError::EventTimeframe);
            }
            if !(1..=3).contains(&event_type) {
                return Err(StrategyConfigError::EventType);
            }
            if subtype == 0 {
                return Err(StrategyConfigError::EventSubtype);
            }
            if !(1..=57).contains(&source) {
                return Err(StrategyConfigError::EventSource);
            }
            let Some(identity) = super::registry::lookup(event_type, subtype, u16::from(source))
            else {
                return Err(StrategyConfigError::EventNotRegistered);
            };
            if identity.role == super::registry::EventRole::Diagnostic {
                // The registry is the authority on the exact identity; this
                // check keeps a diagnostic from becoming a trigger even when
                // its subtype falls inside the accepted range.
                return Err(StrategyConfigError::EventDiagnostic);
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
        StateField::RvolRegime => view.states.rvol_regime,
        StateField::VolRegime => view.states.vol_regime,
        StateField::StructureState => view.states.structure_state,
        StateField::ProgressionStage => view.progression_stage,
        StateField::ProgressionLegDirection => view.progression_leg_direction,
    }
}

/// The event-composite strategy.
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
            if self.config.time_stop_bars() > 0
                && position.bars_held >= self.config.time_stop_bars()
            {
                let Some(side) = position.side else {
                    return Decision::Hold;
                };
                return Decision::Exit(Exit {
                    side,
                    bar: identity(input),
                    reason: ExitReason::TimeStop,
                });
            }
            if let Some(threshold) = self.config.giveback_atr
                && position
                    .mfe_atr
                    .is_some_and(|mfe| mfe >= self.config.giveback_arm_atr)
                && position
                    .giveback_atr()
                    .is_some_and(|giveback| giveback >= threshold)
            {
                let Some(side) = position.side else {
                    return Decision::Hold;
                };
                return Decision::Exit(Exit {
                    side,
                    bar: identity(input),
                    reason: ExitReason::Giveback,
                });
            }
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
        let mut side = None;
        for condition in self.config.entry_conditions() {
            for direction in condition.matched_event_directions(input) {
                let candidate = if direction > 0 {
                    Side::Long
                } else {
                    Side::Short
                };
                match side {
                    None => side = Some(candidate),
                    Some(existing) if existing != candidate => return Decision::Hold,
                    Some(_) => {}
                }
            }
        }
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
    let execution = input.timeframe(input.execution_timeframe_secs);
    super::input::StrategyInput::new(
        input.execution_timeframe_secs,
        input.execution_bar_index,
        input.execution_open_time,
    )
    .with_progression(
        execution.and_then(|view| view.progression_leg_direction),
        execution.and_then(|view| view.progression_stage),
    )
    .with_contiguous(input.execution_contiguous)
}

#[cfg(test)]
mod tests;
