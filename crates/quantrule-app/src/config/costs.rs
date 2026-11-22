//! The cost documents shared by a run configuration and a pack.
//!
//! Execution fees and slippage are validated by the engine's
//! [`ExecutionConfig`]; commission and funding by the portfolio's
//! [`CommissionConfig`] and [`FundingConfig`]. This module owns the JSON shape
//! and the field-level bounds so the run files and the execution-simulation
//! pack cannot drift. Each function takes the document path, so a rejection
//! names the field in whichever document carried it.

use quantrule_backtest::execution::ExecutionConfig;
use quantrule_backtest::portfolio::{
    CommissionBase, CommissionConfig, CommissionPer, FundingConfig, FundingInterval,
};
use serde_json::{Map, Value};

use super::error::ConfigError;
use super::read;

/// Parses and validates the execution fee and slippage fields.
///
/// `path` is the object that carries `fee_rate` and `slippage_bps`; it is the
/// run root or the pack payload. Slippage is bounded below one hundred percent
/// so a cost assumption cannot silently exceed the trade notional.
pub(crate) fn execution(
    file: &str,
    path: &str,
    object: &Map<String, Value>,
) -> Result<ExecutionConfig, ConfigError> {
    let fee_path = read::child(path, "fee_rate");
    let fee_rate = read::number(
        file,
        &fee_path,
        read::field(file, path, object, "fee_rate")?,
    )?;
    if fee_rate < 0.0 {
        return Err(ConfigError::range(
            file,
            &fee_path,
            ">= 0",
            &fee_rate.to_string(),
        ));
    }

    let slippage_path = read::child(path, "slippage_bps");
    let slippage_bps = read::number(
        file,
        &slippage_path,
        read::field(file, path, object, "slippage_bps")?,
    )?;
    if slippage_bps < 0.0 || slippage_bps >= 10_000.0 {
        return Err(ConfigError::range(
            file,
            &slippage_path,
            ">= 0 and < 10000",
            &slippage_bps.to_string(),
        ));
    }

    ExecutionConfig::new(fee_rate, slippage_bps).map_err(|error| {
        ConfigError::range(file, path, "valid execution costs", &error.to_string())
    })
}

/// Parses and validates a commission object.
///
/// The rate is a fraction of equity and must be finite and non-negative. The
/// base and period are closed codes so a statistical or notional-based charge
/// has no representation.
pub(crate) fn commission(
    file: &str,
    path: &str,
    value: &Value,
) -> Result<CommissionConfig, ConfigError> {
    let object = read::object(file, path, value)?;
    read::check_fields(file, path, object, &["rate", "base", "per"])?;

    let rate_path = read::child(path, "rate");
    let rate = read::number(file, &rate_path, read::field(file, path, object, "rate")?)?;
    if rate < 0.0 {
        return Err(ConfigError::range(
            file,
            &rate_path,
            ">= 0",
            &rate.to_string(),
        ));
    }

    let base_path = read::child(path, "base");
    let base_code = read::string(file, &base_path, read::field(file, path, object, "base")?)?;
    let base = CommissionBase::parse(base_code).map_err(|_| {
        ConfigError::unsupported(file, &base_path, CommissionBase::Equity.code(), base_code)
    })?;

    let per_path = read::child(path, "per");
    let per_code = read::string(file, &per_path, read::field(file, path, object, "per")?)?;
    let per = CommissionPer::parse(per_code).map_err(|_| {
        ConfigError::unsupported(file, &per_path, CommissionPer::Operation.code(), per_code)
    })?;

    CommissionConfig::new(rate, base, per).map_err(|error| {
        ConfigError::range(
            file,
            path,
            "a valid commission configuration",
            &error.to_string(),
        )
    })
}

/// Parses and validates a funding object.
///
/// The rate may be negative because it reverses the payer; the interval is one
/// of the contract's aligned UTC boundaries. Funding is charged per interval on
/// the open position notional.
pub(crate) fn funding(file: &str, path: &str, value: &Value) -> Result<FundingConfig, ConfigError> {
    let object = read::object(file, path, value)?;
    read::check_fields(file, path, object, &["rate", "interval_hours"])?;

    let rate_path = read::child(path, "rate");
    let rate = read::number(file, &rate_path, read::field(file, path, object, "rate")?)?;

    let interval_path = read::child(path, "interval_hours");
    let hours = read::u64(
        file,
        &interval_path,
        read::field(file, path, object, "interval_hours")?,
    )?;
    let interval = u32::try_from(hours)
        .ok()
        .and_then(|hours| FundingInterval::new(hours).ok())
        .ok_or_else(|| {
            ConfigError::range(
                file,
                &interval_path,
                "1, 2, 4, 8, 12, or 24",
                &hours.to_string(),
            )
        })?;

    FundingConfig::new(rate, interval).map_err(|error| {
        ConfigError::range(file, &rate_path, "a finite number", &error.to_string())
    })
}
