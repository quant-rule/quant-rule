//! The resolved `quantrule.model-spec/v2` contract and its causal spec run.
//!
//! Version 2 adds reading overrides, an active-event set, a directional vote,
//! and optional panel/session context to the condition-only v1 contract. The
//! shell parses JSON and resolves registry identifiers; this module receives
//! numeric codes and validated configurations, and performs no I/O.
//!
//! The prediction is causal: at bar `t` it counts only event records whose
//! `available_bar_index` equals `t`. It never reads a forward outcome, a
//! strategy position, cash, or P&L.

use std::collections::{BTreeMap, BTreeSet};

use crate::events::panel::PanelEventSeries;
use crate::events::session::SessionEventSeries;
use crate::events::{EventRecord, EventType};
use crate::input::MarketStateConfig;
use crate::panel::{PanelConfig, PanelInstrument, PanelReadings};
use crate::run::ModelRun;
use crate::sessions::{SessionConfig, SessionReadings};
use crate::spec::{
    ConditionResult, MAX_CONDITIONS, SpecCondition, SpecError, SpecEvaluation, SpecExpression,
};

/// The only v2 schema version this module evaluates.
pub const SPEC_V2_SCHEMA: &str = "quantrule.model-spec/v2";
/// The maximum active-event set.
pub const MAX_ACTIVE_EVENTS: usize = 256;
/// The maximum vote threshold.
pub const MAX_MIN_VOTES: usize = 64;

type EventIndex = BTreeMap<u16, Vec<(usize, i8)>>;

/// The role a registry event declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecEventRole {
    /// A directional claim that may vote.
    Signal,
    /// A condition or modulator; it may not vote.
    Context,
    /// A descriptor; it may not vote and never enters a strategy.
    Diagnostic,
}

impl SpecEventRole {
    /// The role's stable code.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::Signal => 1,
            Self::Context => 2,
            Self::Diagnostic => 3,
        }
    }
}

/// One resolved registry event used by a v2 spec.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecEventCode {
    /// The registry event identifier.
    pub id: String,
    /// The stable registry event code.
    pub code: u16,
    /// The registry role.
    pub role: SpecEventRole,
    /// The event type.
    pub event_type: EventType,
    /// The event subtype code.
    pub subtype: u8,
    /// The stable source code.
    pub source: u16,
}

/// The prediction kind. Version 2 supports the directional vote only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PredictionKind {
    /// Count long and short event lists; the sign above the threshold wins.
    Vote,
    /// Count each active signal event's own direction; the sign above the
    /// threshold wins.
    Direction,
}

/// The resolved vote definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecPrediction {
    /// The prediction kind.
    pub kind: PredictionKind,
    /// Events that add a long vote.
    pub long: Vec<SpecEventCode>,
    /// Events that add a short vote.
    pub short: Vec<SpecEventCode>,
    /// Events whose own direction votes, for [`PredictionKind::Direction`].
    pub events: Vec<SpecEventCode>,
    /// The minimum absolute vote count that emits a non-zero prediction.
    pub min_votes: usize,
}

/// A resolved v2 model spec.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelSpecV2 {
    /// The schema version; it must equal [`SPEC_V2_SCHEMA`].
    pub schema: String,
    /// The stable model identifier, such as `M2.1`.
    pub id: String,
    /// The model family, such as `M2`.
    pub family: String,
    /// The evaluated instrument.
    pub instrument: String,
    /// The single timeframe in seconds.
    pub timeframes: Vec<u64>,
    /// The evaluated timeframe; equal to the single `timeframes` entry.
    pub evaluation_timeframe_secs: u64,
    /// The resolved v4 and extended reading configuration.
    pub readings: MarketStateConfig,
    /// Every active registry event, in document order.
    pub active_events: Vec<SpecEventCode>,
    /// The directional vote.
    pub prediction: SpecPrediction,
    /// Optional v1-style diagnostic conditions over the base model run.
    pub conditions: Vec<SpecCondition>,
    /// Optional session calendar for session events.
    pub session: Option<SessionConfig>,
    /// The panel instrument names, leader first. Empty when no panel is used.
    pub panel_instruments: Vec<String>,
}

/// A v2 spec that failed validation or could not be evaluated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpecV2Error {
    /// The schema is not [`SPEC_V2_SCHEMA`].
    UnsupportedSchema,
    /// The id is empty.
    EmptyId,
    /// The family is empty.
    EmptyFamily,
    /// The instrument is empty.
    EmptyInstrument,
    /// The spec must declare exactly one timeframe.
    TimeframeCount(usize),
    /// A timeframe is zero.
    ZeroTimeframe,
    /// The evaluation timeframe is not the single declared timeframe.
    UnknownEvaluationTimeframe(u64),
    /// A reading configuration is invalid.
    InvalidReadings,
    /// The active-event set is empty.
    EmptyActiveEvents,
    /// The active-event set is too large.
    TooManyActiveEvents(usize),
    /// An active event id is duplicated.
    DuplicateActiveEvent(String),
    /// A prediction event is not in the active set.
    PredictionEventNotActive(String),
    /// A prediction event is not a `signal` role.
    PredictionEventNotSignal(String),
    /// A prediction event is duplicated.
    DuplicatePredictionEvent(String),
    /// The prediction has no events.
    EmptyPrediction,
    /// The prediction fields do not match its kind.
    InvalidPredictionShape(&'static str),
    /// The vote threshold is outside 1..=[`MAX_MIN_VOTES`].
    InvalidMinVotes(usize),
    /// A panel instrument name is empty.
    EmptyPanelInstrument,
    /// A panel instrument is duplicated.
    DuplicatePanelInstrument(String),
    /// The evaluated instrument is absent from the panel.
    InstrumentNotInPanel(String),
    /// A panel instrument has no candles.
    MissingPanelInstrument(String),
    /// The panel was passed without being declared.
    UnexpectedPanel,
    /// A panel was declared but has fewer than two instruments.
    PanelTooSmall(usize),
    /// The session start and end are invalid.
    InvalidSession,
    /// There are too many conditions.
    TooManyConditions(usize),
    /// A condition failed.
    Conditions(SpecError),
    /// The evaluation timeframe is absent from the run.
    MissingTimeframe(u64),
}

impl std::fmt::Display for SpecV2Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedSchema => write!(formatter, "the schema is not {SPEC_V2_SCHEMA}"),
            Self::EmptyId => write!(formatter, "the spec id is empty"),
            Self::EmptyFamily => write!(formatter, "the family is empty"),
            Self::EmptyInstrument => write!(formatter, "the instrument is empty"),
            Self::TimeframeCount(count) => {
                write!(
                    formatter,
                    "the v2 spec must declare exactly one timeframe, got {count}"
                )
            }
            Self::ZeroTimeframe => write!(formatter, "the timeframe is zero"),
            Self::UnknownEvaluationTimeframe(timeframe) => write!(
                formatter,
                "the evaluation timeframe {timeframe} is not the declared timeframe"
            ),
            Self::InvalidReadings => write!(formatter, "a reading configuration is invalid"),
            Self::EmptyActiveEvents => write!(formatter, "the active-event set is empty"),
            Self::TooManyActiveEvents(count) => write!(
                formatter,
                "the active-event set has {count} entries, over the {MAX_ACTIVE_EVENTS} limit"
            ),
            Self::DuplicateActiveEvent(id) => {
                write!(formatter, "the active event {id} is duplicated")
            }
            Self::PredictionEventNotActive(id) => {
                write!(formatter, "the prediction event {id} is not active")
            }
            Self::PredictionEventNotSignal(id) => {
                write!(
                    formatter,
                    "the prediction event {id} is not a signal-role event"
                )
            }
            Self::DuplicatePredictionEvent(id) => {
                write!(formatter, "the prediction event {id} is duplicated")
            }
            Self::EmptyPrediction => write!(formatter, "the prediction has no events"),
            Self::InvalidPredictionShape(message) => write!(formatter, "{message}"),
            Self::InvalidMinVotes(min_votes) => write!(
                formatter,
                "min_votes {min_votes} is outside 1..={MAX_MIN_VOTES}"
            ),
            Self::EmptyPanelInstrument => write!(formatter, "a panel instrument name is empty"),
            Self::DuplicatePanelInstrument(name) => {
                write!(formatter, "the panel instrument {name} is duplicated")
            }
            Self::InstrumentNotInPanel(name) => {
                write!(
                    formatter,
                    "the evaluated instrument {name} is not in the panel"
                )
            }
            Self::MissingPanelInstrument(name) => {
                write!(formatter, "the panel instrument {name} has no candles")
            }
            Self::UnexpectedPanel => write!(formatter, "a panel was passed without being declared"),
            Self::PanelTooSmall(count) => {
                write!(
                    formatter,
                    "the panel has {count} instruments; at least two are required"
                )
            }
            Self::InvalidSession => {
                write!(formatter, "the session hours or opening range are invalid")
            }
            Self::TooManyConditions(count) => write!(
                formatter,
                "the spec has {count} conditions, over the {MAX_CONDITIONS} limit"
            ),
            Self::Conditions(error) => write!(formatter, "a condition is invalid: {error}"),
            Self::MissingTimeframe(timeframe) => {
                write!(formatter, "the run does not contain timeframe {timeframe}")
            }
        }
    }
}

impl std::error::Error for SpecV2Error {}

impl ModelSpecV2 {
    /// Validates the spec without evaluating it.
    ///
    /// # Errors
    ///
    /// Returns a [`SpecV2Error`] when the schema, single timeframe, readings,
    /// active-event set, prediction, panel, session, or conditions are invalid.
    pub fn validate(&self) -> Result<(), SpecV2Error> {
        if self.schema != SPEC_V2_SCHEMA {
            return Err(SpecV2Error::UnsupportedSchema);
        }
        if self.id.is_empty() {
            return Err(SpecV2Error::EmptyId);
        }
        if self.family.is_empty() {
            return Err(SpecV2Error::EmptyFamily);
        }
        if self.instrument.is_empty() {
            return Err(SpecV2Error::EmptyInstrument);
        }
        if self.timeframes.len() != 1 {
            return Err(SpecV2Error::TimeframeCount(self.timeframes.len()));
        }
        let timeframe = self.timeframes[0];
        if timeframe == 0 {
            return Err(SpecV2Error::ZeroTimeframe);
        }
        if self.evaluation_timeframe_secs != timeframe {
            return Err(SpecV2Error::UnknownEvaluationTimeframe(
                self.evaluation_timeframe_secs,
            ));
        }
        if !self.readings.readings_for(timeframe).is_valid()
            || !extended_is_valid(self.readings.extended())
        {
            return Err(SpecV2Error::InvalidReadings);
        }
        if self.active_events.is_empty() {
            return Err(SpecV2Error::EmptyActiveEvents);
        }
        if self.active_events.len() > MAX_ACTIVE_EVENTS {
            return Err(SpecV2Error::TooManyActiveEvents(self.active_events.len()));
        }
        let mut identifiers = BTreeSet::new();
        for event in &self.active_events {
            if !identifiers.insert(event.id.as_str()) {
                return Err(SpecV2Error::DuplicateActiveEvent(event.id.clone()));
            }
        }
        let active: BTreeMap<&str, &SpecEventCode> = self
            .active_events
            .iter()
            .map(|event| (event.id.as_str(), event))
            .collect();
        let mut prediction_events = BTreeSet::new();
        let mut validate_prediction_event = |event: &SpecEventCode| {
            if !active.contains_key(event.id.as_str()) {
                return Err(SpecV2Error::PredictionEventNotActive(event.id.clone()));
            }
            if event.role != SpecEventRole::Signal {
                return Err(SpecV2Error::PredictionEventNotSignal(event.id.clone()));
            }
            if !prediction_events.insert(event.id.clone()) {
                return Err(SpecV2Error::DuplicatePredictionEvent(event.id.clone()));
            }
            Ok(())
        };
        match self.prediction.kind {
            PredictionKind::Vote => {
                if !self.prediction.events.is_empty() {
                    return Err(SpecV2Error::InvalidPredictionShape(
                        "a vote prediction must not carry an events list",
                    ));
                }
                for event in self.prediction.long.iter().chain(&self.prediction.short) {
                    validate_prediction_event(event)?;
                }
                if self.prediction.long.is_empty() && self.prediction.short.is_empty() {
                    return Err(SpecV2Error::EmptyPrediction);
                }
            }
            PredictionKind::Direction => {
                if !self.prediction.long.is_empty() || !self.prediction.short.is_empty() {
                    return Err(SpecV2Error::InvalidPredictionShape(
                        "a direction prediction must not carry long or short lists",
                    ));
                }
                for event in &self.prediction.events {
                    validate_prediction_event(event)?;
                }
                if self.prediction.events.is_empty() {
                    return Err(SpecV2Error::EmptyPrediction);
                }
            }
        }
        if !(1..=MAX_MIN_VOTES).contains(&self.prediction.min_votes) {
            return Err(SpecV2Error::InvalidMinVotes(self.prediction.min_votes));
        }
        if !self.panel_instruments.is_empty() {
            let mut names = BTreeSet::new();
            for name in &self.panel_instruments {
                if name.is_empty() {
                    return Err(SpecV2Error::EmptyPanelInstrument);
                }
                if !names.insert(name.as_str()) {
                    return Err(SpecV2Error::DuplicatePanelInstrument(name.clone()));
                }
            }
            if !names.contains(self.instrument.as_str()) {
                return Err(SpecV2Error::InstrumentNotInPanel(self.instrument.clone()));
            }
        }
        if let Some(session) = self.session
            && (session.start_hour >= session.end_hour
                || session.end_hour > 24
                || session.opening_range_bars == 0)
        {
            return Err(SpecV2Error::InvalidSession);
        }
        if self.conditions.len() > MAX_CONDITIONS {
            return Err(SpecV2Error::TooManyConditions(self.conditions.len()));
        }
        Ok(())
    }
}

/// One bar's vote counts and prediction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PredictionSeries {
    /// The prediction: +1 long, -1 short, 0 flat.
    pub values: Vec<i8>,
    /// The long vote count per bar.
    pub long_votes: Vec<u16>,
    /// The short vote count per bar.
    pub short_votes: Vec<u16>,
}

impl PredictionSeries {
    /// The prediction at `bar`.
    #[must_use]
    pub fn value(&self, bar: usize) -> Option<i8> {
        self.values.get(bar).copied()
    }

    /// The number of bars.
    #[must_use]
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Whether the series is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

/// A completed v2 spec run: the base model, the extra event batches, the
/// conditions, and the causal prediction.
#[derive(Debug, Clone, PartialEq)]
pub struct SpecRun {
    base: ModelRun,
    spec_id: String,
    family: String,
    instrument: String,
    evaluation_timeframe_secs: u64,
    bars: usize,
    active_events: Vec<SpecEventCode>,
    prediction: PredictionSeries,
    conditions: Option<SpecEvaluation>,
    session_readings: Option<SessionReadings>,
    panel_readings: Option<PanelReadings>,
    session_events: Vec<EventRecord>,
    panel_events: Vec<EventRecord>,
}

impl SpecRun {
    /// Computes the spec run over a completed base model run.
    ///
    /// The caller must compute `run` with [`ModelSpecV2::readings`]; the spec
    /// does not rebuild the base model. The panel is empty when the spec
    /// declares no panel. Session events are computed when the spec declares a
    /// session. The prediction uses the base events, the session events, and the
    /// evaluated instrument's panel events.
    ///
    /// # Errors
    ///
    /// Returns a [`SpecV2Error`] when the spec is invalid, the evaluation
    /// timeframe is absent, a panel instrument is missing, or a condition fails.
    pub fn compute(
        spec: &ModelSpecV2,
        run: ModelRun,
        panel: &[PanelInstrument<'_>],
    ) -> Result<Self, SpecV2Error> {
        spec.validate()?;
        if spec.panel_instruments.is_empty() && !panel.is_empty() {
            return Err(SpecV2Error::UnexpectedPanel);
        }
        if !spec.panel_instruments.is_empty() {
            if panel.len() < 2 {
                return Err(SpecV2Error::PanelTooSmall(panel.len()));
            }
            let names: BTreeSet<&str> = panel.iter().map(|instrument| instrument.name).collect();
            for name in &spec.panel_instruments {
                if !names.contains(name.as_str()) {
                    return Err(SpecV2Error::MissingPanelInstrument(name.clone()));
                }
            }
        }
        let timeframe =
            run.timeframe(spec.evaluation_timeframe_secs)
                .ok_or(SpecV2Error::MissingTimeframe(
                    spec.evaluation_timeframe_secs,
                ))?;
        let bars = timeframe.len();
        let session_readings = spec.session.map(|config| {
            SessionReadings::compute(timeframe.candles(), timeframe.readings(), &config)
        });
        let session_events = match (&session_readings, spec.session) {
            (Some(sessions), Some(config)) => SessionEventSeries::compute(
                timeframe.candles(),
                timeframe.readings(),
                sessions,
                &config,
                spec.evaluation_timeframe_secs,
            )
            .events()
            .to_vec(),
            _ => Vec::new(),
        };
        let panel_readings = if spec.panel_instruments.is_empty() {
            None
        } else {
            Some(PanelReadings::compute(panel, &PanelConfig::DEFAULT))
        };
        let panel_events = match &panel_readings {
            Some(readings) => {
                let series =
                    PanelEventSeries::compute(panel, readings, spec.evaluation_timeframe_secs);
                series.events(&spec.instrument).to_vec()
            }
            None => Vec::new(),
        };
        let mut events: Vec<EventRecord> = timeframe
            .events()
            .iter()
            .chain(timeframe.extended_events().events())
            .chain(session_events.iter())
            .chain(panel_events.iter())
            .copied()
            .collect();
        events.sort_by_key(|event| (event.available_bar_index, event.occurred_bar_index));
        let conditions = if spec.conditions.is_empty() {
            None
        } else {
            Some(evaluate_conditions(spec, timeframe, &events))
        };
        let prediction = predict(spec, timeframe, &events);
        Ok(Self {
            base: run,
            spec_id: spec.id.clone(),
            family: spec.family.clone(),
            instrument: spec.instrument.clone(),
            evaluation_timeframe_secs: spec.evaluation_timeframe_secs,
            bars,
            active_events: spec.active_events.clone(),
            prediction,
            conditions,
            session_readings,
            panel_readings,
            session_events,
            panel_events,
        })
    }

    /// The base model run.
    #[must_use]
    pub const fn base(&self) -> &ModelRun {
        &self.base
    }

    /// The spec identifier.
    #[must_use]
    pub fn spec_id(&self) -> &str {
        &self.spec_id
    }

    /// The model family.
    #[must_use]
    pub fn family(&self) -> &str {
        &self.family
    }

    /// The evaluated instrument.
    #[must_use]
    pub fn instrument(&self) -> &str {
        &self.instrument
    }

    /// The evaluation timeframe in seconds.
    #[must_use]
    pub const fn evaluation_timeframe_secs(&self) -> u64 {
        self.evaluation_timeframe_secs
    }

    /// The number of evaluation bars.
    #[must_use]
    pub const fn bars(&self) -> usize {
        self.bars
    }

    /// The active events.
    #[must_use]
    pub fn active_events(&self) -> &[SpecEventCode] {
        &self.active_events
    }

    /// The causal prediction.
    #[must_use]
    pub const fn prediction(&self) -> &PredictionSeries {
        &self.prediction
    }

    /// The diagnostic conditions, when the spec declares them.
    #[must_use]
    pub const fn conditions(&self) -> Option<&SpecEvaluation> {
        self.conditions.as_ref()
    }

    /// The session readings for the evaluated instrument, when declared.
    #[must_use]
    pub const fn session_readings(&self) -> Option<&SessionReadings> {
        self.session_readings.as_ref()
    }

    /// The panel readings, when declared.
    #[must_use]
    pub const fn panel_readings(&self) -> Option<&PanelReadings> {
        self.panel_readings.as_ref()
    }

    /// The session events for the evaluated instrument.
    #[must_use]
    pub fn session_events(&self) -> &[EventRecord] {
        &self.session_events
    }

    /// The panel events for the evaluated instrument.
    #[must_use]
    pub fn panel_events(&self) -> &[EventRecord] {
        &self.panel_events
    }

    /// Every event available to the prediction, sorted by availability.
    ///
    /// The base v4 batch, the extended batch, the session batch, and the panel
    /// batch are merged here. The prediction and the `events.csv` artifact use
    /// this one view.
    #[must_use]
    pub fn prediction_events(&self) -> Vec<EventRecord> {
        let Some(timeframe) = self.base.timeframe(self.evaluation_timeframe_secs) else {
            return Vec::new();
        };
        let mut events: Vec<EventRecord> = timeframe
            .events()
            .iter()
            .chain(timeframe.extended_events().events())
            .chain(self.session_events.iter())
            .chain(self.panel_events.iter())
            .copied()
            .collect();
        events.sort_by_key(|event| (event.available_bar_index, event.occurred_bar_index));
        events
    }
}

fn evaluate_conditions(
    spec: &ModelSpecV2,
    timeframe: &crate::timeframe::TimeframeModel,
    events: &[EventRecord],
) -> SpecEvaluation {
    let mut index: EventIndex = BTreeMap::new();
    for event in events {
        index
            .entry(event.event_code)
            .or_default()
            .push((event.available_bar_index, event.direction_code));
    }
    for entries in index.values_mut() {
        entries.sort_unstable();
    }
    let mut conditions = Vec::with_capacity(spec.conditions.len());
    for condition in &spec.conditions {
        let mut values = Vec::with_capacity(timeframe.len());
        for bar in 0..timeframe.len() {
            values.push(evaluate_expression(
                &condition.expression,
                timeframe,
                &index,
                bar,
            ));
        }
        let true_count = values.iter().filter(|value| **value).count();
        conditions.push(ConditionResult {
            id: condition.id.clone(),
            values,
            true_count,
        });
    }
    SpecEvaluation {
        evaluation_timeframe_secs: spec.evaluation_timeframe_secs,
        bars: timeframe.len(),
        conditions,
    }
}

fn evaluate_expression(
    expression: &SpecExpression,
    timeframe: &crate::timeframe::TimeframeModel,
    index: &EventIndex,
    bar: usize,
) -> bool {
    match expression {
        SpecExpression::Event(event) => {
            let Some(entries) = index.get(&event.event_code) else {
                return false;
            };
            let start = bar.saturating_sub(event.within_bars);
            let first = entries.partition_point(|(available, _)| *available < start);
            entries[first..]
                .iter()
                .take_while(|(available, _)| *available <= bar)
                .any(|(_, direction)| {
                    event
                        .direction
                        .is_none_or(|expected| expected == *direction)
                })
        }
        SpecExpression::State(state) => timeframe
            .states_at(bar)
            .is_some_and(|states| state.state.value(&states) == Some(state.equals)),
        SpecExpression::All(children) => children
            .iter()
            .all(|child| evaluate_expression(child, timeframe, index, bar)),
        SpecExpression::Any(children) => children
            .iter()
            .any(|child| evaluate_expression(child, timeframe, index, bar)),
        SpecExpression::Not(child) => !evaluate_expression(child, timeframe, index, bar),
    }
}

fn predict(
    spec: &ModelSpecV2,
    timeframe: &crate::timeframe::TimeframeModel,
    events: &[EventRecord],
) -> PredictionSeries {
    let mut sides = BTreeMap::new();
    for event in &spec.prediction.long {
        sides.insert(event.code, 1_i8);
    }
    for event in &spec.prediction.short {
        sides.insert(event.code, -1_i8);
    }
    let direction_events: BTreeSet<u16> = spec
        .prediction
        .events
        .iter()
        .map(|event| event.code)
        .collect();
    let mut long = vec![0_u16; timeframe.len()];
    let mut short = vec![0_u16; timeframe.len()];
    let mut seen = BTreeSet::new();
    for event in events {
        let key = event.event_code;
        let contribution = match spec.prediction.kind {
            PredictionKind::Vote => sides.get(&key).copied(),
            PredictionKind::Direction => direction_events
                .contains(&key)
                .then_some(event.direction_code),
        };
        let Some(side) = contribution else {
            continue;
        };
        let bar = event.available_bar_index;
        if bar >= timeframe.len() || !seen.insert((bar, key)) {
            continue;
        }
        if side > 0 {
            long[bar] = long[bar].saturating_add(1);
        } else if side < 0 {
            short[bar] = short[bar].saturating_add(1);
        }
    }
    let mut values = Vec::with_capacity(timeframe.len());
    for bar in 0..timeframe.len() {
        let total = i32::from(long[bar]) - i32::from(short[bar]);
        let value = if total.unsigned_abs() as usize >= spec.prediction.min_votes {
            total.signum() as i8
        } else {
            0
        };
        values.push(value);
    }
    PredictionSeries {
        values,
        long_votes: long,
        short_votes: short,
    }
}

fn extended_is_valid(config: crate::readings::ExtendedConfig) -> bool {
    config.vwap_window > 0
        && config.vwap_band_mad > 0.0
        && config.profile_window > 0
        && config.profile_bins > 0
        && (0.0..=1.0).contains(&config.profile_value_area)
        && config.rvol_fast > 0
        && config.rvol_slow > 0
        && config.volume_oscillator > 0
        && config.climax_volume > 0.0
        && config.climax_range_atr > 0.0
        && config.absorption_volume > 0.0
        && config.absorption_range_atr > 0.0
        && config.absorption_level_atr > 0.0
        && config.flow_window > 0
        && config.mfi_window > 0
        && config.cmf_threshold > 0.0
        && config.atr_slow_window > 0
        && config.atr_slope_window > 0
        && config.range_window > 0
        && config.mar_fast > 0
        && config.mar_slow > 0
        && config.keltner_window > 0
        && config.keltner_atr > 0.0
        && config.donchian_window > 0
        && config.level_lookback > 0
        && config.level_atr > 0.0
        && config.aroon_window > 0
        && config.path_window > 0
        && config.candle_body > 0.0
        && config.candle_wick > 0.0
        && config.candle_close > 0.0
        && config.gap_atr > 0.0
}

#[cfg(test)]
mod edge_tests;
#[cfg(test)]
mod tests;
