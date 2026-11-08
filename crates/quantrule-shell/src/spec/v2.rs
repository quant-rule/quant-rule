//! The `quantrule.model-spec/v2` JSON parser.
//!
//! The shell resolves event identifiers, reading overrides, the panel sources,
//! and the session calendar; the model receives a validated [`ModelSpecV2`] and
//! performs no I/O. The parser shares the embedded registry with the v1 parser
//! so both schemas resolve an event ID to the same numeric code and role.

use std::collections::{BTreeMap, BTreeSet};

use quantrule_model::input::{EventSettings, MarketStateConfig};
use quantrule_model::readings::{ExtendedConfig, PivotScale, ReadingsConfig};
use quantrule_model::sessions::{SessionConfig, SessionZone};
use quantrule_model::spec::v2::{
    ModelSpecV2, PredictionKind, SPEC_V2_SCHEMA, SpecEventCode, SpecPrediction,
};
use quantrule_model::spec::{ModelSpec, SPEC_SCHEMA, SpecCondition};
use serde_json::{Map, Value};

use super::{
    Registry, SpecParseError, allow_only, as_array, as_object, as_str, as_u64, field,
    parse_condition,
};

/// A parsed v2 document and the panel source paths the shell must load.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ParsedModelSpecV2 {
    /// The resolved model spec.
    pub(crate) spec: ModelSpecV2,
    /// Panel instrument -> source path as written, excluding the evaluated
    /// instrument. The shell resolves each path against the spec file.
    pub(crate) panel_sources: BTreeMap<String, String>,
}

/// Parses and validates a `quantrule.model-spec/v2` document.
///
/// # Errors
///
/// Returns a [`SpecParseError`] when the JSON is malformed, a field is missing
/// or mistyped, a reading override is invalid, an event identifier is unknown,
/// the prediction is inconsistent, or the resolved spec fails validation.
pub(crate) fn parse_model_spec_v2(text: &str) -> Result<ParsedModelSpecV2, SpecParseError> {
    let value: Value = serde_json::from_str(text)
        .map_err(|error| SpecParseError::new(format!("invalid JSON: {error}")))?;
    let registry = Registry::load()?;
    let object = as_object(&value, "the spec")?;
    allow_only(
        object,
        &[
            "schema",
            "id",
            "family",
            "instrument",
            "timeframes",
            "evaluation_timeframe",
            "readings",
            "active_events",
            "prediction",
            "conditions",
            "panel",
            "session",
        ],
        "the spec",
    )?;
    let schema = as_str(field(object, "schema", "the spec")?, "schema")?;
    if schema != SPEC_V2_SCHEMA {
        return Err(SpecParseError::new(format!(
            "the schema {schema} is not {SPEC_V2_SCHEMA}"
        )));
    }
    let id = non_empty(object, "id", "the spec")?;
    let family = non_empty(object, "family", "the spec")?;
    let instrument = non_empty(object, "instrument", "the spec")?;
    let timeframes = parse_timeframes(object)?;
    let evaluation_timeframe_secs = match object.get("evaluation_timeframe") {
        Some(value) => as_u64(value, "evaluation_timeframe")?,
        None => timeframes[0],
    };
    let mut readings = parse_readings(object.get("readings"))?;
    let active_events = parse_active_events(object, &registry)?;
    enable_active_compression(&mut readings, &active_events);
    let prediction = parse_prediction(object, &registry)?;
    let conditions = match object.get("conditions") {
        Some(value) => as_array(value, "conditions")?
            .iter()
            .map(|value| parse_condition(value, &registry))
            .collect::<Result<Vec<SpecCondition>, _>>()?,
        None => Vec::new(),
    };
    if !conditions.is_empty() {
        let base = ModelSpec {
            schema: SPEC_SCHEMA.to_owned(),
            id: id.clone(),
            ladder: timeframes.clone(),
            evaluation_timeframe_secs,
            conditions: conditions.clone(),
        };
        base.validate()
            .map_err(|error| SpecParseError::new(format!("a v2 condition is invalid: {error}")))?;
    }
    let (panel_instruments, panel_sources) = parse_panel(object, &instrument)?;
    let session = parse_session(object.get("session"))?;
    let spec = ModelSpecV2 {
        schema: schema.to_string(),
        id,
        family,
        instrument,
        timeframes,
        evaluation_timeframe_secs,
        readings,
        active_events,
        prediction,
        conditions,
        session,
        panel_instruments,
    };
    spec.validate()
        .map_err(|error| SpecParseError::new(format!("the spec is invalid: {error}")))?;
    Ok(ParsedModelSpecV2 {
        spec,
        panel_sources,
    })
}

fn non_empty(
    object: &Map<String, Value>,
    name: &str,
    context: &str,
) -> Result<String, SpecParseError> {
    let value = as_str(field(object, name, context)?, name)?;
    if value.trim().is_empty() {
        return Err(SpecParseError::new(format!(
            "{context} has an empty {name}"
        )));
    }
    Ok(value.to_string())
}

fn parse_timeframes(object: &Map<String, Value>) -> Result<Vec<u64>, SpecParseError> {
    let values = as_array(field(object, "timeframes", "the spec")?, "timeframes")?;
    if values.len() != 1 {
        return Err(SpecParseError::new(
            "the v2 spec must declare exactly one timeframe",
        ));
    }
    let timeframe = as_u64(&values[0], "a timeframe")?;
    if timeframe == 0 {
        return Err(SpecParseError::new("the timeframe is zero"));
    }
    Ok(vec![timeframe])
}

fn enable_active_compression(config: &mut MarketStateConfig, active: &[SpecEventCode]) {
    // Compression is disabled by default because it was a legacy v4 family.
    // A v2 spec that explicitly lists a compression event has activated it, so
    // the run must enable the detector rather than silently publish no event.
    let mut events = EventSettings::DEFAULT;
    events.signal.compression_breakout = active
        .iter()
        .any(|event| event.code == quantrule_model::events::codes::SIGNAL_COMPRESSION_BREAKOUT);
    events.signal.compression_range = active
        .iter()
        .any(|event| event.code == quantrule_model::events::codes::SIGNAL_COMPRESSION_RANGE);
    events.signal.compression_range_onset = active
        .iter()
        .any(|event| event.code == quantrule_model::events::codes::SIGNAL_COMPRESSION_RANGE_ONSET);
    config.set_events_default(events);
}

fn parse_readings(value: Option<&Value>) -> Result<MarketStateConfig, SpecParseError> {
    let mut readings = ReadingsConfig::DEFAULT;
    let mut extended = ExtendedConfig::DEFAULT;
    let mut macd_active = true;
    let mut scale = PivotScale::Major;
    if let Some(value) = value {
        let object = as_object(value, "readings")?;
        allow_only(
            object,
            &[
                "structure_scale",
                "ema",
                "rsi",
                "atr",
                "rvol",
                "macd",
                "adx",
                "extended",
            ],
            "readings",
        )?;
        if let Some(value) = object.get("structure_scale") {
            scale = match as_str(value, "structure_scale")? {
                "major" => PivotScale::Major,
                "minor" => PivotScale::Minor,
                other => {
                    return Err(SpecParseError::new(format!(
                        "structure_scale {other} is not major or minor"
                    )));
                }
            };
        }
        if let Some(value) = object.get("ema") {
            let ema = as_object(value, "readings.ema")?;
            allow_only(ema, &["fast", "mid", "slow", "active"], "readings.ema")?;
            readings.ema_fast = optional_usize(ema, "fast", readings.ema_fast)?;
            readings.ema_mid = optional_usize(ema, "mid", readings.ema_mid)?;
            readings.ema_slow = optional_usize(ema, "slow", readings.ema_slow)?;
            readings.ema_active = optional_bool(ema, "active", readings.ema_active)?;
        }
        if let Some(value) = object.get("rsi") {
            let rsi = as_object(value, "readings.rsi")?;
            allow_only(rsi, &["period", "active"], "readings.rsi")?;
            readings.rsi = optional_usize(rsi, "period", readings.rsi)?;
            readings.rsi_active = optional_bool(rsi, "active", readings.rsi_active)?;
        }
        if let Some(value) = object.get("atr") {
            let atr = as_object(value, "readings.atr")?;
            allow_only(atr, &["period", "active"], "readings.atr")?;
            readings.atr = optional_usize(atr, "period", readings.atr)?;
            readings.atr_active = optional_bool(atr, "active", readings.atr_active)?;
        }
        if let Some(value) = object.get("rvol") {
            let rvol = as_object(value, "readings.rvol")?;
            allow_only(rvol, &["period", "active"], "readings.rvol")?;
            readings.rvol = optional_usize(rvol, "period", readings.rvol)?;
            readings.rvol_active = optional_bool(rvol, "active", readings.rvol_active)?;
        }
        if let Some(value) = object.get("macd") {
            let macd = as_object(value, "readings.macd")?;
            allow_only(macd, &["fast", "slow", "signal", "active"], "readings.macd")?;
            readings.macd_fast = optional_usize(macd, "fast", readings.macd_fast)?;
            readings.macd_slow = optional_usize(macd, "slow", readings.macd_slow)?;
            readings.macd_signal = optional_usize(macd, "signal", readings.macd_signal)?;
            macd_active = optional_bool(macd, "active", macd_active)?;
        }
        if let Some(value) = object.get("adx") {
            let adx = as_object(value, "readings.adx")?;
            allow_only(
                adx,
                &["period", "threshold", "slope_bars", "active"],
                "readings.adx",
            )?;
            readings.adx = optional_usize(adx, "period", readings.adx)?;
            readings.adx_threshold = optional_f64(adx, "threshold", readings.adx_threshold)?;
            readings.adx_slope_bars = optional_usize(adx, "slope_bars", readings.adx_slope_bars)?;
            readings.adx_active = optional_bool(adx, "active", readings.adx_active)?;
        }
        if let Some(value) = object.get("extended") {
            extended = parse_extended(value)?;
        }
    }
    let mut config = MarketStateConfig::new(scale);
    config.set_max_timeframe_secs(3_600);
    config.set_readings_default(readings);
    config.set_macd_default(macd_active);
    config.set_extended(extended);
    Ok(config)
}

fn parse_extended(value: &Value) -> Result<ExtendedConfig, SpecParseError> {
    let object = as_object(value, "readings.extended")?;
    allow_only(
        object,
        &[
            "vwap_window",
            "vwap_band_mad",
            "profile_window",
            "profile_bins",
            "profile_value_area",
            "rvol_fast",
            "rvol_slow",
            "volume_oscillator",
            "climax_volume",
            "climax_range_atr",
            "absorption_volume",
            "absorption_range_atr",
            "absorption_level_atr",
            "flow_window",
            "mfi_window",
            "cmf_threshold",
            "atr_slow_window",
            "atr_slope_window",
            "range_window",
            "mar_fast",
            "mar_slow",
            "keltner_window",
            "keltner_atr",
            "donchian_window",
            "level_lookback",
            "level_atr",
            "aroon_window",
            "path_window",
            "candle_body",
            "candle_wick",
            "candle_close",
            "gap_atr",
        ],
        "readings.extended",
    )?;
    let mut config = ExtendedConfig::DEFAULT;
    config.vwap_window = optional_usize(object, "vwap_window", config.vwap_window)?;
    config.vwap_band_mad = optional_f64(object, "vwap_band_mad", config.vwap_band_mad)?;
    config.profile_window = optional_usize(object, "profile_window", config.profile_window)?;
    config.profile_bins = optional_usize(object, "profile_bins", config.profile_bins)?;
    config.profile_value_area =
        optional_f64(object, "profile_value_area", config.profile_value_area)?;
    config.rvol_fast = optional_usize(object, "rvol_fast", config.rvol_fast)?;
    config.rvol_slow = optional_usize(object, "rvol_slow", config.rvol_slow)?;
    config.volume_oscillator =
        optional_usize(object, "volume_oscillator", config.volume_oscillator)?;
    config.climax_volume = optional_f64(object, "climax_volume", config.climax_volume)?;
    config.climax_range_atr = optional_f64(object, "climax_range_atr", config.climax_range_atr)?;
    config.absorption_volume = optional_f64(object, "absorption_volume", config.absorption_volume)?;
    config.absorption_range_atr =
        optional_f64(object, "absorption_range_atr", config.absorption_range_atr)?;
    config.absorption_level_atr =
        optional_f64(object, "absorption_level_atr", config.absorption_level_atr)?;
    config.flow_window = optional_usize(object, "flow_window", config.flow_window)?;
    config.mfi_window = optional_usize(object, "mfi_window", config.mfi_window)?;
    config.cmf_threshold = optional_f64(object, "cmf_threshold", config.cmf_threshold)?;
    config.atr_slow_window = optional_usize(object, "atr_slow_window", config.atr_slow_window)?;
    config.atr_slope_window = optional_usize(object, "atr_slope_window", config.atr_slope_window)?;
    config.range_window = optional_usize(object, "range_window", config.range_window)?;
    config.mar_fast = optional_usize(object, "mar_fast", config.mar_fast)?;
    config.mar_slow = optional_usize(object, "mar_slow", config.mar_slow)?;
    config.keltner_window = optional_usize(object, "keltner_window", config.keltner_window)?;
    config.keltner_atr = optional_f64(object, "keltner_atr", config.keltner_atr)?;
    config.donchian_window = optional_usize(object, "donchian_window", config.donchian_window)?;
    config.level_lookback = optional_usize(object, "level_lookback", config.level_lookback)?;
    config.level_atr = optional_f64(object, "level_atr", config.level_atr)?;
    config.aroon_window = optional_usize(object, "aroon_window", config.aroon_window)?;
    config.path_window = optional_usize(object, "path_window", config.path_window)?;
    config.candle_body = optional_f64(object, "candle_body", config.candle_body)?;
    config.candle_wick = optional_f64(object, "candle_wick", config.candle_wick)?;
    config.candle_close = optional_f64(object, "candle_close", config.candle_close)?;
    config.gap_atr = optional_f64(object, "gap_atr", config.gap_atr)?;
    Ok(config)
}

fn parse_active_events(
    object: &Map<String, Value>,
    registry: &Registry,
) -> Result<Vec<SpecEventCode>, SpecParseError> {
    let values = as_array(field(object, "active_events", "the spec")?, "active_events")?;
    let mut events = Vec::with_capacity(values.len());
    let mut seen = BTreeSet::new();
    for value in values {
        let id = as_str(value, "an active event")?;
        if !seen.insert(id.to_string()) {
            return Err(SpecParseError::new(format!(
                "the active event {id} is duplicated"
            )));
        }
        events.push(resolve_event(id, registry)?);
    }
    Ok(events)
}

fn parse_prediction(
    object: &Map<String, Value>,
    registry: &Registry,
) -> Result<SpecPrediction, SpecParseError> {
    let prediction = as_object(field(object, "prediction", "the spec")?, "prediction")?;
    allow_only(
        prediction,
        &["kind", "long", "short", "events", "min_votes"],
        "prediction",
    )?;
    let kind = match prediction.get("kind") {
        Some(value) => match as_str(value, "prediction.kind")? {
            "vote" => PredictionKind::Vote,
            "direction" => PredictionKind::Direction,
            other => {
                return Err(SpecParseError::new(format!(
                    "prediction.kind {other} is not vote or direction"
                )));
            }
        },
        None => PredictionKind::Vote,
    };
    let has_sides = prediction.get("long").is_some() || prediction.get("short").is_some();
    let has_events = prediction.get("events").is_some();
    if kind == PredictionKind::Vote && has_events {
        return Err(SpecParseError::new(
            "a vote prediction must not carry an events list",
        ));
    }
    if kind == PredictionKind::Direction && has_sides {
        return Err(SpecParseError::new(
            "a direction prediction must not carry long or short lists",
        ));
    }
    let long = if kind == PredictionKind::Vote {
        parse_event_list(prediction.get("long"), "prediction.long", registry)?
    } else {
        Vec::new()
    };
    let short = if kind == PredictionKind::Vote {
        parse_event_list(prediction.get("short"), "prediction.short", registry)?
    } else {
        Vec::new()
    };
    let events = if kind == PredictionKind::Direction {
        parse_event_list(prediction.get("events"), "prediction.events", registry)?
    } else {
        Vec::new()
    };
    let min_votes = match prediction.get("min_votes") {
        Some(value) => {
            let value = as_u64(value, "prediction.min_votes")?;
            usize::try_from(value)
                .map_err(|_| SpecParseError::new("prediction.min_votes is too large"))?
        }
        None => 1,
    };
    Ok(SpecPrediction {
        kind,
        long,
        short,
        events,
        min_votes,
    })
}

fn parse_event_list(
    value: Option<&Value>,
    context: &str,
    registry: &Registry,
) -> Result<Vec<SpecEventCode>, SpecParseError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    as_array(value, context)?
        .iter()
        .map(|value| {
            let id = as_str(value, "a prediction event")?;
            resolve_event(id, registry)
        })
        .collect()
}

fn resolve_event(id: &str, registry: &Registry) -> Result<SpecEventCode, SpecParseError> {
    let code = registry
        .events
        .get(id)
        .ok_or_else(|| SpecParseError::new(format!("the event id {id} is not in the registry")))?;
    Ok(SpecEventCode {
        id: id.to_string(),
        code: code.code,
        role: code.role,
        event_type: code.event_type,
        subtype: code.subtype,
        source: code.source,
    })
}

fn parse_panel(
    object: &Map<String, Value>,
    instrument: &str,
) -> Result<(Vec<String>, BTreeMap<String, String>), SpecParseError> {
    let Some(value) = object.get("panel") else {
        return Ok((Vec::new(), BTreeMap::new()));
    };
    let panel = as_object(value, "panel")?;
    allow_only(panel, &["leader", "sources"], "panel")?;
    let leader = non_empty(panel, "leader", "panel")?;
    let sources = as_object(field(panel, "sources", "panel")?, "panel.sources")?;
    if sources.is_empty() {
        return Err(SpecParseError::new("panel.sources is empty"));
    }
    let mut paths = BTreeMap::new();
    for (name, value) in sources {
        if name.trim().is_empty() {
            return Err(SpecParseError::new("panel.sources has an empty name"));
        }
        if name == instrument {
            return Err(SpecParseError::new(format!(
                "panel.sources must not repeat the evaluated instrument {instrument}; it comes from the run config"
            )));
        }
        let path = as_str(value, "a panel source path")?;
        if path.trim().is_empty() {
            return Err(SpecParseError::new("a panel source path is empty"));
        }
        paths.insert(name.clone(), path.to_string());
    }
    if leader != instrument && !paths.contains_key(&leader) {
        return Err(SpecParseError::new(format!(
            "panel.leader {leader} is neither the evaluated instrument nor a panel source"
        )));
    }
    let mut names = Vec::with_capacity(paths.len() + 1);
    names.push(leader.clone());
    if instrument != leader {
        names.push(instrument.to_string());
    }
    for name in paths.keys() {
        if name != &leader {
            names.push(name.clone());
        }
    }
    Ok((names, paths))
}

fn parse_session(value: Option<&Value>) -> Result<Option<SessionConfig>, SpecParseError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let object = as_object(value, "session")?;
    allow_only(
        object,
        &[
            "zone",
            "start_hour",
            "end_hour",
            "opening_range_bars",
            "sweep_atr",
        ],
        "session",
    )?;
    let mut config = SessionConfig::london();
    if let Some(value) = object.get("zone") {
        config.zone = match as_str(value, "session.zone")? {
            "utc" => SessionZone::Utc,
            "london" => SessionZone::London,
            "new_york" => SessionZone::NewYork,
            other => {
                return Err(SpecParseError::new(format!(
                    "session.zone {other} is not utc, london, or new_york"
                )));
            }
        };
    }
    if let Some(value) = object.get("start_hour") {
        config.start_hour = u32::try_from(as_u64(value, "session.start_hour")?)
            .map_err(|_| SpecParseError::new("session.start_hour is too large"))?;
    }
    if let Some(value) = object.get("end_hour") {
        config.end_hour = u32::try_from(as_u64(value, "session.end_hour")?)
            .map_err(|_| SpecParseError::new("session.end_hour is too large"))?;
    }
    if let Some(value) = object.get("opening_range_bars") {
        config.opening_range_bars =
            usize::try_from(as_u64(value, "session.opening_range_bars")?)
                .map_err(|_| SpecParseError::new("session.opening_range_bars is too large"))?;
    }
    config.sweep_atr = optional_f64(object, "sweep_atr", config.sweep_atr)?;
    if !config.sweep_atr.is_finite() || config.sweep_atr < 0.0 {
        return Err(SpecParseError::new(
            "session.sweep_atr must be finite and non-negative",
        ));
    }
    Ok(Some(config))
}

fn optional_usize(
    object: &Map<String, Value>,
    name: &str,
    default: usize,
) -> Result<usize, SpecParseError> {
    match object.get(name) {
        Some(value) => usize::try_from(as_u64(value, name)?)
            .map_err(|_| SpecParseError::new(format!("{name} is too large"))),
        None => Ok(default),
    }
}

fn optional_bool(
    object: &Map<String, Value>,
    name: &str,
    default: bool,
) -> Result<bool, SpecParseError> {
    match object.get(name) {
        Some(value) => value
            .as_bool()
            .ok_or_else(|| SpecParseError::new(format!("{name} must be a boolean"))),
        None => Ok(default),
    }
}

fn optional_f64(
    object: &Map<String, Value>,
    name: &str,
    default: f64,
) -> Result<f64, SpecParseError> {
    match object.get(name) {
        Some(value) => value
            .as_f64()
            .ok_or_else(|| SpecParseError::new(format!("{name} must be a number"))),
        None => Ok(default),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_model_spec_v2;

    const FIXTURE: &str = r#"
    {
      "schema": "quantrule.model-spec/v2",
      "id": "M2.1",
      "family": "M2",
      "instrument": "XAU",
      "timeframes": [3600],
      "readings": {
        "ema": {"fast": 10, "mid": 30, "slow": 90},
        "rsi": {"period": 10},
        "extended": {"vwap_window": 12, "profile_window": 48}
      },
      "active_events": [
        "signal.rsi_threshold",
        "signal.session_open",
        "signal.relative_strength_cross"
      ],
      "prediction": {
        "kind": "vote",
        "long": ["signal.rsi_threshold"],
        "short": [],
        "min_votes": 1
      },
      "conditions": [
        {
          "id": "session_open",
          "when": {
            "event": {"id": "signal.session_open", "timeframe": 3600}
          }
        }
      ],
      "panel": {
        "leader": "BTC",
        "sources": {
          "BTC": "../btc.csv",
          "SP500": "../sp500.csv",
          "EURUSD": "../eurusd.csv"
        }
      },
      "session": {
        "zone": "london",
        "start_hour": 8,
        "end_hour": 16,
        "opening_range_bars": 2
      }
    }
    "#;

    #[test]
    fn a_v2_fixture_parses_and_resolves() {
        let parsed = parse_model_spec_v2(FIXTURE).expect("the fixture parses");
        assert_eq!(parsed.spec.id, "M2.1");
        assert_eq!(parsed.spec.family, "M2");
        assert_eq!(parsed.spec.instrument, "XAU");
        assert_eq!(parsed.spec.timeframes, [3600]);
        assert_eq!(parsed.spec.readings.readings_for(3600).ema_fast, 10);
        assert_eq!(parsed.spec.readings.extended().vwap_window, 12);
        assert_eq!(parsed.spec.active_events.len(), 3);
        assert_eq!(parsed.spec.prediction.long.len(), 1);
        assert_eq!(parsed.spec.conditions.len(), 1);
        assert_eq!(
            parsed.spec.panel_instruments,
            ["BTC", "XAU", "EURUSD", "SP500"]
        );
        assert_eq!(parsed.panel_sources.len(), 3);
        assert!(!parsed.panel_sources.contains_key("XAU"));
        assert!(parsed.spec.session.is_some());
    }

    #[test]
    fn an_active_compression_event_enables_the_detector() {
        let document = FIXTURE.replace(
            "\"signal.relative_strength_cross\"",
            "\"signal.relative_strength_cross\", \"signal.compression_breakout\"",
        );
        let parsed = parse_model_spec_v2(&document).expect("the compression spec parses");
        assert!(
            parsed
                .spec
                .readings
                .events_for(3600)
                .signal
                .compression_breakout
        );
    }

    #[test]
    fn a_v2_fixture_rejects_two_timeframes() {
        let document = FIXTURE.replace("\"timeframes\": [3600]", "\"timeframes\": [3600, 900]");
        let error = parse_model_spec_v2(&document).expect_err("two timeframes");
        assert!(error.to_string().contains("exactly one timeframe"));
    }

    #[test]
    fn a_v2_fixture_rejects_a_context_vote() {
        let document = FIXTURE.replace(
            "\"long\": [\"signal.rsi_threshold\"]",
            "\"long\": [\"signal.session_open\"]",
        );
        let error = parse_model_spec_v2(&document).expect_err("a context vote");
        assert!(error.to_string().contains("not a signal-role event"));
    }

    #[test]
    fn a_v2_fixture_rejects_an_inactive_prediction_event() {
        let document = FIXTURE.replace(
            "\"long\": [\"signal.rsi_threshold\"]",
            "\"long\": [\"signal.ema_pair_cross\"]",
        );
        let error = parse_model_spec_v2(&document).expect_err("an inactive event");
        assert!(error.to_string().contains("not active"));
    }

    #[test]
    fn a_v2_fixture_rejects_the_evaluated_instrument_in_panel_sources() {
        let document = FIXTURE.replace(
            "\"BTC\": \"../btc.csv\"",
            "\"BTC\": \"../btc.csv\",\n          \"XAU\": \"../xau.csv\"",
        );
        let error = parse_model_spec_v2(&document).expect_err("the evaluated source");
        assert!(
            error
                .to_string()
                .contains("must not repeat the evaluated instrument")
        );
    }
}

#[cfg(test)]
mod direction_tests {
    use super::parse_model_spec_v2;
    use quantrule_model::spec::v2::PredictionKind;

    const DOCUMENT: &str = r#"
    {
      "schema": "quantrule.model-spec/v2",
      "id": "M2.direction",
      "family": "M2",
      "instrument": "XAU",
      "timeframes": [3600],
      "active_events": ["signal.rsi_threshold", "signal.ema_pair_cross"],
      "prediction": {
        "kind": "direction",
        "events": ["signal.rsi_threshold", "signal.ema_pair_cross"],
        "min_votes": 1
      }
    }
    "#;

    #[test]
    fn a_direction_prediction_parses() {
        let parsed = parse_model_spec_v2(DOCUMENT).expect("the direction spec parses");
        assert_eq!(parsed.spec.prediction.kind, PredictionKind::Direction);
        assert_eq!(parsed.spec.prediction.events.len(), 2);
        assert!(parsed.spec.prediction.long.is_empty());
        assert!(parsed.spec.prediction.short.is_empty());
    }

    #[test]
    fn a_direction_prediction_rejects_sides() {
        let document = DOCUMENT.replace(
            "\"events\": [\"signal.rsi_threshold\", \"signal.ema_pair_cross\"],",
            "\"long\": [\"signal.rsi_threshold\"],",
        );
        let error = parse_model_spec_v2(&document).expect_err("a side list");
        assert!(
            error
                .to_string()
                .contains("must not carry long or short lists")
        );
    }
}
