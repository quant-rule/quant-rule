//! `strategy.json`: the human-owned strategy rule for the strategy stage.
//!
//! The file names one of the generic engines in `quantrule-backtest`, the
//! execution timeframe, optional context timeframes, the risk policy, and the
//! engine-specific conditions. The parser validates the file before a model
//! bundle is read. It reads no candle, model value, clock, or account state.
//!
//! The schema is `quantrule.strategy/v1`. It is separate from `run.json`: the
//! run composition root keeps its `quantrule.run/v2` shape, and the strategy
//! command takes this file explicitly.

use std::path::Path;

use quantrule_backtest::intent::{RiskPolicy, RiskPolicyError};
use quantrule_backtest::strategy::{
    CompressionBracketConfig, Condition, EventCompositeConfig, EventStrategyInput, StateField,
};
use quantrule_backtest::strategy::{
    CompressionBracketInput, CompressionBracketStrategy, Decision, EventCompositeStrategy,
};
use quantrule_backtest::{intent::StrategyPosition, side::Side};
use quantrule_data::Timeframe;
use serde_json::{Map, Value};

use super::error::ConfigError;
use super::read::{self, ROOT};

/// The strategy configuration schema.
pub(crate) const SCHEMA: &str = "quantrule.strategy/v1";

/// One validated strategy configuration.
#[derive(Debug, Clone)]
pub(crate) struct StrategyConfig {
    /// The optional human label, recorded in the decision summary.
    pub(crate) id: Option<String>,
    /// The timeframe whose closed bars produce decisions.
    pub(crate) execution_timeframe: Timeframe,
    /// The context timeframes read at each execution bar, ascending.
    pub(crate) context_timeframes: Vec<Timeframe>,
    /// The generic engine and its validated conditions.
    engine: StrategyEngine,
    /// The ATR stop and reward-to-risk policy attached to an entry.
    pub(crate) risk: RiskPolicy,
    /// Whether a decision requires the next execution bar to be contiguous.
    pub(crate) require_contiguous: bool,
}

/// The generic strategy engine selected by the configuration.
#[derive(Debug, Clone, Copy)]
pub(crate) enum StrategyEngine {
    /// The bounded event/state conjunction engine.
    EventComposite(EventCompositeConfig),
    /// The two-sided compression-range bracket engine.
    CompressionBracket(CompressionBracketConfig),
}

impl StrategyConfig {
    /// The engine name recorded in the summary and provenance.
    #[must_use]
    pub(crate) fn engine_name(&self) -> &'static str {
        match self.engine {
            StrategyEngine::EventComposite(_) => "event_composite",
            StrategyEngine::CompressionBracket(_) => "compression_bracket",
        }
    }

    /// The event window in bars used to build the per-bar snapshot.
    #[must_use]
    pub(crate) fn event_window_bars(&self) -> usize {
        match self.engine {
            StrategyEngine::EventComposite(config) => config.window_bars(),
            // The bracket reads only the event that became knowable on the
            // current execution bar.
            StrategyEngine::CompressionBracket(_) => 1,
        }
    }

    /// The entry conditions for the event-composite engine.
    #[must_use]
    pub(crate) fn entry_conditions(&self) -> &[Condition] {
        match &self.engine {
            StrategyEngine::EventComposite(config) => config.entry_conditions(),
            StrategyEngine::CompressionBracket(_) => &[],
        }
    }

    /// The exit conditions for the event-composite engine.
    #[must_use]
    pub(crate) fn exit_conditions(&self) -> &[Condition] {
        match &self.engine {
            StrategyEngine::EventComposite(config) => config.exit_conditions(),
            StrategyEngine::CompressionBracket(_) => &[],
        }
    }

    /// The timeframes named by the event-composite conditions.
    #[must_use]
    pub(crate) fn condition_timeframes(&self) -> Vec<u64> {
        self.entry_conditions()
            .iter()
            .chain(self.exit_conditions())
            .map(|condition| match condition {
                Condition::Event { timeframe_secs, .. }
                | Condition::State { timeframe_secs, .. } => u64::from(*timeframe_secs),
            })
            .collect()
    }

    /// The compression event identity for the bracket engine.
    #[must_use]
    pub(crate) fn compression_event(&self) -> Option<(u64, u8, u8, u16)> {
        match self.engine {
            StrategyEngine::EventComposite(_) => None,
            StrategyEngine::CompressionBracket(config) => Some((
                config.event_timeframe_secs(),
                config.event_type(),
                config.event_subtype(),
                config.event_source(),
            )),
        }
    }

    /// Applies the configured engine to one closed bar and position.
    #[must_use]
    pub(crate) fn decide(
        &self,
        input: &EventStrategyInput,
        position: StrategyPosition,
    ) -> Decision {
        match self.engine {
            StrategyEngine::EventComposite(config) => {
                EventCompositeStrategy::new(config, self.risk, self.require_contiguous)
                    .decide(input, position)
            }
            StrategyEngine::CompressionBracket(config) => {
                let Some(execution) = input.timeframe(input.execution_timeframe_secs) else {
                    return Decision::Hold;
                };
                let bracket = CompressionBracketInput {
                    execution_timeframe_secs: input.execution_timeframe_secs,
                    execution_bar_index: input.execution_bar_index,
                    execution_open_time: input.execution_open_time,
                    execution_contiguous: input.execution_contiguous,
                    events: &execution.events,
                };
                CompressionBracketStrategy::new(config, self.risk, self.require_contiguous)
                    .decide(&bracket, position)
            }
        }
    }

    /// The side a decision intends, when the engine selected one.
    #[must_use]
    pub(crate) fn decision_side(decision: Decision) -> Option<Side> {
        match decision {
            Decision::Enter(entry) => Some(entry.side()),
            Decision::Exit(exit) => Some(exit.side),
            Decision::EnterBracket(_) | Decision::Hold => None,
        }
    }
}

/// Loads and validates a strategy file and returns its content identity.
///
/// The identity is a stable FNV-1a fingerprint of the file bytes. It is an
/// identity aid, not a security primitive.
///
/// # Errors
///
/// Returns [`ConfigError`] when the file is missing, unreadable, not JSON, or
/// disagrees with the schema.
pub(crate) fn load(path: &Path) -> Result<(StrategyConfig, String), ConfigError> {
    let file = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("strategy.json");
    let text = std::fs::read_to_string(path).map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => ConfigError::missing_file(file),
        _ => ConfigError::unreadable_file(file),
    })?;
    let value: Value = serde_json::from_str(&text)
        .map_err(|error| ConfigError::invalid_json(file, &error.to_string()))?;
    let config = parse(&value, file)?;
    Ok((config, crate::fingerprint::fnv1a(text.as_bytes())))
}

/// Parses and validates one strategy document.
///
/// # Errors
///
/// Returns [`ConfigError`] naming the file and JSON path when a field is
/// missing, unknown, of the wrong type, or outside its allowed range.
pub(crate) fn parse(value: &Value, file: &str) -> Result<StrategyConfig, ConfigError> {
    let object = read::object(file, ROOT, value)?;
    read::schema(file, object, SCHEMA)?;
    let engine = read::string(file, "$.engine", read::field(file, ROOT, object, "engine")?)?;
    let allowed: &[&str] = match engine {
        "event_composite" => &[
            "schema",
            "id",
            "engine",
            "execution_timeframe",
            "context_timeframes",
            "event_window_bars",
            "time_stop_bars",
            "require_contiguous",
            "risk",
            "entry",
            "exit",
        ],
        "compression_bracket" => &[
            "schema",
            "id",
            "engine",
            "execution_timeframe",
            "context_timeframes",
            "time_stop_bars",
            "require_contiguous",
            "risk",
            "event",
            "entry_valid_bars",
        ],
        other => {
            return Err(ConfigError::range(
                file,
                "$.engine",
                "event_composite or compression_bracket",
                other,
            ));
        }
    };
    read::check_fields(file, ROOT, object, allowed)?;

    let id = object
        .get("id")
        .map(|value| read::non_empty_string(file, "$.id", value).map(str::to_owned))
        .transpose()?;
    let execution_timeframe = parse_timeframe(
        file,
        "$.execution_timeframe",
        read::field(file, ROOT, object, "execution_timeframe")?,
    )?;
    let context_timeframes = parse_context_timeframes(file, object, execution_timeframe)?;
    let risk = parse_risk(file, read::field(file, ROOT, object, "risk")?)?;
    let require_contiguous = object
        .get("require_contiguous")
        .map(|value| read::boolean(file, "$.require_contiguous", value))
        .transpose()?
        .unwrap_or(true);

    let engine = match engine {
        "event_composite" => {
            let event_window_bars = optional_u64(file, object, "event_window_bars", 1)?;
            let event_window_bars = usize::try_from(event_window_bars).map_err(|_| {
                ConfigError::range(
                    file,
                    "$.event_window_bars",
                    "a representable bar count",
                    &event_window_bars.to_string(),
                )
            })?;
            let time_stop_bars = optional_u64(file, object, "time_stop_bars", 0)?;
            let entry =
                parse_conditions(file, "$.entry", read::field(file, ROOT, object, "entry")?)?;
            let exit = object
                .get("exit")
                .map(|value| parse_conditions(file, "$.exit", value))
                .transpose()?
                .unwrap_or_default();
            let config =
                EventCompositeConfig::new(event_window_bars, time_stop_bars, &entry, &exit)
                    .map_err(|error| {
                        ConfigError::range(
                            file,
                            "$",
                            "a valid event-composite strategy",
                            &error.to_string(),
                        )
                    })?;
            StrategyEngine::EventComposite(config)
        }
        "compression_bracket" => {
            let event = read::object(file, "$.event", read::field(file, ROOT, object, "event")?)?;
            read::check_fields(
                file,
                "$.event",
                event,
                &["timeframe", "event_type", "subtype", "source"],
            )?;
            let event_timeframe = parse_timeframe(
                file,
                "$.event.timeframe",
                read::field(file, "$.event", event, "timeframe")?,
            )?;
            if event_timeframe != execution_timeframe {
                return Err(ConfigError::inconsistent(
                    file,
                    "$.event.timeframe",
                    "the execution timeframe",
                    event_timeframe.code(),
                ));
            }
            let event_type = integer_u8(
                file,
                "$.event.event_type",
                read::field(file, "$.event", event, "event_type")?,
            )?;
            let subtype = integer_u8(
                file,
                "$.event.subtype",
                read::field(file, "$.event", event, "subtype")?,
            )?;
            let source = integer_u16(
                file,
                "$.event.source",
                read::field(file, "$.event", event, "source")?,
            )?;
            let entry_valid_bars = integer_u32(
                file,
                "$.entry_valid_bars",
                read::field(file, ROOT, object, "entry_valid_bars")?,
            )?;
            let time_stop_bars = read::u64(
                file,
                "$.time_stop_bars",
                read::field(file, ROOT, object, "time_stop_bars")?,
            )?;
            let config = CompressionBracketConfig::new(
                u64::from(event_timeframe.seconds()),
                event_type,
                subtype,
                source,
                entry_valid_bars,
                time_stop_bars,
            )
            .map_err(|error| {
                ConfigError::range(
                    file,
                    "$",
                    "a valid compression-bracket strategy",
                    &error.to_string(),
                )
            })?;
            StrategyEngine::CompressionBracket(config)
        }
        _ => unreachable!("the engine name was matched above"),
    };

    Ok(StrategyConfig {
        id,
        execution_timeframe,
        context_timeframes,
        engine,
        risk,
        require_contiguous,
    })
}

fn parse_timeframe(file: &str, path: &str, value: &Value) -> Result<Timeframe, ConfigError> {
    let code = read::string(file, path, value)?;
    let timeframe = Timeframe::from_code(code)
        .ok_or_else(|| ConfigError::range(file, path, "1m, 5m, 15m, or 1h", code))?;
    if timeframe.seconds() > Timeframe::H1.seconds() {
        return Err(ConfigError::range(
            file,
            path,
            "a timeframe at or below the model ceiling 1h",
            code,
        ));
    }
    Ok(timeframe)
}

fn parse_context_timeframes(
    file: &str,
    object: &Map<String, Value>,
    execution: Timeframe,
) -> Result<Vec<Timeframe>, ConfigError> {
    let Some(value) = object.get("context_timeframes") else {
        return Ok(Vec::new());
    };
    let values = read::array(file, "$.context_timeframes", value)?;
    let mut timeframes = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        let path = read::index("$.context_timeframes", index);
        let timeframe = parse_timeframe(file, &path, value)?;
        if timeframe == execution {
            return Err(ConfigError::range(
                file,
                &path,
                "a context timeframe different from the execution timeframe",
                timeframe.code(),
            ));
        }
        if timeframes.contains(&timeframe) {
            return Err(ConfigError::range(
                file,
                &path,
                "a unique context timeframe",
                timeframe.code(),
            ));
        }
        timeframes.push(timeframe);
    }
    timeframes.sort_by_key(|timeframe| timeframe.seconds());
    Ok(timeframes)
}

fn parse_risk(file: &str, value: &Value) -> Result<RiskPolicy, ConfigError> {
    let object = read::object(file, "$.risk", value)?;
    read::check_fields(
        file,
        "$.risk",
        object,
        &["stop_atr_multiple", "reward_to_risk"],
    )?;
    let stop = read::number(
        file,
        "$.risk.stop_atr_multiple",
        read::field(file, "$.risk", object, "stop_atr_multiple")?,
    )?;
    let reward = read::number(
        file,
        "$.risk.reward_to_risk",
        read::field(file, "$.risk", object, "reward_to_risk")?,
    )?;
    RiskPolicy::new(stop, reward).map_err(|error| risk_error(file, error))
}

fn risk_error(file: &str, error: RiskPolicyError) -> ConfigError {
    ConfigError::range(
        file,
        "$.risk",
        "a positive finite ATR stop and reward-to-risk",
        &error.to_string(),
    )
}

fn parse_conditions(file: &str, path: &str, value: &Value) -> Result<Vec<Condition>, ConfigError> {
    let values = read::array(file, path, value)?;
    let mut conditions = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        conditions.push(parse_condition(file, &read::index(path, index), value)?);
    }
    Ok(conditions)
}

fn parse_condition(file: &str, path: &str, value: &Value) -> Result<Condition, ConfigError> {
    let object = read::object(file, path, value)?;
    let kind = read::string(
        file,
        &format!("{path}.kind"),
        read::field(file, path, object, "kind")?,
    )?;
    match kind {
        "event" => {
            read::check_fields(
                file,
                path,
                object,
                &[
                    "kind",
                    "timeframe",
                    "event_type",
                    "subtype",
                    "source",
                    "direction",
                ],
            )?;
            let timeframe = parse_timeframe(
                file,
                &format!("{path}.timeframe"),
                read::field(file, path, object, "timeframe")?,
            )?;
            let event_type = integer_u8(
                file,
                &format!("{path}.event_type"),
                read::field(file, path, object, "event_type")?,
            )?;
            let subtype = integer_u8(
                file,
                &format!("{path}.subtype"),
                read::field(file, path, object, "subtype")?,
            )?;
            let source = integer_u8(
                file,
                &format!("{path}.source"),
                read::field(file, path, object, "source")?,
            )?;
            let direction = integer_i8(
                file,
                &format!("{path}.direction"),
                read::field(file, path, object, "direction")?,
            )?;
            Ok(Condition::Event {
                timeframe_secs: timeframe.seconds(),
                event_type,
                subtype,
                source,
                direction,
            })
        }
        "state" => {
            read::check_fields(file, path, object, &["kind", "timeframe", "field", "value"])?;
            let timeframe = parse_timeframe(
                file,
                &format!("{path}.timeframe"),
                read::field(file, path, object, "timeframe")?,
            )?;
            let field = read::string(
                file,
                &format!("{path}.field"),
                read::field(file, path, object, "field")?,
            )?;
            let field = StateField::from_code(field).ok_or_else(|| {
                ConfigError::range(
                    file,
                    &format!("{path}.field"),
                    "a registered state field",
                    field,
                )
            })?;
            let value = integer_i8(
                file,
                &format!("{path}.value"),
                read::field(file, path, object, "value")?,
            )?;
            Ok(Condition::State {
                timeframe_secs: timeframe.seconds(),
                field,
                value,
            })
        }
        other => Err(ConfigError::range(
            file,
            &format!("{path}.kind"),
            "event or state",
            other,
        )),
    }
}

fn optional_u64(
    file: &str,
    object: &Map<String, Value>,
    name: &str,
    default: u64,
) -> Result<u64, ConfigError> {
    match object.get(name) {
        Some(value) => read::u64(file, &format!("$.{name}"), value),
        None => Ok(default),
    }
}

fn integer_u8(file: &str, path: &str, value: &Value) -> Result<u8, ConfigError> {
    let number = integer(file, path, value)?;
    u8::try_from(number)
        .map_err(|_| ConfigError::range(file, path, "an integer in 0..=255", &number.to_string()))
}

fn integer_u16(file: &str, path: &str, value: &Value) -> Result<u16, ConfigError> {
    let number = integer(file, path, value)?;
    u16::try_from(number)
        .map_err(|_| ConfigError::range(file, path, "an integer in 0..=65535", &number.to_string()))
}

fn integer_u32(file: &str, path: &str, value: &Value) -> Result<u32, ConfigError> {
    let number = integer(file, path, value)?;
    u32::try_from(number).map_err(|_| {
        ConfigError::range(
            file,
            path,
            "an integer in 0..=4294967295",
            &number.to_string(),
        )
    })
}

fn integer_i8(file: &str, path: &str, value: &Value) -> Result<i8, ConfigError> {
    let number = integer(file, path, value)?;
    i8::try_from(number).map_err(|_| {
        ConfigError::range(file, path, "an integer in -128..=127", &number.to_string())
    })
}

fn integer(file: &str, path: &str, value: &Value) -> Result<i64, ConfigError> {
    value
        .as_i64()
        .ok_or_else(|| ConfigError::wrong_type(file, path, "an integer", &read::observed(value)))
}

#[cfg(test)]
mod tests {
    use super::{SCHEMA, parse};
    use serde_json::json;

    fn valid() -> serde_json::Value {
        json!({
            "schema": SCHEMA,
            "id": "S1.0",
            "engine": "event_composite",
            "execution_timeframe": "5m",
            "context_timeframes": ["15m", "1h"],
            "event_window_bars": 3,
            "time_stop_bars": 0,
            "require_contiguous": true,
            "risk": { "stop_atr_multiple": 2.0, "reward_to_risk": 1.5 },
            "entry": [
                { "kind": "event", "timeframe": "5m", "event_type": 3, "subtype": 1, "source": 4, "direction": 1 }
            ],
            "exit": [
                { "kind": "state", "timeframe": "5m", "field": "macd_state", "value": -1 }
            ]
        })
    }

    #[test]
    fn a_valid_event_composite_config_parses() {
        let config = parse(&valid(), "strategy.json").expect("the config is valid");
        assert_eq!(config.engine_name(), "event_composite");
        assert_eq!(config.execution_timeframe.code(), "5m");
        assert_eq!(config.context_timeframes.len(), 2);
        assert_eq!(config.entry_conditions().len(), 1);
        assert_eq!(config.exit_conditions().len(), 1);
    }

    #[test]
    fn a_valid_compression_bracket_config_parses() {
        let config = parse(
            &json!({
                "schema": SCHEMA,
                "engine": "compression_bracket",
                "execution_timeframe": "5m",
                "time_stop_bars": 12,
                "risk": { "stop_atr_multiple": 2.0, "reward_to_risk": 1.5 },
                "event": { "timeframe": "5m", "event_type": 3, "subtype": 8, "source": 5 },
                "entry_valid_bars": 1
            }),
            "strategy.json",
        )
        .expect("the config is valid");
        assert_eq!(config.engine_name(), "compression_bracket");
        assert_eq!(config.compression_event(), Some((300, 3, 8, 5)));
    }

    #[test]
    fn an_unknown_engine_is_rejected() {
        let mut value = valid();
        value["engine"] = json!("weighted_score");
        let error = parse(&value, "strategy.json").expect_err("the engine is invalid");
        assert!(error.to_string().contains("event_composite"));
    }

    #[test]
    fn a_context_timeframe_equal_to_execution_is_rejected() {
        let mut value = valid();
        value["context_timeframes"] = json!(["5m"]);
        let error = parse(&value, "strategy.json").expect_err("the context repeats execution");
        assert!(error.to_string().contains("context timeframe"));
    }

    #[test]
    fn an_unknown_field_is_rejected() {
        let mut value = valid();
        value["entry"][0]["confidence"] = json!(0.9);
        let error = parse(&value, "strategy.json").expect_err("the field is unknown");
        assert!(error.to_string().contains("confidence"));
    }

    #[test]
    fn a_bracket_event_timeframe_must_equal_execution() {
        let error = parse(
            &json!({
                "schema": SCHEMA,
                "engine": "compression_bracket",
                "execution_timeframe": "5m",
                "time_stop_bars": 12,
                "risk": { "stop_atr_multiple": 2.0, "reward_to_risk": 1.5 },
                "event": { "timeframe": "15m", "event_type": 3, "subtype": 8, "source": 5 },
                "entry_valid_bars": 1
            }),
            "strategy.json",
        )
        .expect_err("the event timeframe differs from execution");
        assert!(error.to_string().contains("execution timeframe"));
    }

    #[test]
    fn a_compression_event_window_is_rejected() {
        let mut value = valid();
        value["engine"] = json!("compression_bracket");
        value["time_stop_bars"] = json!(12);
        value["event"] = json!({ "timeframe": "5m", "event_type": 3, "subtype": 8, "source": 5 });
        value["entry_valid_bars"] = json!(1);
        value.as_object_mut().expect("an object").remove("entry");
        value.as_object_mut().expect("an object").remove("exit");
        value["event_window_bars"] = json!(3);
        let error = parse(&value, "strategy.json").expect_err("the window is not a bracket field");
        assert!(error.to_string().contains("event_window_bars"));
    }
}
