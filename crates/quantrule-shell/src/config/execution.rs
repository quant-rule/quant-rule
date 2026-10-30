//! `execution.json`: the fill mechanics and execution costs.
//!
//! Execution owns how a decision becomes a fill: the fill timeframe, the order
//! type, the tie rule, slippage, fees, and the zero-volume/end-of-data policy.
//! It does not own the strategy's risk/reward. The bracket distances arrive from
//! `strategy.json` as a `TradeIntent` and are resolved by the execution engine,
//! not stored in this file.

use quantrule_backtest::execution::ExecutionConfig;
use quantrule_data::Timeframe;
use serde_json::Value;

use super::data::timeframe;
use super::error::ConfigError;
use super::read::{self, ROOT};

/// The resolved `execution.json`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ExecutionRunConfig {
    /// The costs and fill mechanics.
    pub(crate) engine: ExecutionConfig,
    /// The timeframe fills are simulated on; it must equal the strategy
    /// timeframe.
    pub(crate) fill_timeframe: Timeframe,
}

/// Parses `execution.json`.
///
/// The file owns costs and mechanics only. Stop and target distances are part
/// of the strategy's risk intent; the execution file must not duplicate them.
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
            "tie_break",
            "fee_rate",
            "slippage_bps",
            "zero_volume",
            "end_of_data",
        ],
    )?;
    read::schema(file, object, "quantrule.execution/v2")?;

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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::parse;
    use crate::config::error::ConfigErrorKind;

    fn valid() -> serde_json::Value {
        json!({
            "schema": "quantrule.execution/v2",
            "fill_timeframe": "15m",
            "order_type": "market_next_open",
            "tie_break": "stop_first",
            "fee_rate": 0.0,
            "slippage_bps": 2.0,
            "zero_volume": "reject",
            "end_of_data": "leave_open"
        })
    }

    #[test]
    fn parses_the_fill_mechanics() {
        let execution = parse(&valid(), "execution.json").expect("the file is valid");
        assert_eq!(execution.engine.fee_rate(), 0.0);
        assert_eq!(execution.engine.slippage_bps(), 2.0);
        assert_eq!(execution.fill_timeframe.code(), "15m");
    }

    #[test]
    fn rejects_the_old_bracket_fields() {
        let mut value = valid();
        value["stop_atr_multiple"] = json!(2.0);
        assert_eq!(
            parse(&value, "execution.json")
                .expect_err("the bracket moved to the strategy")
                .kind(),
            ConfigErrorKind::UnknownField
        );

        let mut value = valid();
        value["target_atr_multiple"] = json!(3.0);
        assert_eq!(
            parse(&value, "execution.json")
                .expect_err("the bracket moved to the strategy")
                .kind(),
            ConfigErrorKind::UnknownField
        );
    }
}
