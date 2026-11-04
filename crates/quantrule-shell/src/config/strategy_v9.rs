//! The `quantrule.strategy/v9` compression-breakout parser.
//!
//! The v9 schema is a compact rule definition: one published event, a time
//! stop, and an ATR/reward-to-risk policy. It reuses the common thesis, ladder,
//! and no-trade parsing from the parent module.

use quantrule_backtest::context::LadderConfig;
use quantrule_backtest::strategy::StrategyConfig;
use serde_json::{Map, Value};

use super::data::timeframe;
use super::error::ConfigError;
use super::read::{self, ROOT};
use super::strategy::{Risk, Schema, StrategyRunConfig, parse_risk};

pub(super) fn parse_v9(
    file: &str,
    object: &Map<String, Value>,
    ladder: LadderConfig,
    require_contiguous_execution: bool,
    thesis: String,
) -> Result<StrategyRunConfig, ConfigError> {
    let event_path = "$.event";
    let event = read::object(file, event_path, read::field(file, ROOT, object, "event")?)?;
    read::check_fields(
        file,
        event_path,
        event,
        &["timeframe", "event_type", "subtype", "source"],
    )?;
    let event_timeframe = timeframe(
        file,
        "$.event.timeframe",
        read::field(file, event_path, event, "timeframe")?,
    )?;
    if u64::from(event_timeframe.seconds()) != ladder.execution_timeframe_secs() {
        return Err(ConfigError::inconsistent(
            file,
            "$.event.timeframe",
            "the strategy execution timeframe",
            event_timeframe.code(),
        ));
    }
    let event_type = read::u64(
        file,
        "$.event.event_type",
        read::field(file, event_path, event, "event_type")?,
    )?;
    let event_type = u8::try_from(event_type).map_err(|_| {
        ConfigError::range(
            file,
            "$.event.event_type",
            "an event type code",
            &event_type.to_string(),
        )
    })?;
    let subtype = read::u64(
        file,
        "$.event.subtype",
        read::field(file, event_path, event, "subtype")?,
    )?;
    let subtype = u8::try_from(subtype).map_err(|_| {
        ConfigError::range(
            file,
            "$.event.subtype",
            "an event subtype code",
            &subtype.to_string(),
        )
    })?;
    let source = read::u64(
        file,
        "$.event.source",
        read::field(file, event_path, event, "source")?,
    )?;
    let source = u16::try_from(source).map_err(|_| {
        ConfigError::range(
            file,
            "$.event.source",
            "an event source code",
            &source.to_string(),
        )
    })?;
    let window_bars = read::u64(
        file,
        "$.window_bars",
        read::field(file, ROOT, object, "window_bars")?,
    )?;
    let window_bars = usize::try_from(window_bars).map_err(|_| {
        ConfigError::range(
            file,
            "$.window_bars",
            "a representable bar count",
            &window_bars.to_string(),
        )
    })?;
    let time_stop_bars = read::u64(
        file,
        "$.time_stop_bars",
        read::field(file, ROOT, object, "time_stop_bars")?,
    )?;
    let risk = match parse_risk(file, read::field(file, ROOT, object, "risk")?, Schema::V9)? {
        Risk::Atr(risk) => risk,
        Risk::Level { .. } => {
            return Err(ConfigError::range(
                file,
                "$.risk",
                "an ATR/reward-to-risk policy",
                "a level bracket",
            ));
        }
    };
    let compression = quantrule_backtest::strategy::CompressionBreakoutConfig::new(
        u64::from(event_timeframe.seconds()),
        event_type,
        subtype,
        source,
        window_bars,
        time_stop_bars,
    )
    .map_err(|error| {
        ConfigError::range(
            file,
            "$.event",
            "a valid compression-breakout configuration",
            &error.to_string(),
        )
    })?;
    let engine = StrategyConfig::new_compression_breakout(
        ladder,
        compression,
        risk,
        require_contiguous_execution,
    )
    .map_err(|error| {
        ConfigError::range(
            file,
            "$.event",
            "a valid compression-breakout configuration",
            &error.to_string(),
        )
    })?;
    Ok(StrategyRunConfig { engine, thesis })
}
