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

type EventIndex = BTreeMap<(u8, u8, u16), Vec<(usize, i8)>>;

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
    /// Two active events share a numeric code.
    DuplicateActiveEventCode(u8, u8, u16),
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
            Self::DuplicateActiveEventCode(event_type, subtype, source) => write!(
                formatter,
                "two active events share type {event_type}, subtype {subtype}, source {source}"
            ),
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
        let mut codes = BTreeSet::new();
        for event in &self.active_events {
            if !identifiers.insert(event.id.as_str()) {
                return Err(SpecV2Error::DuplicateActiveEvent(event.id.clone()));
            }
            if !codes.insert((event.event_type.code(), event.subtype, event.source)) {
                return Err(SpecV2Error::DuplicateActiveEventCode(
                    event.event_type.code(),
                    event.subtype,
                    event.source,
                ));
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
        let session_events = match spec.session {
            Some(config) => {
                let sessions =
                    SessionReadings::compute(timeframe.candles(), timeframe.readings(), &config);
                SessionEventSeries::compute(
                    timeframe.candles(),
                    timeframe.readings(),
                    &sessions,
                    &config,
                    spec.evaluation_timeframe_secs,
                )
                .events()
                .to_vec()
            }
            None => Vec::new(),
        };
        let panel_events = if spec.panel_instruments.is_empty() {
            Vec::new()
        } else {
            let readings = PanelReadings::compute(panel, &PanelConfig::DEFAULT);
            let series =
                PanelEventSeries::compute(panel, &readings, spec.evaluation_timeframe_secs);
            series.events(&spec.instrument).to_vec()
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
            .entry((
                event.event_type.code(),
                event.event_subtype_code,
                event.source_code,
            ))
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
            let key = (event.event_type.code(), event.subtype, event.source);
            let Some(entries) = index.get(&key) else {
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
        sides.insert((event.event_type.code(), event.subtype, event.source), 1_i8);
    }
    for event in &spec.prediction.short {
        sides.insert(
            (event.event_type.code(), event.subtype, event.source),
            -1_i8,
        );
    }
    let direction_events: BTreeSet<(u8, u8, u16)> = spec
        .prediction
        .events
        .iter()
        .map(|event| (event.event_type.code(), event.subtype, event.source))
        .collect();
    let mut long = vec![0_u16; timeframe.len()];
    let mut short = vec![0_u16; timeframe.len()];
    let mut seen = BTreeSet::new();
    for event in events {
        let key = (
            event.event_type.code(),
            event.event_subtype_code,
            event.source_code,
        );
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
        if bar >= timeframe.len() || !seen.insert((bar, key.0, key.1, key.2)) {
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
mod tests {
    use super::{
        ModelSpecV2, PredictionKind, SPEC_V2_SCHEMA, SpecEventCode, SpecEventRole, SpecPrediction,
        SpecRun, SpecV2Error,
    };
    use crate::events::EventType;
    use crate::input::{MarketStateConfig, TimeframeInput};
    use crate::panel::{PanelConfig, PanelInstrument, PanelReadings};
    use crate::run::ModelRun;
    use crate::spec::{SpecCondition, SpecEvent, SpecExpression};
    use quantrule_data::Candle;

    const BASE: i64 = 1_700_000_100;

    fn candle(index: usize, timestamp: i64, open: f64, close: f64) -> Candle {
        Candle::new(
            index as u64,
            timestamp,
            open,
            open.max(close) + 0.4,
            open.min(close) - 0.4,
            close,
            Some(1_000.0 + index as f64),
        )
        .expect("the generated candle is valid")
    }

    fn candles(count: usize) -> Vec<Candle> {
        (0..count)
            .map(|index| {
                let phase = index as f64;
                let open = 100.0 + (phase / 7.0).sin() * 3.0 + 0.02 * phase;
                let close = 100.0 + ((phase + 1.0) / 7.0).sin() * 3.0 + 0.02 * (phase + 1.0);
                candle(index, BASE + index as i64 * 300, open, close)
            })
            .collect()
    }

    fn event(id: &str, role: SpecEventRole, subtype: u8, source: u16) -> SpecEventCode {
        SpecEventCode {
            id: id.to_string(),
            role,
            event_type: EventType::Signal,
            subtype,
            source,
        }
    }

    fn spec() -> ModelSpecV2 {
        let long = event("signal.rsi_threshold", SpecEventRole::Signal, 1, 4);
        let short = event("signal.ema_pair_cross", SpecEventRole::Signal, 2, 1);
        ModelSpecV2 {
            schema: SPEC_V2_SCHEMA.to_string(),
            id: "M2.test".to_string(),
            family: "M2".to_string(),
            instrument: "XAU".to_string(),
            timeframes: vec![300],
            evaluation_timeframe_secs: 300,
            readings: MarketStateConfig::default(),
            active_events: vec![long.clone(), short.clone()],
            prediction: SpecPrediction {
                kind: PredictionKind::Vote,
                long: vec![long],
                short: vec![short],
                events: Vec::new(),
                min_votes: 1,
            },
            conditions: Vec::new(),
            session: None,
            panel_instruments: Vec::new(),
        }
    }

    fn run(candles: &[Candle]) -> ModelRun {
        run_with(candles, &MarketStateConfig::default())
    }

    fn run_with(candles: &[Candle], config: &MarketStateConfig) -> ModelRun {
        ModelRun::compute(
            &[TimeframeInput {
                timeframe_secs: 300,
                candles,
            }],
            config,
        )
    }

    #[test]
    fn a_v2_spec_predicts_and_rejects_a_prefix_change() {
        let candles = candles(400);
        let full = SpecRun::compute(&spec(), run(&candles), &[]).expect("the full spec run");
        assert_eq!(full.bars(), candles.len());
        assert_eq!(full.prediction().len(), candles.len());
        assert!(
            full.prediction()
                .values
                .iter()
                .all(|value| (-1..=1).contains(value))
        );
        assert!(
            full.prediction().values.iter().any(|value| *value != 0),
            "the synthetic series produced no vote"
        );

        let prefix = SpecRun::compute(&spec(), run(&candles[..300]), &[]).expect("the prefix run");
        assert_eq!(
            prefix.prediction().values,
            full.prediction().values[..300],
            "a past prediction changed when later bars were appended"
        );
    }

    #[test]
    fn a_v2_spec_is_deterministic() {
        let candles = candles(400);
        let first = SpecRun::compute(&spec(), run(&candles), &[]).expect("the first run");
        let second = SpecRun::compute(&spec(), run(&candles), &[]).expect("the second run");
        assert_eq!(first, second);
    }

    #[test]
    fn the_minimum_vote_suppresses_a_single_vote() {
        let candles = candles(400);
        let mut spec = spec();
        spec.prediction.min_votes = 2;
        let result = SpecRun::compute(&spec, run(&candles), &[]).expect("the threshold run");
        for bar in 0..result.prediction().len() {
            let total = i32::from(result.prediction().long_votes[bar])
                - i32::from(result.prediction().short_votes[bar]);
            let expected = if total.unsigned_abs() as usize >= 2 {
                total.signum() as i8
            } else {
                0
            };
            assert_eq!(result.prediction().value(bar), Some(expected));
        }
    }

    #[test]
    fn a_reading_override_reaches_the_extended_readings() {
        let candles = candles(60);
        let mut spec = spec();
        let mut readings = spec.readings;
        let mut extended = readings.extended();
        extended.vwap_window = 5;
        readings.set_extended(extended);
        spec.readings = readings;
        let result = SpecRun::compute(&spec, run_with(&candles, &spec.readings), &[])
            .expect("the override run");
        let timeframe = result
            .base()
            .timeframe(300)
            .expect("the evaluation timeframe");
        assert!(
            timeframe.extended().vwap.get(4).is_some(),
            "the five-bar VWAP was not available at bar 4"
        );
        assert!(
            timeframe.extended().vwap.get(3).is_none(),
            "the VWAP was available before its window"
        );
    }

    #[test]
    fn conditions_see_the_session_batch() {
        let candles = candles(400);
        let mut spec = spec();
        spec.session = Some(crate::sessions::SessionConfig::london());
        spec.active_events
            .push(event("signal.session_open", SpecEventRole::Context, 23, 50));
        spec.conditions = vec![SpecCondition {
            id: "session_open".to_string(),
            expression: SpecExpression::Event(SpecEvent {
                event_type: EventType::Signal,
                subtype: 23,
                source: 50,
                timeframe_secs: 300,
                direction: Some(0),
                within_bars: 0,
            }),
        }];
        let result = SpecRun::compute(&spec, run(&candles), &[]).expect("the session run");
        let evaluation = result.conditions().expect("the condition evaluation");
        assert_eq!(evaluation.conditions[0].values.len(), candles.len());
        assert!(
            evaluation.conditions[0].true_count > 0,
            "the synthetic series had no session open"
        );
    }

    #[test]
    fn conditions_see_the_panel_batch() {
        let candles = candles(400);
        let mut other = Vec::with_capacity(candles.len());
        for (index, candle) in candles.iter().enumerate() {
            let close = 50.0 + (index as f64 * 0.11).sin() * 4.0;
            other.push(candle_with_close(index, candle.timestamp(), close));
        }
        let panel = [
            PanelInstrument {
                name: "XAU",
                candles: &candles,
            },
            PanelInstrument {
                name: "BTC",
                candles: &other,
            },
        ];
        let mut spec = spec();
        spec.panel_instruments = vec!["XAU".to_string(), "BTC".to_string()];
        spec.active_events.push(event(
            "signal.relative_strength_cross",
            SpecEventRole::Context,
            22,
            45,
        ));
        spec.conditions = vec![SpecCondition {
            id: "relative_strength".to_string(),
            expression: SpecExpression::Event(SpecEvent {
                event_type: EventType::Signal,
                subtype: 22,
                source: 45,
                timeframe_secs: 300,
                direction: None,
                within_bars: 0,
            }),
        }];
        let result = SpecRun::compute(&spec, run(&candles), &panel).expect("the panel run");
        let expected = crate::events::panel::PanelEventSeries::compute(
            &panel,
            &PanelReadings::compute(&panel, &PanelConfig::DEFAULT),
            300,
        )
        .events("XAU")
        .to_vec();
        assert_eq!(result.panel_events(), expected.as_slice());
        let evaluation = result.conditions().expect("the condition evaluation");
        assert_eq!(evaluation.conditions[0].values.len(), candles.len());
    }

    #[test]
    fn a_direction_prediction_votes_from_event_directions() {
        let candles = candles(400);
        let mut spec = spec();
        spec.prediction = SpecPrediction {
            kind: PredictionKind::Direction,
            long: Vec::new(),
            short: Vec::new(),
            events: vec![
                event("signal.rsi_threshold", SpecEventRole::Signal, 1, 4),
                event("signal.ema_pair_cross", SpecEventRole::Signal, 2, 1),
            ],
            min_votes: 1,
        };
        let result = SpecRun::compute(&spec, run(&candles), &[]).expect("the direction run");
        assert_eq!(result.prediction().len(), candles.len());
        assert!(
            result.prediction().values.iter().any(|value| *value != 0),
            "the synthetic series produced no direction vote"
        );
    }

    #[test]
    fn a_prediction_kind_rejects_the_other_kind_fields() {
        let first = event("signal.rsi_threshold", SpecEventRole::Signal, 1, 4);
        let mut vote_with_events = spec();
        vote_with_events.prediction.events = vec![first.clone()];
        assert!(matches!(
            vote_with_events.validate(),
            Err(SpecV2Error::InvalidPredictionShape(_))
        ));
        let mut direction_with_sides = spec();
        direction_with_sides.prediction.kind = PredictionKind::Direction;
        direction_with_sides.prediction.events = vec![first];
        assert!(matches!(
            direction_with_sides.validate(),
            Err(SpecV2Error::InvalidPredictionShape(_))
        ));
    }

    fn candle_with_close(index: usize, timestamp: i64, close: f64) -> Candle {
        Candle::new(
            index as u64,
            timestamp,
            close,
            close + 0.5,
            close - 0.5,
            close,
            Some(1_000.0 + index as f64),
        )
        .expect("the generated candle is valid")
    }
}

#[cfg(test)]
mod edge_tests {
    use super::{ModelSpecV2, SPEC_V2_SCHEMA, SpecRun, SpecV2Error};
    use crate::input::{MarketStateConfig, TimeframeInput};
    use crate::run::ModelRun;
    use quantrule_data::Candle;

    fn candle(index: usize) -> Candle {
        Candle::new(
            index as u64,
            index as i64 * 300,
            100.0,
            101.0,
            99.0,
            100.5,
            Some(1_000.0),
        )
        .expect("the edge candle is valid")
    }

    fn spec() -> ModelSpecV2 {
        let event = super::SpecEventCode {
            id: "signal.rsi_threshold".to_string(),
            role: super::SpecEventRole::Signal,
            event_type: crate::events::EventType::Signal,
            subtype: 1,
            source: 4,
        };
        ModelSpecV2 {
            schema: SPEC_V2_SCHEMA.to_string(),
            id: "M2.edge".to_string(),
            family: "M2".to_string(),
            instrument: "XAU".to_string(),
            timeframes: vec![300],
            evaluation_timeframe_secs: 300,
            readings: MarketStateConfig::default(),
            active_events: vec![event.clone()],
            prediction: super::SpecPrediction {
                kind: super::PredictionKind::Vote,
                long: vec![event],
                short: Vec::new(),
                events: Vec::new(),
                min_votes: 1,
            },
            conditions: Vec::new(),
            session: None,
            panel_instruments: Vec::new(),
        }
    }

    #[test]
    fn empty_single_and_short_series_are_safe() {
        let empty = ModelRun::compute(&[], &MarketStateConfig::default());
        assert!(matches!(
            SpecRun::compute(&spec(), empty, &[]),
            Err(SpecV2Error::MissingTimeframe(300))
        ));
        for count in [1_usize, 5, 20] {
            let candles: Vec<Candle> = (0..count).map(candle).collect();
            let run = ModelRun::compute(
                &[TimeframeInput {
                    timeframe_secs: 300,
                    candles: &candles,
                }],
                &MarketStateConfig::default(),
            );
            let result = SpecRun::compute(&spec(), run, &[]).expect("the short series runs");
            assert_eq!(result.bars(), count);
            assert_eq!(result.prediction().len(), count);
            assert!(
                result.prediction().values.iter().all(|value| *value == 0),
                "a short series produced a vote"
            );
        }
    }
}
