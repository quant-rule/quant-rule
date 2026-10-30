//! `model.json`: the per-timeframe reading configuration.
//!
//! The model's axis formulas, event rules, and aggregation rules remain fixed
//! by the model contract. Version 2 exposes the scalar reading periods:
//! defaults for every timeframe plus optional per-timeframe overrides. Version
//! 1 remains accepted for the historical `structure_scale`-only shape.

use quantrule_data::Timeframe;
use quantrule_model::input::MarketStateConfig;
use quantrule_model::readings::{PivotScale, ReadingsConfig};
use serde_json::Value;

use super::error::ConfigError;
use super::read::{self, ROOT};

/// Parses `model.json`.
pub(crate) fn parse(value: &Value, file: &str) -> Result<MarketStateConfig, ConfigError> {
    let object = read::object(file, ROOT, value)?;
    let schema = read::string(file, "$.schema", read::field(file, ROOT, object, "schema")?)?;
    match schema {
        "quantrule.model/v1" => parse_v1(file, object),
        "quantrule.model/v2" => parse_v2(file, object),
        _ => Err(ConfigError::unsupported(
            file,
            "$.schema",
            "quantrule.model/v1 or quantrule.model/v2",
            schema,
        )),
    }
}

fn parse_v1(
    file: &str,
    object: &serde_json::Map<String, Value>,
) -> Result<MarketStateConfig, ConfigError> {
    read::check_fields(file, ROOT, object, &["schema", "structure_scale"])?;
    let scale = structure_scale(
        file,
        "$.structure_scale",
        read::field(file, ROOT, object, "structure_scale")?,
    )?;
    Ok(MarketStateConfig::new(scale))
}

fn parse_v2(
    file: &str,
    object: &serde_json::Map<String, Value>,
) -> Result<MarketStateConfig, ConfigError> {
    read::check_fields(
        file,
        ROOT,
        object,
        &["schema", "defaults", "timeframes", "structure"],
    )?;

    let defaults_value = read::field(file, ROOT, object, "defaults")?;
    let defaults_object = read::object(file, "$.defaults", defaults_value)?;
    read::check_fields(
        file,
        "$.defaults",
        defaults_object,
        &["ema", "rsi", "atr", "rvol"],
    )?;
    for field in ["ema", "rsi", "atr", "rvol"] {
        read::field(file, "$.defaults", defaults_object, field)?;
    }
    let defaults = readings(
        file,
        "$.defaults",
        defaults_object,
        ReadingsConfig::default(),
    )?;

    let structure_value = read::field(file, ROOT, object, "structure")?;
    let structure_object = read::object(file, "$.structure", structure_value)?;
    read::check_fields(file, "$.structure", structure_object, &["scale"])?;
    let scale = structure_scale(
        file,
        "$.structure.scale",
        read::field(file, "$.structure", structure_object, "scale")?,
    )?;

    let mut config = MarketStateConfig::new(scale);
    let timeframes_value = read::field(file, ROOT, object, "timeframes")?;
    let timeframes_object = read::object(file, "$.timeframes", timeframes_value)?;
    for (code, value) in timeframes_object {
        let path = format!("$.timeframes.{code}");
        let timeframe = Timeframe::from_code(code)
            .filter(|timeframe| timeframe.code() == code)
            .ok_or_else(|| ConfigError::range(file, &path, "a supported timeframe", code))?;
        let override_object = read::object(file, &path, value)?;
        read::check_fields(file, &path, override_object, &["ema", "rsi", "atr", "rvol"])?;
        let override_readings = readings(file, &path, override_object, defaults)?;
        config.set_readings(timeframe, override_readings);
    }

    Ok(config)
}

fn readings(
    file: &str,
    path: &str,
    object: &serde_json::Map<String, Value>,
    inherited: ReadingsConfig,
) -> Result<ReadingsConfig, ConfigError> {
    let mut config = inherited;

    if object.contains_key("ema") {
        let values = read::array(
            file,
            &format!("{path}.ema"),
            read::field(file, path, object, "ema")?,
        )?;
        if values.len() != 3 {
            return Err(ConfigError::range(
                file,
                &format!("{path}.ema"),
                "exactly three periods",
                &read::observed(read::field(file, path, object, "ema")?),
            ));
        }
        let mut periods = [0usize; 3];
        for (index, value) in values.iter().enumerate() {
            let period = read::u64(file, &read::index(&format!("{path}.ema"), index), value)?;
            periods[index] = usize::try_from(period).map_err(|_| {
                ConfigError::range(
                    file,
                    &read::index(&format!("{path}.ema"), index),
                    "a representable period",
                    &period.to_string(),
                )
            })?;
        }
        config.ema_fast = periods[0];
        config.ema_mid = periods[1];
        config.ema_slow = periods[2];
    }

    if object.contains_key("rsi") {
        config.rsi = positive_period(file, path, object, "rsi")?;
    }
    if object.contains_key("atr") {
        config.atr = positive_period(file, path, object, "atr")?;
    }
    if object.contains_key("rvol") {
        config.rvol = positive_period(file, path, object, "rvol")?;
    }

    if !config.is_valid() {
        return Err(ConfigError::range(
            file,
            path,
            "positive periods with strictly increasing EMA periods",
            &format!(
                "ema=[{},{},{}], rsi={}, atr={}, rvol={}",
                config.ema_fast,
                config.ema_mid,
                config.ema_slow,
                config.rsi,
                config.atr,
                config.rvol
            ),
        ));
    }

    Ok(config)
}

fn positive_period(
    file: &str,
    path: &str,
    object: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<usize, ConfigError> {
    let value = read::u64(
        file,
        &format!("{path}.{field}"),
        read::field(file, path, object, field)?,
    )?;
    if value == 0 {
        return Err(ConfigError::range(
            file,
            &format!("{path}.{field}"),
            "> 0",
            "0",
        ));
    }
    usize::try_from(value).map_err(|_| {
        ConfigError::range(
            file,
            &format!("{path}.{field}"),
            "a representable period",
            &value.to_string(),
        )
    })
}

/// Parses the `major` or `minor` structure scale.
pub(super) fn structure_scale(
    file: &str,
    path: &str,
    value: &Value,
) -> Result<PivotScale, ConfigError> {
    let code = read::string(file, path, value)?;
    match code {
        "major" => Ok(PivotScale::Major),
        "minor" => Ok(PivotScale::Minor),
        _ => Err(ConfigError::range(file, path, "major or minor", code)),
    }
}
