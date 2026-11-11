//! `model.json`: the per-timeframe reading configuration.
//!
//! The current schema is `quantrule.model/v4`; its parser lives in
//! [`super::model_v4`]. The v1, v2, and v3 schemas are retired: they fail with
//! an explicit unsupported-schema error instead of being migrated silently.

use quantrule_model::input::MarketStateConfig;
use quantrule_model::readings::{PivotScale, ReadingsConfig};
use serde_json::Value;

use super::error::ConfigError;
use super::model_v4::parse_v4;
use super::read::{self, ROOT};

/// Parses `model.json`.
pub fn parse(value: &Value, file: &str) -> Result<MarketStateConfig, ConfigError> {
    let object = read::object(file, ROOT, value)?;
    let schema = read::string(file, "$.schema", read::field(file, ROOT, object, "schema")?)?;
    match schema {
        "quantrule.model/v4" => parse_v4(file, object),
        _ => Err(ConfigError::unsupported(
            file,
            "$.schema",
            "quantrule.model/v4",
            schema,
        )),
    }
}

/// Parses the v3 indicator block. The caller owns the schema-version switch.
pub(super) fn v3_readings(
    file: &str,
    path: &str,
    object: &serde_json::Map<String, Value>,
    inherited: ReadingsConfig,
) -> Result<(ReadingsConfig, Option<bool>), ConfigError> {
    let mut config = inherited;
    let mut macd_active = None;

    if let Some(ema_value) = object.get("ema") {
        let ema_path = format!("{path}.ema");
        let ema = read::object(file, &ema_path, ema_value)?;
        read::check_fields(file, &ema_path, ema, &["periods", "active"])?;
        let values = read::array(
            file,
            &format!("{ema_path}.periods"),
            read::field(file, &ema_path, ema, "periods")?,
        )?;
        if values.len() != 3 {
            return Err(ConfigError::range(
                file,
                &format!("{ema_path}.periods"),
                "exactly three periods",
                &read::observed(read::field(file, &ema_path, ema, "periods")?),
            ));
        }
        let mut periods = [0usize; 3];
        for (index, value) in values.iter().enumerate() {
            let period = read::u64(
                file,
                &read::index(&format!("{ema_path}.periods"), index),
                value,
            )?;
            periods[index] = usize::try_from(period).map_err(|_| {
                ConfigError::range(
                    file,
                    &read::index(&format!("{ema_path}.periods"), index),
                    "a representable period",
                    &period.to_string(),
                )
            })?;
        }
        config.ema_fast = periods[0];
        config.ema_mid = periods[1];
        config.ema_slow = periods[2];
        config.ema_active = v3_active(file, &ema_path, ema)?;
    }

    if object.contains_key("rsi") {
        let path = format!("{path}.rsi");
        let rsi = read::object(
            file,
            &path,
            read::field(file, path.as_str(), object, "rsi")?,
        )?;
        read::check_fields(file, &path, rsi, &["period", "active"])?;
        config.rsi = positive_period(file, &path, rsi, "period")?;
        config.rsi_active = v3_active(file, &path, rsi)?;
    }
    if object.contains_key("atr") {
        let path = format!("{path}.atr");
        let atr = read::object(
            file,
            &path,
            read::field(file, path.as_str(), object, "atr")?,
        )?;
        read::check_fields(file, &path, atr, &["period", "active"])?;
        config.atr = positive_period(file, &path, atr, "period")?;
        config.atr_active = v3_active(file, &path, atr)?;
    }
    if object.contains_key("rvol") {
        let path = format!("{path}.rvol");
        let rvol = read::object(
            file,
            &path,
            read::field(file, path.as_str(), object, "rvol")?,
        )?;
        read::check_fields(file, &path, rvol, &["period", "active"])?;
        config.rvol = positive_period(file, &path, rvol, "period")?;
        config.rvol_active = v3_active(file, &path, rvol)?;
    }
    if object.contains_key("macd") {
        let path = format!("{path}.macd");
        let macd = read::object(
            file,
            &path,
            read::field(file, path.as_str(), object, "macd")?,
        )?;
        read::check_fields(file, &path, macd, &["fast", "slow", "signal", "active"])?;
        let fast = positive_period(file, &path, macd, "fast")?;
        let slow = positive_period(file, &path, macd, "slow")?;
        let signal = positive_period(file, &path, macd, "signal")?;
        config = config.with_macd(fast, slow, signal);
        macd_active = Some(v3_active(file, &path, macd)?);
    }
    if object.contains_key("adx") {
        let path = format!("{path}.adx");
        let adx = read::object(
            file,
            &path,
            read::field(file, path.as_str(), object, "adx")?,
        )?;
        read::check_fields(
            file,
            &path,
            adx,
            &["period", "threshold", "slope_bars", "active"],
        )?;
        let period = positive_period(file, &path, adx, "period")?;
        let threshold = read::number(
            file,
            &format!("{path}.threshold"),
            read::field(file, &path, adx, "threshold")?,
        )?;
        if threshold <= 0.0 {
            return Err(ConfigError::range(
                file,
                &format!("{path}.threshold"),
                "> 0",
                &threshold.to_string(),
            ));
        }
        let slope = read::u64(
            file,
            &format!("{path}.slope_bars"),
            read::field(file, &path, adx, "slope_bars")?,
        )?;
        if slope == 0 {
            return Err(ConfigError::range(
                file,
                &format!("{path}.slope_bars"),
                "> 0",
                "0",
            ));
        }
        let slope_bars = usize::try_from(slope).map_err(|_| {
            ConfigError::range(
                file,
                &format!("{path}.slope_bars"),
                "a representable bar count",
                &slope.to_string(),
            )
        })?;
        config = config.with_adx(period, threshold, slope_bars);
        config.adx_active = v3_active(file, &path, adx)?;
    }

    if !config.is_valid() {
        return Err(ConfigError::range(
            file,
            path,
            "positive periods with strictly increasing EMA periods and a positive ADX threshold",
            &format!(
                "ema=[{},{},{}], rsi={}, atr={}, rvol={}, macd={}/{}/{}, adx={}/{}",
                config.ema_fast,
                config.ema_mid,
                config.ema_slow,
                config.rsi,
                config.atr,
                config.rvol,
                config.macd_fast,
                config.macd_slow,
                config.macd_signal,
                config.adx,
                config.adx_threshold
            ),
        ));
    }

    Ok((config, macd_active))
}

fn v3_active(
    file: &str,
    path: &str,
    object: &serde_json::Map<String, Value>,
) -> Result<bool, ConfigError> {
    match object.get("active") {
        Some(value) => read::boolean(file, &format!("{path}.active"), value),
        None => Ok(true),
    }
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
