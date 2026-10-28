//! `execution.json`: the fill policy and the execution costs.
//!
//! The fill policy's enum values are fixed in v1. The two ATR multiples are
//! carried here because the contract exposes them, and the example fixes them
//! at 2 and 3. The engine's cost config is built from the fee rate and
//! slippage; applying the multiples is the backtest runner's job (#172).

use quantrule_backtest::execution::ExecutionConfig;
use quantrule_data::Timeframe;
use serde_json::Value;

use super::data::timeframe;
use super::error::ConfigError;
use super::read::{self, ROOT};

/// The resolved `execution.json`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ExecutionRunConfig {
    /// The fee and slippage the execution engine applies.
    pub(crate) engine: ExecutionConfig,
    /// The timeframe fills are simulated on; it must equal the strategy
    /// timeframe.
    pub(crate) fill_timeframe: Timeframe,
    /// The stop distance in ATR units.
    pub(crate) stop_atr_multiple: f64,
    /// The target distance in ATR units.
    pub(crate) target_atr_multiple: f64,
}

/// Parses `execution.json`.
pub(crate) fn parse(value: &Value, file: &str) -> Result<ExecutionRunConfig, ConfigError> {
    let object = read::object(file, ROOT, value)?;
    read::check_fields(
        file,
        ROOT,
        object,
        &[
            "schema",
            "fill_timeframe",
            "order_type",
            "stop_atr_multiple",
            "target_atr_multiple",
            "tie_break",
            "fee_rate",
            "slippage_bps",
            "zero_volume",
            "end_of_data",
        ],
    )?;
    read::schema(file, object, "quantrule.execution/v1")?;

    let fill_timeframe = timeframe(
        file,
        "$.fill_timeframe",
        read::field(file, ROOT, object, "fill_timeframe")?,
    )?;

    require_code(
        file,
        "$.order_type",
        read::field(file, ROOT, object, "order_type")?,
        "market_next_open",
    )?;

    let stop_atr_multiple = read::number(
        file,
        "$.stop_atr_multiple",
        read::field(file, ROOT, object, "stop_atr_multiple")?,
    )?;
    if stop_atr_multiple <= 0.0 {
        return Err(ConfigError::range(
            file,
            "$.stop_atr_multiple",
            "> 0",
            &stop_atr_multiple.to_string(),
        ));
    }
    let target_atr_multiple = read::number(
        file,
        "$.target_atr_multiple",
        read::field(file, ROOT, object, "target_atr_multiple")?,
    )?;
    if target_atr_multiple <= 0.0 {
        return Err(ConfigError::range(
            file,
            "$.target_atr_multiple",
            "> 0",
            &target_atr_multiple.to_string(),
        ));
    }

    require_code(
        file,
        "$.tie_break",
        read::field(file, ROOT, object, "tie_break")?,
        "stop_first",
    )?;

    let fee_rate = read::number(
        file,
        "$.fee_rate",
        read::field(file, ROOT, object, "fee_rate")?,
    )?;
    if fee_rate < 0.0 {
        return Err(ConfigError::range(
            file,
            "$.fee_rate",
            ">= 0",
            &fee_rate.to_string(),
        ));
    }

    let slippage_bps = read::number(
        file,
        "$.slippage_bps",
        read::field(file, ROOT, object, "slippage_bps")?,
    )?;
    if slippage_bps < 0.0 || slippage_bps >= 10_000.0 {
        return Err(ConfigError::range(
            file,
            "$.slippage_bps",
            ">= 0 and < 10000",
            &slippage_bps.to_string(),
        ));
    }

    require_code(
        file,
        "$.zero_volume",
        read::field(file, ROOT, object, "zero_volume")?,
        "reject",
    )?;
    require_code(
        file,
        "$.end_of_data",
        read::field(file, ROOT, object, "end_of_data")?,
        "leave_open",
    )?;

    let engine = ExecutionConfig::new(fee_rate, slippage_bps).map_err(|error| {
        ConfigError::range(file, "$", "valid execution costs", &error.to_string())
    })?;

    Ok(ExecutionRunConfig {
        engine,
        fill_timeframe,
        stop_atr_multiple,
        target_atr_multiple,
    })
}

/// Requires a field to be one exact string.
fn require_code(file: &str, path: &str, value: &Value, expected: &str) -> Result<(), ConfigError> {
    let found = read::string(file, path, value)?;
    if found != expected {
        return Err(ConfigError::unsupported(file, path, expected, found));
    }
    Ok(())
}
