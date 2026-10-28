//! `strategy.json`: the axis-agreement thresholds and timeframe.
//!
//! The strategy is the only engine whose thresholds are tunable in v1. The
//! thresholds must straddle zero so that a long classification always means a
//! strictly positive reading and a short classification a strictly negative
//! one. The timeframe is checked against the data ladder in the composition
//! root, where both files are available.

use quantrule_backtest::strategy::StrategyConfig;
use serde_json::Value;

use super::data::timeframe;
use super::error::ConfigError;
use super::read::{self, ROOT};

/// The only strategy name accepted in v1.
const NAME: &str = "axis-agreement";

/// Parses `strategy.json`.
pub(crate) fn parse(value: &Value, file: &str) -> Result<StrategyConfig, ConfigError> {
    let object = read::object(file, ROOT, value)?;
    read::check_fields(
        file,
        ROOT,
        object,
        &[
            "schema",
            "name",
            "timeframe",
            "long_threshold",
            "short_threshold",
        ],
    )?;
    read::schema(file, object, "quantrule.strategy/v1")?;

    let name = read::string(file, "$.name", read::field(file, ROOT, object, "name")?)?;
    if name != NAME {
        return Err(ConfigError::unsupported(file, "$.name", NAME, name));
    }

    let timeframe = timeframe(
        file,
        "$.timeframe",
        read::field(file, ROOT, object, "timeframe")?,
    )?;
    let long_threshold = read::number(
        file,
        "$.long_threshold",
        read::field(file, ROOT, object, "long_threshold")?,
    )?;
    let short_threshold = read::number(
        file,
        "$.short_threshold",
        read::field(file, ROOT, object, "short_threshold")?,
    )?;

    if long_threshold < 0.0 {
        return Err(ConfigError::range(
            file,
            "$.long_threshold",
            ">= 0",
            &long_threshold.to_string(),
        ));
    }
    if short_threshold > 0.0 {
        return Err(ConfigError::range(
            file,
            "$.short_threshold",
            "<= 0",
            &short_threshold.to_string(),
        ));
    }
    if long_threshold <= short_threshold {
        return Err(ConfigError::inconsistent(
            file,
            "$.long_threshold",
            "a value greater than $.short_threshold",
            &long_threshold.to_string(),
        ));
    }

    StrategyConfig::new(u64::from(timeframe.seconds()), long_threshold, short_threshold).map_err(
        |error| {
            ConfigError::range(
                file,
                "$",
                "a valid strategy configuration",
                &error.to_string(),
            )
        },
    )
}
