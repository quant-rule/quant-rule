//! The resolved `quantrule.model-spec/v1` contract and its bounded evaluator.
//!
//! A model spec declares a timeframe ladder, one evaluation timeframe, and a
//! set of named conditions. A condition is a boolean expression over the
//! published event records and discrete states of the ladder; it is evaluated
//! as of each evaluation bar, with a higher-timeframe bar selected only after
//! its close is at or before the evaluation bar's close.
//!
//! The module is pure: it reads a completed [`ModelRun`] and a resolved spec,
//! performs no I/O, reads no clock, and uses no randomness. The JSON document
//! is parsed at the shell boundary; the model receives numeric codes, so the
//! evaluator does not depend on a JSON crate or on the registry file.

use std::collections::{BTreeMap, BTreeSet};

use crate::events::EventType;
use crate::run::ModelRun;
use crate::timeframe::{ModelStates, TimeframeModel};

pub mod v2;

/// The only schema version this module evaluates.
pub const SPEC_SCHEMA: &str = "quantrule.model-spec/v1";
/// The maximum expression depth, counting the root as one.
pub const MAX_EXPRESSION_DEPTH: usize = 16;
/// The maximum expression nodes in one condition.
pub const MAX_EXPRESSION_NODES: usize = 256;
/// The maximum number of conditions in one spec.
pub const MAX_CONDITIONS: usize = 64;
/// The maximum event lookback window, in referenced-timeframe bars.
pub const MAX_WITHIN_BARS: usize = 10_000;

/// A resolved model spec.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelSpec {
    /// The schema version; it must equal [`SPEC_SCHEMA`].
    pub schema: String,
    /// The spec's stable identifier.
    pub id: String,
    /// The timeframe ladder in seconds, strictly ascending.
    pub ladder: Vec<u64>,
    /// The timeframe whose bars are evaluated.
    pub evaluation_timeframe_secs: u64,
    /// The named conditions, in document order.
    pub conditions: Vec<SpecCondition>,
}

/// One named condition.
#[derive(Debug, Clone, PartialEq)]
pub struct SpecCondition {
    /// The condition's stable identifier, unique within the spec.
    pub id: String,
    /// The boolean expression.
    pub expression: SpecExpression,
}

/// A bounded boolean expression over events and states.
#[derive(Debug, Clone, PartialEq)]
pub enum SpecExpression {
    /// True when a matching event is available in the window.
    Event(SpecEvent),
    /// True when a discrete state equals the expected code.
    State(SpecState),
    /// True when every child is true.
    All(Vec<SpecExpression>),
    /// True when at least one child is true.
    Any(Vec<SpecExpression>),
    /// True when the child is false.
    Not(Box<SpecExpression>),
}

/// One event predicate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpecEvent {
    /// The stable registry event code.
    pub event_code: u16,
    /// The event type.
    pub event_type: EventType,
    /// The event subtype code.
    pub subtype: u8,
    /// The stable source code.
    pub source: u16,
    /// The timeframe whose events are read.
    pub timeframe_secs: u64,
    /// The required direction, or `None` for either direction.
    pub direction: Option<i8>,
    /// The lookback window in referenced-timeframe bars.
    pub within_bars: usize,
}

/// One discrete-state predicate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpecState {
    /// The state field.
    pub state: SpecStateField,
    /// The timeframe whose state is read.
    pub timeframe_secs: u64,
    /// The expected code.
    pub equals: i8,
}

/// The discrete states a spec may read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecStateField {
    /// The EMA stack state.
    EmaStack,
    /// The RSI zone state.
    RsiZone,
    /// The MACD line/signal state.
    MacdState,
    /// The MACD histogram state.
    MacdHistogramState,
    /// The ADX regime state.
    AdxRegime,
    /// The ADX slope sign.
    AdxSlope,
    /// The DI state.
    DiState,
    /// The relative-volume regime.
    RvolRegime,
    /// The volatility regime.
    VolRegime,
    /// The confirmed major structure state.
    StructureState,
}

impl SpecStateField {
    /// Every state field, in a fixed order.
    pub const ALL: [Self; 10] = [
        Self::EmaStack,
        Self::RsiZone,
        Self::MacdState,
        Self::MacdHistogramState,
        Self::AdxRegime,
        Self::AdxSlope,
        Self::DiState,
        Self::RvolRegime,
        Self::VolRegime,
        Self::StructureState,
    ];

    /// The state value at one model bar, or `None` when it is unavailable.
    #[must_use]
    pub fn value(self, states: &ModelStates) -> Option<i8> {
        match self {
            Self::EmaStack => states.ema_stack,
            Self::RsiZone => states.rsi_zone,
            Self::MacdState => states.macd_state,
            Self::MacdHistogramState => states.macd_histogram_state,
            Self::AdxRegime => states.adx_regime,
            Self::AdxSlope => states.adx_slope,
            Self::DiState => states.di_state,
            Self::RvolRegime => states.rvol_regime,
            Self::VolRegime => states.vol_regime,
            Self::StructureState => states.structure_state,
        }
    }

    /// The valid code range for the state.
    #[must_use]
    pub const fn bounds(self) -> (i8, i8) {
        match self {
            Self::EmaStack => (-1, 1),
            Self::RsiZone => (-1, 1),
            Self::MacdState => (-1, 1),
            Self::MacdHistogramState => (-1, 1),
            Self::AdxRegime => (0, 1),
            Self::AdxSlope => (-1, 1),
            Self::DiState => (-1, 1),
            Self::RvolRegime => (-1, 1),
            Self::VolRegime => (0, 2),
            Self::StructureState => (-1, 1),
        }
    }
}

/// A spec that failed validation or could not be evaluated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpecError {
    /// The schema version is not [`SPEC_SCHEMA`].
    UnsupportedSchema,
    /// The spec identifier is empty.
    EmptyId,
    /// The ladder is empty.
    EmptyLadder,
    /// A timeframe is zero.
    ZeroTimeframe,
    /// The ladder is not strictly ascending.
    LadderNotAscending {
        /// The previous timeframe.
        previous: u64,
        /// The current timeframe.
        current: u64,
    },
    /// The evaluation timeframe is not in the ladder.
    UnknownEvaluationTimeframe(u64),
    /// The spec has no conditions.
    EmptyConditions,
    /// The spec has too many conditions.
    TooManyConditions(usize),
    /// A condition identifier is empty.
    EmptyConditionId,
    /// Two conditions share an identifier.
    DuplicateConditionId(String),
    /// An `all` or `any` has no children.
    EmptyCombinator,
    /// The expression is deeper than [`MAX_EXPRESSION_DEPTH`].
    ExpressionTooDeep,
    /// The expression has more than [`MAX_EXPRESSION_NODES`] nodes.
    ExpressionTooLarge,
    /// A condition references a timeframe outside the ladder.
    UnknownTimeframe(u64),
    /// The event code is zero.
    ZeroEventCode,
    /// The event subtype is zero.
    ZeroEventSubtype,
    /// The event source is zero.
    ZeroEventSource,
    /// The direction is outside -1..=1.
    InvalidDirection(i8),
    /// The lookback window is larger than [`MAX_WITHIN_BARS`].
    WithinBarsTooLarge(usize),
    /// The expected state code is outside the state's bounds.
    InvalidStateCode {
        /// The state field.
        state: SpecStateField,
        /// The expected code.
        equals: i8,
    },
    /// A referenced timeframe is absent from the run.
    MissingTimeframe(u64),
}

impl std::fmt::Display for SpecError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedSchema => write!(formatter, "the schema is not {SPEC_SCHEMA}"),
            Self::EmptyId => write!(formatter, "the spec id is empty"),
            Self::EmptyLadder => write!(formatter, "the ladder is empty"),
            Self::ZeroTimeframe => write!(formatter, "a timeframe is zero"),
            Self::LadderNotAscending { previous, current } => {
                write!(
                    formatter,
                    "the ladder is not ascending: {previous} then {current}"
                )
            }
            Self::UnknownEvaluationTimeframe(timeframe) => {
                write!(
                    formatter,
                    "the evaluation timeframe {timeframe} is not in the ladder"
                )
            }
            Self::EmptyConditions => write!(formatter, "the spec has no conditions"),
            Self::TooManyConditions(count) => {
                write!(
                    formatter,
                    "the spec has {count} conditions, over the {MAX_CONDITIONS} limit"
                )
            }
            Self::EmptyConditionId => write!(formatter, "a condition id is empty"),
            Self::DuplicateConditionId(id) => {
                write!(formatter, "the condition id {id} is duplicated")
            }
            Self::EmptyCombinator => write!(formatter, "an all/any expression has no children"),
            Self::ExpressionTooDeep => {
                write!(
                    formatter,
                    "the expression is deeper than {MAX_EXPRESSION_DEPTH}"
                )
            }
            Self::ExpressionTooLarge => {
                write!(
                    formatter,
                    "the expression has more than {MAX_EXPRESSION_NODES} nodes"
                )
            }
            Self::UnknownTimeframe(timeframe) => {
                write!(
                    formatter,
                    "the condition timeframe {timeframe} is not in the ladder"
                )
            }
            Self::ZeroEventCode => write!(formatter, "the event code is zero"),
            Self::ZeroEventSubtype => write!(formatter, "the event subtype is zero"),
            Self::ZeroEventSource => write!(formatter, "the event source is zero"),
            Self::InvalidDirection(direction) => {
                write!(formatter, "the direction {direction} is outside -1..=1")
            }
            Self::WithinBarsTooLarge(bars) => {
                write!(
                    formatter,
                    "the lookback {bars} is over the {MAX_WITHIN_BARS} limit"
                )
            }
            Self::InvalidStateCode { state, equals } => {
                write!(formatter, "the code {equals} is outside {state:?}'s bounds")
            }
            Self::MissingTimeframe(timeframe) => {
                write!(formatter, "the run does not contain timeframe {timeframe}")
            }
        }
    }
}

impl std::error::Error for SpecError {}

impl ModelSpec {
    /// Validates the spec without evaluating it.
    ///
    /// # Errors
    ///
    /// Returns a [`SpecError`] when the schema, ladder, identifiers, or
    /// expression bounds are invalid.
    pub fn validate(&self) -> Result<(), SpecError> {
        if self.schema != SPEC_SCHEMA {
            return Err(SpecError::UnsupportedSchema);
        }
        if self.id.is_empty() {
            return Err(SpecError::EmptyId);
        }
        if self.ladder.is_empty() {
            return Err(SpecError::EmptyLadder);
        }
        let mut previous = None;
        for timeframe in &self.ladder {
            if *timeframe == 0 {
                return Err(SpecError::ZeroTimeframe);
            }
            if let Some(previous) = previous
                && *timeframe <= previous
            {
                return Err(SpecError::LadderNotAscending {
                    previous,
                    current: *timeframe,
                });
            }
            previous = Some(*timeframe);
        }
        if !self.ladder.contains(&self.evaluation_timeframe_secs) {
            return Err(SpecError::UnknownEvaluationTimeframe(
                self.evaluation_timeframe_secs,
            ));
        }
        if self.conditions.is_empty() {
            return Err(SpecError::EmptyConditions);
        }
        if self.conditions.len() > MAX_CONDITIONS {
            return Err(SpecError::TooManyConditions(self.conditions.len()));
        }
        let ladder: BTreeSet<u64> = self.ladder.iter().copied().collect();
        let mut identifiers = BTreeSet::new();
        for condition in &self.conditions {
            if condition.id.is_empty() {
                return Err(SpecError::EmptyConditionId);
            }
            if !identifiers.insert(condition.id.clone()) {
                return Err(SpecError::DuplicateConditionId(condition.id.clone()));
            }
            let mut nodes = 0;
            validate_expression(&condition.expression, &ladder, 1, &mut nodes)?;
        }
        Ok(())
    }

    /// Evaluates every condition over a completed model run.
    ///
    /// # Errors
    ///
    /// Returns a [`SpecError`] when the spec is invalid or a referenced
    /// timeframe is absent from the run.
    pub fn evaluate(&self, run: &ModelRun) -> Result<SpecEvaluation, SpecError> {
        self.validate()?;
        let evaluation = run
            .timeframe(self.evaluation_timeframe_secs)
            .ok_or(SpecError::MissingTimeframe(self.evaluation_timeframe_secs))?;
        for condition in &self.conditions {
            collect_timeframes(&condition.expression, &mut |timeframe| {
                if run.timeframe(timeframe).is_none() {
                    Err(SpecError::MissingTimeframe(timeframe))
                } else {
                    Ok(())
                }
            })?;
        }
        let mut referenced = BTreeSet::new();
        for condition in &self.conditions {
            collect_timeframes(&condition.expression, &mut |timeframe| {
                referenced.insert(timeframe);
                Ok(())
            })?;
        }
        let as_of = AsOf::build(run, evaluation, self.evaluation_timeframe_secs, &referenced);
        let predicates = EventPredicates::build(run, &self.conditions);
        let mut conditions = Vec::with_capacity(self.conditions.len());
        for condition in &self.conditions {
            let mut values = Vec::with_capacity(evaluation.len());
            for bar in 0..evaluation.len() {
                let context = EvalContext {
                    bar,
                    as_of: &as_of,
                    predicates: &predicates,
                    run,
                };
                values.push(evaluate_expression(&condition.expression, &context));
            }
            let true_count = values.iter().filter(|value| **value).count();
            conditions.push(ConditionResult {
                id: condition.id.clone(),
                values,
                true_count,
            });
        }
        Ok(SpecEvaluation {
            evaluation_timeframe_secs: self.evaluation_timeframe_secs,
            bars: evaluation.len(),
            conditions,
        })
    }
}

/// The result of evaluating one spec.
#[derive(Debug, Clone, PartialEq)]
pub struct SpecEvaluation {
    /// The evaluation timeframe in seconds.
    pub evaluation_timeframe_secs: u64,
    /// The evaluation bar count.
    pub bars: usize,
    /// One result per condition, in document order.
    pub conditions: Vec<ConditionResult>,
}

/// One condition's per-bar values.
#[derive(Debug, Clone, PartialEq)]
pub struct ConditionResult {
    /// The condition identifier.
    pub id: String,
    /// The value at each evaluation bar.
    pub values: Vec<bool>,
    /// The number of true bars.
    pub true_count: usize,
}

fn validate_expression(
    expression: &SpecExpression,
    ladder: &BTreeSet<u64>,
    depth: usize,
    nodes: &mut usize,
) -> Result<(), SpecError> {
    if depth > MAX_EXPRESSION_DEPTH {
        return Err(SpecError::ExpressionTooDeep);
    }
    *nodes += 1;
    if *nodes > MAX_EXPRESSION_NODES {
        return Err(SpecError::ExpressionTooLarge);
    }
    match expression {
        SpecExpression::Event(event) => {
            if event.event_code == 0 {
                return Err(SpecError::ZeroEventCode);
            }
            if event.subtype == 0 {
                return Err(SpecError::ZeroEventSubtype);
            }
            if event.source == 0 {
                return Err(SpecError::ZeroEventSource);
            }
            if event
                .direction
                .is_some_and(|direction| !(-1..=1).contains(&direction))
            {
                return Err(SpecError::InvalidDirection(
                    event.direction.unwrap_or_default(),
                ));
            }
            if event.within_bars > MAX_WITHIN_BARS {
                return Err(SpecError::WithinBarsTooLarge(event.within_bars));
            }
            if !ladder.contains(&event.timeframe_secs) {
                return Err(SpecError::UnknownTimeframe(event.timeframe_secs));
            }
        }
        SpecExpression::State(state) => {
            let (low, high) = state.state.bounds();
            if !(low..=high).contains(&state.equals) {
                return Err(SpecError::InvalidStateCode {
                    state: state.state,
                    equals: state.equals,
                });
            }
            if !ladder.contains(&state.timeframe_secs) {
                return Err(SpecError::UnknownTimeframe(state.timeframe_secs));
            }
        }
        SpecExpression::All(children) | SpecExpression::Any(children) => {
            if children.is_empty() {
                return Err(SpecError::EmptyCombinator);
            }
            for child in children {
                validate_expression(child, ladder, depth + 1, nodes)?;
            }
        }
        SpecExpression::Not(child) => {
            validate_expression(child, ladder, depth + 1, nodes)?;
        }
    }
    Ok(())
}

fn collect_timeframes(
    expression: &SpecExpression,
    visit: &mut impl FnMut(u64) -> Result<(), SpecError>,
) -> Result<(), SpecError> {
    match expression {
        SpecExpression::Event(event) => visit(event.timeframe_secs),
        SpecExpression::State(state) => visit(state.timeframe_secs),
        SpecExpression::All(children) | SpecExpression::Any(children) => {
            for child in children {
                collect_timeframes(child, visit)?;
            }
            Ok(())
        }
        SpecExpression::Not(child) => collect_timeframes(child, visit),
    }
}

/// The as-of bar for each referenced timeframe at each evaluation bar.
struct AsOf {
    indices: BTreeMap<u64, Vec<Option<usize>>>,
}

impl AsOf {
    fn build(
        run: &ModelRun,
        evaluation: &TimeframeModel,
        evaluation_secs: u64,
        referenced: &BTreeSet<u64>,
    ) -> Self {
        let mut indices = BTreeMap::new();
        for timeframe in referenced {
            if let Some(model) = run.timeframe(*timeframe) {
                indices.insert(
                    *timeframe,
                    as_of_indices(model, evaluation, evaluation_secs, *timeframe),
                );
            }
        }
        Self { indices }
    }

    fn index(&self, timeframe: u64, bar: usize) -> Option<usize> {
        self.indices
            .get(&timeframe)
            .and_then(|series| series.get(bar).copied().flatten())
    }
}

fn as_of_indices(
    reference: &TimeframeModel,
    evaluation: &TimeframeModel,
    evaluation_secs: u64,
    reference_secs: u64,
) -> Vec<Option<usize>> {
    let mut indices = vec![None; evaluation.len()];
    if evaluation.timeframe_secs() == reference.timeframe_secs() {
        for (bar, index) in indices.iter_mut().enumerate() {
            *index = Some(bar);
        }
        return indices;
    }
    let reference_secs = i64::try_from(reference_secs).unwrap_or(i64::MAX);
    let mut next = 0;
    for (bar, close_time) in evaluation
        .open_times()
        .iter()
        .map(|open| open.checked_add(i64::try_from(evaluation_secs).unwrap_or(i64::MAX)))
        .enumerate()
    {
        let Some(close_time) = close_time else {
            continue;
        };
        while next < reference.open_times().len() {
            let Some(reference_close) = reference.open_times()[next].checked_add(reference_secs)
            else {
                break;
            };
            if reference_close > close_time {
                break;
            }
            next += 1;
        }
        if next > 0 {
            indices[bar] = Some(next - 1);
        }
    }
    indices
}

type PredicateKey = (u64, u16, Option<i8>);

/// The available bars for every event predicate in the spec.
struct EventPredicates {
    available: BTreeMap<PredicateKey, Vec<usize>>,
}

impl EventPredicates {
    fn build(run: &ModelRun, conditions: &[SpecCondition]) -> Self {
        let mut keys = BTreeSet::new();
        for condition in conditions {
            collect_predicates(&condition.expression, &mut keys);
        }
        let mut available = BTreeMap::new();
        for key in &keys {
            let Some(timeframe) = run.timeframe(key.0) else {
                continue;
            };
            let mut bars: Vec<usize> = timeframe
                .events()
                .iter()
                .chain(timeframe.extended_events().events())
                .filter(|event| {
                    event.event_code == key.1
                        && key
                            .2
                            .is_none_or(|direction| event.direction_code == direction)
                })
                .map(|event| event.available_bar_index)
                .collect();
            // The two batches are each sorted; merging them restores one
            // ascending list for the window search.
            bars.sort_unstable();
            available.insert(*key, bars);
        }
        Self { available }
    }

    fn contains(&self, key: &PredicateKey, as_of: usize, within: usize) -> bool {
        let Some(bars) = self.available.get(key) else {
            return false;
        };
        let start = bars.partition_point(|bar| *bar < as_of.saturating_sub(within));
        let end = bars.partition_point(|bar| *bar <= as_of);
        end > start
    }
}

fn collect_predicates(expression: &SpecExpression, keys: &mut BTreeSet<PredicateKey>) {
    match expression {
        SpecExpression::Event(event) => {
            keys.insert((event.timeframe_secs, event.event_code, event.direction));
        }
        SpecExpression::State(_) => {}
        SpecExpression::All(children) | SpecExpression::Any(children) => {
            for child in children {
                collect_predicates(child, keys);
            }
        }
        SpecExpression::Not(child) => collect_predicates(child, keys),
    }
}

struct EvalContext<'a> {
    bar: usize,
    as_of: &'a AsOf,
    predicates: &'a EventPredicates,
    run: &'a ModelRun,
}

fn evaluate_expression(expression: &SpecExpression, context: &EvalContext<'_>) -> bool {
    match expression {
        SpecExpression::Event(event) => {
            let Some(as_of) = context.as_of.index(event.timeframe_secs, context.bar) else {
                return false;
            };
            let key = (event.timeframe_secs, event.event_code, event.direction);
            context.predicates.contains(&key, as_of, event.within_bars)
        }
        SpecExpression::State(state) => {
            let Some(as_of) = context.as_of.index(state.timeframe_secs, context.bar) else {
                return false;
            };
            let Some(timeframe) = context.run.timeframe(state.timeframe_secs) else {
                return false;
            };
            timeframe
                .states_at(as_of)
                .and_then(|states| state.state.value(&states))
                .is_some_and(|value| value == state.equals)
        }
        SpecExpression::All(children) => children
            .iter()
            .all(|child| evaluate_expression(child, context)),
        SpecExpression::Any(children) => children
            .iter()
            .any(|child| evaluate_expression(child, context)),
        SpecExpression::Not(child) => !evaluate_expression(child, context),
    }
}

#[cfg(test)]
mod tests;
