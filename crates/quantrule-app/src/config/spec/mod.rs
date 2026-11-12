//! The `quantrule.model-spec` JSON parsers.
//!
//! The application service owns JSON parsing; the model owns the resolved spec
//! and its evaluators. The v1 parser and the v2 parser share one embedded registry so
//! an event identifier resolves to the same numeric codes in both schemas.
//!
//! The registry is embedded at compile time so the parser and the model cannot
//! drift apart without a rebuild, and a spec run records the registry version
//! with its provenance.

use std::collections::BTreeMap;
use std::fmt;

use quantrule_model::events::EventType;
use quantrule_model::spec::v2::SpecEventRole;
use quantrule_model::spec::{
    ModelSpec, SPEC_SCHEMA, SpecCondition, SpecEvent, SpecExpression, SpecState, SpecStateField,
};
use serde_json::{Map, Value};

pub mod v2;

const REGISTRY_JSON: &str = include_str!("../../../../../docs/registry/indicators.json");

/// A spec document that could not be parsed or validated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecParseError {
    message: String,
}

impl SpecParseError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for SpecParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for SpecParseError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EventCode {
    pub(crate) code: u16,
    pub(crate) event_type: EventType,
    pub(crate) subtype: u8,
    pub(crate) source: u16,
    pub(crate) role: SpecEventRole,
}

/// Reads the schema field without validating the rest of the document.
///
/// The report command uses this to dispatch to the v1 or v2 parser with one
/// error shape. A malformed document is rejected here.
///
/// # Errors
///
/// Returns a [`SpecParseError`] when the JSON is malformed, the root is not an
/// object, or the schema field is missing or not a string.
pub fn schema_of(text: &str) -> Result<String, SpecParseError> {
    let value: Value = serde_json::from_str(text)
        .map_err(|error| SpecParseError::new(format!("invalid JSON: {error}")))?;
    let object = as_object(&value, "the spec")?;
    Ok(as_str(field(object, "schema", "the spec")?, "the spec schema")?.to_string())
}

/// Parses and validates a `quantrule.model-spec/v1` document.
///
/// # Errors
///
/// Returns a [`SpecParseError`] when the JSON is malformed, a required field is
/// missing or mistyped, an event identifier is unknown, or the resolved spec
/// fails the model's validation.
pub fn parse_model_spec(text: &str) -> Result<ModelSpec, SpecParseError> {
    let value: Value = serde_json::from_str(text)
        .map_err(|error| SpecParseError::new(format!("invalid JSON: {error}")))?;
    let registry = Registry::load()?;
    let object = as_object(&value, "the spec")?;
    allow_only(
        object,
        &[
            "schema",
            "id",
            "ladder",
            "evaluation_timeframe",
            "conditions",
        ],
        "the spec",
    )?;

    let schema = as_str(field(object, "schema", "the spec")?, "schema")?;
    if schema != SPEC_SCHEMA {
        return Err(SpecParseError::new(format!(
            "the schema {schema} is not {SPEC_SCHEMA}"
        )));
    }
    let id = as_str(field(object, "id", "the spec")?, "id")?.to_string();
    let ladder_value = field(object, "ladder", "the spec")?;
    let ladder = as_array(ladder_value, "ladder")?
        .iter()
        .map(|value| as_u64(value, "a ladder entry"))
        .collect::<Result<Vec<u64>, _>>()?;
    if ladder.is_empty() {
        return Err(SpecParseError::new("the ladder is empty"));
    }
    let evaluation_timeframe_secs = match object.get("evaluation_timeframe") {
        Some(value) => as_u64(value, "evaluation_timeframe")?,
        None => ladder[0],
    };
    let conditions = as_array(field(object, "conditions", "the spec")?, "conditions")?
        .iter()
        .map(|value| parse_condition(value, &registry))
        .collect::<Result<Vec<SpecCondition>, _>>()?;

    let spec = ModelSpec {
        schema: schema.to_string(),
        id,
        ladder,
        evaluation_timeframe_secs,
        conditions,
    };
    spec.validate()
        .map_err(|error| SpecParseError::new(format!("the spec is invalid: {error}")))?;
    Ok(spec)
}

pub(crate) fn parse_condition(
    value: &Value,
    registry: &Registry,
) -> Result<SpecCondition, SpecParseError> {
    let object = as_object(value, "a condition")?;
    allow_only(object, &["id", "when"], "a condition")?;
    let id = as_str(field(object, "id", "a condition")?, "a condition id")?.to_string();
    let expression = parse_expression(field(object, "when", "a condition")?, registry)?;
    Ok(SpecCondition { id, expression })
}

fn parse_expression(value: &Value, registry: &Registry) -> Result<SpecExpression, SpecParseError> {
    let object = as_object(value, "an expression")?;
    let keys: Vec<&str> = object.keys().map(String::as_str).collect();
    if keys.len() != 1 {
        return Err(SpecParseError::new(
            "an expression must have exactly one of all, any, not, event, state",
        ));
    }
    let key = keys[0];
    let operand = &object[key];
    match key {
        "all" => Ok(SpecExpression::All(parse_children(operand, registry)?)),
        "any" => Ok(SpecExpression::Any(parse_children(operand, registry)?)),
        "not" => Ok(SpecExpression::Not(Box::new(parse_expression(
            operand, registry,
        )?))),
        "event" => Ok(SpecExpression::Event(parse_event(operand, registry)?)),
        "state" => Ok(SpecExpression::State(parse_state(operand)?)),
        other => Err(SpecParseError::new(format!(
            "the expression key {other} is not all, any, not, event, or state"
        ))),
    }
}

fn parse_children(
    value: &Value,
    registry: &Registry,
) -> Result<Vec<SpecExpression>, SpecParseError> {
    as_array(value, "a combinator")?
        .iter()
        .map(|child| parse_expression(child, registry))
        .collect()
}

fn parse_event(value: &Value, registry: &Registry) -> Result<SpecEvent, SpecParseError> {
    let object = as_object(value, "an event expression")?;
    allow_only(
        object,
        &["id", "timeframe", "direction", "within_bars"],
        "an event expression",
    )?;
    let id = as_str(field(object, "id", "an event expression")?, "an event id")?;
    let code = registry
        .events
        .get(id)
        .ok_or_else(|| SpecParseError::new(format!("the event id {id} is not in the registry")))?;
    let timeframe_secs = as_u64(
        field(object, "timeframe", "an event expression")?,
        "an event timeframe",
    )?;
    let direction = match object.get("direction") {
        Some(value) => {
            let direction = as_i64(value, "an event direction")?;
            let direction = i8::try_from(direction)
                .map_err(|_| SpecParseError::new("an event direction is outside the i8 range"))?;
            Some(direction)
        }
        None => None,
    };
    let within_bars = match object.get("within_bars") {
        Some(value) => {
            let bars = as_u64(value, "within_bars")?;
            usize::try_from(bars)
                .map_err(|_| SpecParseError::new("within_bars is outside the usize range"))?
        }
        None => 0,
    };
    Ok(SpecEvent {
        event_code: code.code,
        event_type: code.event_type,
        subtype: code.subtype,
        source: code.source,
        timeframe_secs,
        direction,
        within_bars,
    })
}

fn parse_state(value: &Value) -> Result<SpecState, SpecParseError> {
    let object = as_object(value, "a state expression")?;
    allow_only(
        object,
        &["state", "timeframe", "equals"],
        "a state expression",
    )?;
    let name = as_str(
        field(object, "state", "a state expression")?,
        "a state name",
    )?;
    let state = state_field(name).ok_or_else(|| {
        SpecParseError::new(format!("the state {name} is not a registered state field"))
    })?;
    let timeframe_secs = as_u64(
        field(object, "timeframe", "a state expression")?,
        "a state timeframe",
    )?;
    let equals = as_i64(
        field(object, "equals", "a state expression")?,
        "a state equals",
    )?;
    let equals = i8::try_from(equals)
        .map_err(|_| SpecParseError::new("a state equals is outside the i8 range"))?;
    Ok(SpecState {
        state,
        timeframe_secs,
        equals,
    })
}

fn state_field(name: &str) -> Option<SpecStateField> {
    match name {
        "ema_stack" => Some(SpecStateField::EmaStack),
        "rsi_zone" => Some(SpecStateField::RsiZone),
        "macd_state" => Some(SpecStateField::MacdState),
        "macd_histogram_state" => Some(SpecStateField::MacdHistogramState),
        "adx_regime" => Some(SpecStateField::AdxRegime),
        "adx_slope" => Some(SpecStateField::AdxSlope),
        "di_state" => Some(SpecStateField::DiState),
        "rvol_regime" => Some(SpecStateField::RvolRegime),
        "vol_regime" => Some(SpecStateField::VolRegime),
        "structure_state" => Some(SpecStateField::StructureState),
        _ => None,
    }
}

pub(crate) struct Registry {
    pub(crate) events: BTreeMap<String, EventCode>,
}

impl Registry {
    pub(crate) fn load() -> Result<Self, SpecParseError> {
        let value: Value = serde_json::from_str(REGISTRY_JSON)
            .map_err(|error| SpecParseError::new(format!("invalid embedded registry: {error}")))?;
        let object = as_object(&value, "the registry")?;
        let events = as_array(
            field(object, "events", "the registry")?,
            "the registry events",
        )?;
        let mut map = BTreeMap::new();
        for event in events {
            let event = as_object(event, "a registry event")?;
            let id = as_str(
                field(event, "id", "a registry event")?,
                "a registry event id",
            )?;
            let code = as_u64(
                field(event, "code", "a registry event")?,
                "a registry event code",
            )?;
            let code = u16::try_from(code).map_err(|_| {
                SpecParseError::new(format!("the registry event {id} has a bad code"))
            })?;
            let event_type = as_u64(
                field(event, "event_type", "a registry event")?,
                "a registry event type",
            )?;
            let event_type = u8::try_from(event_type)
                .ok()
                .and_then(EventType::from_code)
                .ok_or_else(|| {
                    SpecParseError::new(format!("the registry event {id} has an unknown type"))
                })?;
            let subtype = as_u64(
                field(event, "subtype", "a registry event")?,
                "a registry event subtype",
            )?;
            let subtype = u8::try_from(subtype).map_err(|_| {
                SpecParseError::new(format!("the registry event {id} has a bad subtype"))
            })?;
            let source = as_u64(
                field(event, "source", "a registry event")?,
                "a registry event source",
            )?;
            let source = u16::try_from(source).map_err(|_| {
                SpecParseError::new(format!("the registry event {id} has a bad source"))
            })?;
            let role = as_str(
                field(event, "role", "a registry event")?,
                "a registry event role",
            )?;
            let role = match role {
                "signal" => SpecEventRole::Signal,
                "context" => SpecEventRole::Context,
                "diagnostic" => SpecEventRole::Diagnostic,
                other => {
                    return Err(SpecParseError::new(format!(
                        "the registry event {id} has the unknown role {other}"
                    )));
                }
            };
            map.insert(
                id.to_string(),
                EventCode {
                    code,
                    event_type,
                    subtype,
                    source,
                    role,
                },
            );
        }
        Ok(Self { events: map })
    }
}

pub(crate) fn field<'a>(
    object: &'a Map<String, Value>,
    name: &str,
    context: &str,
) -> Result<&'a Value, SpecParseError> {
    object
        .get(name)
        .ok_or_else(|| SpecParseError::new(format!("{context} is missing {name}")))
}

pub(crate) fn as_object<'a>(
    value: &'a Value,
    context: &str,
) -> Result<&'a Map<String, Value>, SpecParseError> {
    value
        .as_object()
        .ok_or_else(|| SpecParseError::new(format!("{context} must be an object")))
}

pub(crate) fn as_array<'a>(
    value: &'a Value,
    context: &str,
) -> Result<&'a Vec<Value>, SpecParseError> {
    value
        .as_array()
        .ok_or_else(|| SpecParseError::new(format!("{context} must be an array")))
}

pub(crate) fn as_str<'a>(value: &'a Value, context: &str) -> Result<&'a str, SpecParseError> {
    value
        .as_str()
        .ok_or_else(|| SpecParseError::new(format!("{context} must be a string")))
}

pub(crate) fn as_u64(value: &Value, context: &str) -> Result<u64, SpecParseError> {
    value
        .as_u64()
        .ok_or_else(|| SpecParseError::new(format!("{context} must be a non-negative integer")))
}

pub(crate) fn as_i64(value: &Value, context: &str) -> Result<i64, SpecParseError> {
    value
        .as_i64()
        .ok_or_else(|| SpecParseError::new(format!("{context} must be an integer")))
}

pub(crate) fn allow_only(
    object: &Map<String, Value>,
    allowed: &[&str],
    context: &str,
) -> Result<(), SpecParseError> {
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(SpecParseError::new(format!(
                "{context} has the unknown field {key}"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_model_spec;
    use quantrule_data::Candle;
    use quantrule_model::events::EventType;
    use quantrule_model::input::{MarketStateConfig, TimeframeInput};
    use quantrule_model::run::ModelRun;
    use quantrule_model::spec::{SpecExpression, SpecStateField};

    const FIXTURE: &str =
        include_str!("../../../../../crates/quantrule-shell/tests/fixtures/model_spec_v1.json");
    const BASE: i64 = 1_700_000_100;

    fn candle(index: usize, timestamp: i64, open: f64, close: f64) -> Candle {
        Candle::new(
            index as u64,
            timestamp,
            open,
            open.max(close) + 0.4,
            open.min(close) - 0.4,
            close,
            Some(1_000.0),
        )
        .expect("the generated candle is valid")
    }

    fn m5_candles(count: usize) -> Vec<Candle> {
        (0..count)
            .map(|index| {
                let phase = index as f64;
                let open = 100.0 + (phase / 7.0).sin() * 3.0 + 0.02 * phase;
                let close = 100.0 + ((phase + 1.0) / 7.0).sin() * 3.0 + 0.02 * (phase + 1.0);
                candle(index, BASE + index as i64 * 300, open, close)
            })
            .collect()
    }

    fn aggregate(candles: &[Candle], factor: usize) -> Vec<Candle> {
        candles
            .chunks_exact(factor)
            .enumerate()
            .map(|(index, chunk)| {
                let open = chunk[0].open();
                let close = chunk[chunk.len() - 1].close();
                let high = chunk
                    .iter()
                    .map(Candle::high)
                    .fold(f64::NEG_INFINITY, f64::max);
                let low = chunk.iter().map(Candle::low).fold(f64::INFINITY, f64::min);
                Candle::new(
                    index as u64,
                    chunk[0].timestamp(),
                    open,
                    high,
                    low,
                    close,
                    Some(1_000.0 * chunk.len() as f64),
                )
                .expect("the aggregated candle is valid")
            })
            .collect()
    }

    fn run(candles: &[Candle]) -> ModelRun {
        let m15 = aggregate(candles, 3);
        ModelRun::compute(
            &[
                TimeframeInput {
                    timeframe_secs: 300,
                    candles,
                },
                TimeframeInput {
                    timeframe_secs: 900,
                    candles: &m15,
                },
            ],
            &MarketStateConfig::default(),
        )
    }

    #[test]
    fn the_context_states_are_registered() {
        assert_eq!(
            super::state_field("vol_regime"),
            Some(SpecStateField::VolRegime)
        );
        assert_eq!(
            super::state_field("structure_state"),
            Some(SpecStateField::StructureState)
        );
    }

    #[test]
    fn a_fixture_parses_validates_and_evaluates() {
        let spec = parse_model_spec(FIXTURE).expect("the fixture parses");
        assert_eq!(spec.id, "fixture-v1");
        assert_eq!(spec.ladder, [300, 900]);
        assert_eq!(spec.evaluation_timeframe_secs, 300);
        assert_eq!(spec.conditions.len(), 3);
        let SpecExpression::All(children) = &spec.conditions[0].expression else {
            panic!("the first condition is not an all");
        };
        let SpecExpression::Event(event) = &children[0] else {
            panic!("the first child is not an event");
        };
        assert_eq!(event.event_type, EventType::Signal);
        assert_eq!((event.subtype, event.source), (1, 4));
        assert_eq!(event.direction, Some(1));
        let SpecExpression::State(state) = &spec.conditions[1].expression else {
            panic!("the second condition is not a state");
        };
        assert_eq!(state.state, SpecStateField::DiState);
        assert_eq!(state.equals, 1);
        let SpecExpression::Event(event) = &spec.conditions[2].expression else {
            panic!("the third condition is not an event");
        };
        assert_eq!((event.subtype, event.source), (16, 30));

        let candles = m5_candles(400);
        let evaluation = spec.evaluate(&run(&candles)).expect("the spec evaluates");
        assert_eq!(evaluation.bars, candles.len());
        assert!(evaluation.conditions[0].true_count > 0);
        assert!(evaluation.conditions[1].true_count > 0);
        assert!(evaluation.conditions[2].true_count > 0);
    }

    #[test]
    fn a_malformed_document_is_rejected() {
        let cases = [
            ("{", "invalid JSON"),
            (
                r#"{"schema":"quantrule.model-spec/v1","id":"x","ladder":[300],"conditions":[]}"#,
                "no conditions",
            ),
            (
                r#"{"schema":"quantrule.model-spec/v2","id":"x","ladder":[300],"conditions":[{"id":"c","when":{"event":{"id":"signal.rsi_threshold","timeframe":300}}}]}"#,
                "is not quantrule.model-spec/v1",
            ),
            (
                r#"{"schema":"quantrule.model-spec/v1","id":"x","ladder":[300],"conditions":[{"id":"c","when":{"event":{"id":"signal.missing","timeframe":300}}}]}"#,
                "not in the registry",
            ),
            (
                r#"{"schema":"quantrule.model-spec/v1","id":"x","ladder":[300],"conditions":[{"id":"c","when":{"state":{"state":"missing","timeframe":300,"equals":1}}}]}"#,
                "the state missing",
            ),
            (
                r#"{"schema":"quantrule.model-spec/v1","id":"x","ladder":[300],"conditions":[{"id":"c","when":{"event":{"id":"signal.rsi_threshold","timeframe":900}}}]}"#,
                "not in the ladder",
            ),
            (
                r#"{"schema":"quantrule.model-spec/v1","id":"x","ladder":[300],"conditions":[{"id":"c","when":{"all":[]}}]}"#,
                "no children",
            ),
            (
                r#"{"schema":"quantrule.model-spec/v1","id":"x","ladder":[300],"conditions":[{"id":"c","when":{"event":{"id":"signal.rsi_threshold","timeframe":300}},"extra":1}]}"#,
                "unknown field",
            ),
        ];
        for (document, expected) in cases {
            let error = parse_model_spec(document).expect_err(document);
            assert!(
                error.to_string().contains(expected),
                "expected {expected:?} in {error}"
            );
        }
    }
}
