//! The `quantrule.model/v4` config parser and effective-config serializer.
//!
//! The v4 config adds event and progression settings on top of the v3 readings.
//! This module keeps the v4-only surface out of the v3 parser so the file stays
//! readable and the historical schema remains untouched.

use quantrule_data::Timeframe;
use quantrule_model::input::{
    DivergenceSettings, EventSettings, LevelEventSettings, MarketStateConfig, ProgressionSettings,
    SignalSettings,
};
use quantrule_model::readings::{PivotScale, ReadingsConfig};
use serde_json::Value;

use super::error::ConfigError;
use super::model::{structure_scale, v3_readings};
use super::read::{self, ROOT};

pub(super) fn parse_v4(
    file: &str,
    object: &serde_json::Map<String, Value>,
) -> Result<MarketStateConfig, ConfigError> {
    read::check_fields(
        file,
        ROOT,
        object,
        &["schema", "defaults", "timeframes", "structure"],
    )?;

    let defaults_object = read::object(
        file,
        "$.defaults",
        read::field(file, ROOT, object, "defaults")?,
    )?;
    read::check_fields(
        file,
        "$.defaults",
        defaults_object,
        &[
            "ema",
            "rsi",
            "atr",
            "rvol",
            "macd",
            "adx",
            "events",
            "progression",
        ],
    )?;
    let (readings, macd_default) = v3_readings(
        file,
        "$.defaults",
        defaults_object,
        ReadingsConfig::default(),
    )?;
    let events = v4_events(
        file,
        "$.defaults.events",
        defaults_object,
        EventSettings::DEFAULT,
        readings,
        ReadingsConfig::default(),
    )?;
    let progression = v4_progression(
        file,
        "$.defaults.progression",
        defaults_object,
        ProgressionSettings::DEFAULT,
    )?;

    let scale = if let Some(structure_value) = object.get("structure") {
        let structure = read::object(file, "$.structure", structure_value)?;
        read::check_fields(file, "$.structure", structure, &["scale"])?;
        structure_scale(
            file,
            "$.structure.scale",
            read::field(file, "$.structure", structure, "scale")?,
        )?
    } else {
        PivotScale::Major
    };

    let mut config = MarketStateConfig::new(scale);
    config.set_max_timeframe_secs(3_600);
    config.set_readings_default(readings);
    config.set_events_default(events);
    config.set_progression_default(progression);
    if let Some(active) = macd_default {
        config.set_macd_default(active);
    }

    if let Some(timeframes_value) = object.get("timeframes") {
        let timeframes_object = read::object(file, "$.timeframes", timeframes_value)?;
        for (code, value) in timeframes_object {
            let path = format!("$.timeframes.{code}");
            let timeframe = Timeframe::from_code(code)
                .filter(|timeframe| timeframe.code() == code)
                .ok_or_else(|| ConfigError::range(file, &path, "a supported timeframe", code))?;
            if u64::from(timeframe.seconds()) > 3_600 {
                return Err(ConfigError::range(
                    file,
                    &path,
                    "a timeframe at or below 1h",
                    code,
                ));
            }
            let override_object = read::object(file, &path, value)?;
            read::check_fields(
                file,
                &path,
                override_object,
                &[
                    "ema",
                    "rsi",
                    "atr",
                    "rvol",
                    "macd",
                    "adx",
                    "pivots",
                    "events",
                    "progression",
                ],
            )?;
            let (override_readings, macd_active) =
                v3_readings(file, &path, override_object, readings)?;
            let override_events = v4_events(
                file,
                &format!("{path}.events"),
                override_object,
                events,
                override_readings,
                readings,
            )?;
            let override_progression = v4_progression(
                file,
                &format!("{path}.progression"),
                override_object,
                progression,
            )?;
            config.set_readings(timeframe, override_readings);
            config.set_events(timeframe, override_events);
            config.set_progression(timeframe, override_progression);
            if let Some(active) = macd_active {
                config.set_macd_enabled(timeframe, active);
            }
        }
    }

    Ok(config)
}

fn v4_events(
    file: &str,
    path: &str,
    parent: &serde_json::Map<String, Value>,
    inherited: EventSettings,
    readings: ReadingsConfig,
    inherited_readings: ReadingsConfig,
) -> Result<EventSettings, ConfigError> {
    let Some(value) = parent.get("events") else {
        let mut settings = inherited;
        if readings != inherited_readings {
            settings.signal.ema_pairs = role_pairs(readings);
        }
        return Ok(settings);
    };
    let events = read::object(file, path, value)?;
    read::check_fields(file, path, events, &["level", "divergence", "signal"])?;
    let mut settings = inherited;
    if let Some(level_value) = events.get("level") {
        settings.level = v4_level(file, &format!("{path}.level"), level_value, settings.level)?;
    }
    if let Some(divergence_value) = events.get("divergence") {
        settings.divergence = v4_divergence(
            file,
            &format!("{path}.divergence"),
            divergence_value,
            settings.divergence,
        )?;
    }
    let explicit_pairs = events
        .get("signal")
        .and_then(Value::as_object)
        .is_some_and(|signal| signal.contains_key("ema_pairs"));
    if let Some(signal_value) = events.get("signal") {
        settings.signal = v4_signal(
            file,
            &format!("{path}.signal"),
            signal_value,
            settings.signal,
        )?;
    }
    if explicit_pairs {
        validate_ema_pairs(
            file,
            &format!("{path}.signal.ema_pairs"),
            settings.signal,
            readings,
        )?;
    } else if readings != inherited_readings {
        settings.signal.ema_pairs = role_pairs(readings);
    }
    Ok(settings)
}

fn role_pairs(readings: ReadingsConfig) -> [(usize, usize); 3] {
    [
        (readings.ema_fast, readings.ema_mid),
        (readings.ema_mid, readings.ema_slow),
        (readings.ema_fast, readings.ema_slow),
    ]
}

fn validate_ema_pairs(
    file: &str,
    path: &str,
    settings: SignalSettings,
    readings: ReadingsConfig,
) -> Result<(), ConfigError> {
    let allowed = [readings.ema_fast, readings.ema_mid, readings.ema_slow];
    for (index, (fast, slow)) in settings.ema_pairs.iter().enumerate() {
        if !allowed.contains(fast) || !allowed.contains(slow) {
            return Err(ConfigError::range(
                file,
                &format!("{path}[{index}]"),
                "periods from the configured EMA triple",
                &format!("{fast}/{slow}"),
            ));
        }
    }
    Ok(())
}

fn v4_level(
    file: &str,
    path: &str,
    value: &Value,
    inherited: LevelEventSettings,
) -> Result<LevelEventSettings, ConfigError> {
    let level = read::object(file, path, value)?;
    read::check_fields(file, path, level, &["references", "test_atr", "break_atr"])?;
    let mut settings = inherited;

    if let Some(references_value) = level.get("references") {
        let references_path = format!("{path}.references");
        let references = read::array(file, &references_path, references_value)?;
        let mut ema = false;
        let mut pivots = false;
        for (index, reference) in references.iter().enumerate() {
            let reference = read::string(file, &read::index(&references_path, index), reference)?;
            match reference {
                "ema" if !ema => ema = true,
                "pivots" if !pivots => pivots = true,
                "ema" | "pivots" => {
                    return Err(ConfigError::inconsistent(
                        file,
                        &read::index(&references_path, index),
                        "unique level references",
                        reference,
                    ));
                }
                other => {
                    return Err(ConfigError::unsupported(
                        file,
                        &read::index(&references_path, index),
                        "ema or pivots",
                        other,
                    ));
                }
            }
        }
        if !ema && !pivots {
            return Err(ConfigError::range(
                file,
                &references_path,
                "at least one reference",
                "empty",
            ));
        }
        settings.ema = ema;
        settings.pivots = pivots;
    }

    if let Some(test_value) = level.get("test_atr") {
        settings.test_atr = positive_number(file, &format!("{path}.test_atr"), test_value)?;
    }
    if let Some(break_value) = level.get("break_atr") {
        settings.break_atr = positive_number(file, &format!("{path}.break_atr"), break_value)?;
    }
    Ok(settings)
}

fn v4_divergence(
    file: &str,
    path: &str,
    value: &Value,
    inherited: DivergenceSettings,
) -> Result<DivergenceSettings, ConfigError> {
    let divergence = read::object(file, path, value)?;
    read::check_fields(
        file,
        path,
        divergence,
        &["indicator", "pivot_scale", "window"],
    )?;
    let mut settings = inherited;
    if let Some(indicator) = divergence.get("indicator") {
        require_code(file, &format!("{path}.indicator"), indicator, "rsi")?;
    }
    if let Some(scale_value) = divergence.get("pivot_scale") {
        let scale = read::string(file, &format!("{path}.pivot_scale"), scale_value)?;
        settings.scale = match scale {
            "minor" => PivotScale::Minor,
            "major" => PivotScale::Major,
            other => {
                return Err(ConfigError::unsupported(
                    file,
                    &format!("{path}.pivot_scale"),
                    "minor or major",
                    other,
                ));
            }
        };
    }
    if let Some(window_value) = divergence.get("window") {
        let window = read::u64(file, &format!("{path}.window"), window_value)?;
        settings.window = usize::try_from(window).map_err(|_| {
            ConfigError::range(
                file,
                &format!("{path}.window"),
                "a representable bar count",
                &window.to_string(),
            )
        })?;
    }
    Ok(settings)
}

fn v4_signal(
    file: &str,
    path: &str,
    value: &Value,
    inherited: SignalSettings,
) -> Result<SignalSettings, ConfigError> {
    let signal = read::object(file, path, value)?;
    read::check_fields(
        file,
        path,
        signal,
        &[
            "rsi_levels",
            "ema_pairs",
            "macd",
            "di_cross",
            "trend_continuation",
            "compression_breakout",
            "compression_range",
            "compression_range_onset",
        ],
    )?;
    let mut settings = inherited;

    if let Some(levels_value) = signal.get("rsi_levels") {
        let levels_path = format!("{path}.rsi_levels");
        let rsi_levels = read::array(file, &levels_path, levels_value)?;
        if rsi_levels.len() != 3 {
            return Err(ConfigError::range(
                file,
                &levels_path,
                "exactly three levels",
                &rsi_levels.len().to_string(),
            ));
        }
        let mut levels = [0.0; 3];
        for (index, value) in rsi_levels.iter().enumerate() {
            levels[index] = read::number(file, &read::index(&levels_path, index), value)?;
            if !(0.0..=100.0).contains(&levels[index]) {
                return Err(ConfigError::range(
                    file,
                    &read::index(&levels_path, index),
                    "a level in [0, 100]",
                    &levels[index].to_string(),
                ));
            }
        }
        settings.rsi_levels = levels;
    }

    if let Some(pairs_value) = signal.get("ema_pairs") {
        let pairs_path = format!("{path}.ema_pairs");
        let ema_pairs = read::array(file, &pairs_path, pairs_value)?;
        if ema_pairs.len() != 3 {
            return Err(ConfigError::range(
                file,
                &pairs_path,
                "exactly three pairs",
                &ema_pairs.len().to_string(),
            ));
        }
        let mut pairs = [(0usize, 0usize); 3];
        for (index, pair) in ema_pairs.iter().enumerate() {
            let pair = read::array(file, &read::index(&pairs_path, index), pair)?;
            if pair.len() != 2 {
                return Err(ConfigError::range(
                    file,
                    &read::index(&pairs_path, index),
                    "a two-period pair",
                    &pair.len().to_string(),
                ));
            }
            let fast = period(file, &read::index(&pairs_path, index), &pair[0])?;
            let slow = period(file, &read::index(&pairs_path, index), &pair[1])?;
            if fast >= slow {
                return Err(ConfigError::range(
                    file,
                    &read::index(&pairs_path, index),
                    "a fast period below the slow period",
                    &format!("{fast}/{slow}"),
                ));
            }
            pairs[index] = (fast, slow);
        }
        settings.ema_pairs = pairs;
    }

    if let Some(macd_value) = signal.get("macd") {
        let macd_path = format!("{path}.macd");
        let macd_values = read::array(file, &macd_path, macd_value)?;
        let mut macd_line_signal_cross = false;
        let mut macd_histogram_slope_flip = false;
        let mut macd_zero_cross = false;
        for (index, value) in macd_values.iter().enumerate() {
            let value = read::string(file, &read::index(&macd_path, index), value)?;
            let flag = match value {
                "line_signal_cross" => &mut macd_line_signal_cross,
                "histogram_slope_flip" => &mut macd_histogram_slope_flip,
                "zero_cross" => &mut macd_zero_cross,
                other => {
                    return Err(ConfigError::unsupported(
                        file,
                        &read::index(&macd_path, index),
                        "a registered MACD family",
                        other,
                    ));
                }
            };
            if *flag {
                return Err(ConfigError::inconsistent(
                    file,
                    &read::index(&macd_path, index),
                    "unique MACD families",
                    value,
                ));
            }
            *flag = true;
        }
        settings.macd_line_signal_cross = macd_line_signal_cross;
        settings.macd_histogram_slope_flip = macd_histogram_slope_flip;
        settings.macd_zero_cross = macd_zero_cross;
    }

    if let Some(di_cross) = signal.get("di_cross") {
        settings.di_cross = read::boolean(file, &format!("{path}.di_cross"), di_cross)?;
    }

    if let Some(trend_value) = signal.get("trend_continuation") {
        settings = v4_trend(
            file,
            &format!("{path}.trend_continuation"),
            trend_value,
            settings,
        )?;
    }

    if let Some(compression_value) = signal.get("compression_breakout") {
        settings = v4_compression(
            file,
            &format!("{path}.compression_breakout"),
            compression_value,
            settings,
        )?;
    }

    if let Some(range_value) = signal.get("compression_range") {
        settings = v4_compression_range(
            file,
            &format!("{path}.compression_range"),
            range_value,
            settings,
        )?;
    }

    if let Some(onset_value) = signal.get("compression_range_onset") {
        settings = v4_compression_onset(
            file,
            &format!("{path}.compression_range_onset"),
            onset_value,
            settings,
        )?;
    }
    Ok(settings)
}

fn v4_compression(
    file: &str,
    path: &str,
    value: &Value,
    inherited: SignalSettings,
) -> Result<SignalSettings, ConfigError> {
    let compression = read::object(file, path, value)?;
    read::check_fields(
        file,
        path,
        compression,
        &["enabled", "coiling_bars", "break_atr", "rvol_min"],
    )?;
    let mut settings = inherited;
    if let Some(enabled) = compression.get("enabled") {
        settings.compression_breakout = read::boolean(file, &format!("{path}.enabled"), enabled)?;
    }
    if let Some(coiling) = compression.get("coiling_bars") {
        settings.compression_coiling_bars =
            positive_usize(file, &format!("{path}.coiling_bars"), coiling)?;
    }
    if let Some(break_atr) = compression.get("break_atr") {
        settings.compression_break_atr =
            positive_number(file, &format!("{path}.break_atr"), break_atr)?;
    }
    if let Some(rvol_min) = compression.get("rvol_min") {
        settings.compression_rvol_min =
            positive_number(file, &format!("{path}.rvol_min"), rvol_min)?;
    }
    Ok(settings)
}

fn v4_compression_range(
    file: &str,
    path: &str,
    value: &Value,
    inherited: SignalSettings,
) -> Result<SignalSettings, ConfigError> {
    let range = read::object(file, path, value)?;
    read::check_fields(file, path, range, &["enabled", "coiling_bars"])?;
    let mut settings = inherited;
    if let Some(enabled) = range.get("enabled") {
        settings.compression_range = read::boolean(file, &format!("{path}.enabled"), enabled)?;
    }
    if let Some(coiling) = range.get("coiling_bars") {
        settings.compression_range_coiling_bars =
            positive_usize(file, &format!("{path}.coiling_bars"), coiling)?;
    }
    Ok(settings)
}

fn v4_compression_onset(
    file: &str,
    path: &str,
    value: &Value,
    inherited: SignalSettings,
) -> Result<SignalSettings, ConfigError> {
    let onset = read::object(file, path, value)?;
    read::check_fields(file, path, onset, &["enabled", "coiling_bars"])?;
    let mut settings = inherited;
    if let Some(enabled) = onset.get("enabled") {
        settings.compression_range_onset =
            read::boolean(file, &format!("{path}.enabled"), enabled)?;
    }
    if let Some(coiling) = onset.get("coiling_bars") {
        settings.compression_range_onset_coiling_bars =
            positive_usize(file, &format!("{path}.coiling_bars"), coiling)?;
    }
    Ok(settings)
}

fn v4_trend(
    file: &str,
    path: &str,
    value: &Value,
    inherited: SignalSettings,
) -> Result<SignalSettings, ConfigError> {
    let trend = read::object(file, path, value)?;
    read::check_fields(
        file,
        path,
        trend,
        &["enabled", "adx_threshold", "adx_slope_bars"],
    )?;
    let mut settings = inherited;
    if let Some(enabled) = trend.get("enabled") {
        settings.trend_continuation = read::boolean(file, &format!("{path}.enabled"), enabled)?;
    }
    if let Some(threshold) = trend.get("adx_threshold") {
        settings.trend_adx_threshold =
            positive_number(file, &format!("{path}.adx_threshold"), threshold)?;
    }
    if let Some(slope) = trend.get("adx_slope_bars") {
        settings.trend_adx_slope_bars =
            positive_usize(file, &format!("{path}.adx_slope_bars"), slope)?;
    }
    Ok(settings)
}

fn v4_progression(
    file: &str,
    path: &str,
    parent: &serde_json::Map<String, Value>,
    inherited: ProgressionSettings,
) -> Result<ProgressionSettings, ConfigError> {
    let Some(value) = parent.get("progression") else {
        return Ok(inherited);
    };
    let progression = read::object(file, path, value)?;
    read::check_fields(
        file,
        path,
        progression,
        &[
            "target_atr",
            "stability_bars",
            "invalidation_atr",
            "min_target_atr",
        ],
    )?;
    let mut settings = inherited;
    if let Some(target) = progression.get("target_atr") {
        settings.target_atr = positive_number(file, &format!("{path}.target_atr"), target)?;
    }
    if let Some(stability) = progression.get("stability_bars") {
        settings.stability_bars =
            positive_usize(file, &format!("{path}.stability_bars"), stability)?;
    }
    if let Some(invalidation) = progression.get("invalidation_atr") {
        settings.invalidation_atr =
            positive_number(file, &format!("{path}.invalidation_atr"), invalidation)?;
    }
    if let Some(minimum) = progression.get("min_target_atr") {
        settings.min_target_atr =
            positive_number(file, &format!("{path}.min_target_atr"), minimum)?;
    }
    Ok(settings)
}

fn require_code(file: &str, path: &str, value: &Value, expected: &str) -> Result<(), ConfigError> {
    let found = read::string(file, path, value)?;
    if found != expected {
        return Err(ConfigError::unsupported(file, path, expected, found));
    }
    Ok(())
}

fn positive_number(file: &str, path: &str, value: &Value) -> Result<f64, ConfigError> {
    let number = read::number(file, path, value)?;
    if number <= 0.0 {
        return Err(ConfigError::range(file, path, "> 0", &number.to_string()));
    }
    Ok(number)
}

fn positive_usize(file: &str, path: &str, value: &Value) -> Result<usize, ConfigError> {
    let number = read::u64(file, path, value)?;
    if number == 0 {
        return Err(ConfigError::range(file, path, "> 0", "0"));
    }
    usize::try_from(number).map_err(|_| {
        ConfigError::range(file, path, "a representable bar count", &number.to_string())
    })
}

fn period(file: &str, path: &str, value: &Value) -> Result<usize, ConfigError> {
    positive_usize(file, path, value)
}

/// Serializes the effective v4 model configuration.
#[must_use]
pub fn effective_json(config: &MarketStateConfig) -> Value {
    let mut timeframes = serde_json::Map::new();
    for timeframe in Timeframe::ALL {
        let seconds = u64::from(timeframe.seconds());
        if seconds > 3_600 {
            continue;
        }
        let readings = config.readings_for(seconds);
        let events = config.events_for(seconds);
        let progression = config.progression_for(seconds);
        let mut level_references = Vec::new();
        if events.level.ema {
            level_references.push("ema");
        }
        if events.level.pivots {
            level_references.push("pivots");
        }
        timeframes.insert(
            timeframe.code().to_owned(),
            serde_json::json!({
                "ema": {"periods": [readings.ema_fast, readings.ema_mid, readings.ema_slow], "active": readings.ema_active},
                "rsi": {"period": readings.rsi, "active": readings.rsi_active},
                "atr": {"period": readings.atr, "active": readings.atr_active},
                "rvol": {"period": readings.rvol, "active": readings.rvol_active},
                "macd": {
                    "fast": readings.macd_fast,
                    "slow": readings.macd_slow,
                    "signal": readings.macd_signal,
                    "active": config.macd_enabled_for(seconds)
                },
                "adx": {
                    "period": readings.adx,
                    "threshold": readings.adx_threshold,
                    "slope_bars": readings.adx_slope_bars,
                    "active": readings.adx_active
                },
                "events": {
                    "level": {
                        "references": level_references,
                        "test_atr": events.level.test_atr,
                        "break_atr": events.level.break_atr
                    },
                    "divergence": {
                        "indicator": "rsi",
                        "pivot_scale": match events.divergence.scale { PivotScale::Minor => "minor", PivotScale::Major => "major" },
                        "window": events.divergence.window
                    },
                    "signal": {
                        "rsi_levels": events.signal.rsi_levels,
                        "ema_pairs": events.signal.ema_pairs,
                        "macd_line_signal_cross": events.signal.macd_line_signal_cross,
                        "macd_histogram_slope_flip": events.signal.macd_histogram_slope_flip,
                        "macd_zero_cross": events.signal.macd_zero_cross,
                        "di_cross": events.signal.di_cross,
                        "trend_continuation": {
                            "enabled": events.signal.trend_continuation,
                            "adx_threshold": events.signal.trend_adx_threshold,
                            "adx_slope_bars": events.signal.trend_adx_slope_bars
                        },
                        "compression_breakout": {
                            "enabled": events.signal.compression_breakout,
                            "coiling_bars": events.signal.compression_coiling_bars,
                            "break_atr": events.signal.compression_break_atr,
                            "rvol_min": events.signal.compression_rvol_min
                        },
                        "compression_range": {
                            "enabled": events.signal.compression_range,
                            "coiling_bars": events.signal.compression_range_coiling_bars
                        },
                        "compression_range_onset": {
                            "enabled": events.signal.compression_range_onset,
                            "coiling_bars": events.signal.compression_range_onset_coiling_bars
                        }
                    }
                },
                "progression": {
                    "target_atr": progression.target_atr,
                    "stability_bars": progression.stability_bars,
                    "invalidation_atr": progression.invalidation_atr,
                    "min_target_atr": progression.min_target_atr
                }
            }),
        );
    }
    serde_json::json!({
        "schema": "quantrule.model/v4",
        "structure_scale": match config.structure_scale { PivotScale::Minor => "minor", PivotScale::Major => "major" },
        "timeframes": timeframes
    })
}
