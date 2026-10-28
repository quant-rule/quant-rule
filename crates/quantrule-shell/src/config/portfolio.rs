//! `portfolio.json`: capital, sizing, costs, and risk limits.
//!
//! The shell validates every field before it builds the engine's
//! [`PortfolioConfig`], so an error names the exact JSON path. The engine
//! repeats the checks that protect the ledger; the two must agree, and the
//! table-driven tests keep them in step.

use quantrule_backtest::portfolio::{
    CommissionBase, CommissionConfig, CommissionPer, Currency, DrawdownAction, FundingConfig,
    FundingInterval, MarkToMarket, MaxDrawdownConfig, PortfolioConfig, SizingMode,
};
use serde_json::Value;

use super::error::ConfigError;
use super::read::{self, ROOT};

/// The only leverage accepted in v1.
const LEVERAGE: f64 = 1.0;

/// The only open-position limit accepted in v1.
const MAX_OPEN_POSITIONS: u64 = 1;

/// Parses `portfolio.json`.
pub(crate) fn parse(value: &Value, file: &str) -> Result<PortfolioConfig, ConfigError> {
    let object = read::object(file, ROOT, value)?;
    read::check_fields(
        file,
        ROOT,
        object,
        &[
            "schema",
            "initial_cash",
            "currency",
            "sizing",
            "leverage",
            "max_open_positions",
            "allow_short",
            "commission",
            "funding",
            "max_drawdown",
            "mark_to_market",
        ],
    )?;
    read::schema(file, object, "quantrule.portfolio/v1")?;

    let initial_cash = read::number(
        file,
        "$.initial_cash",
        read::field(file, ROOT, object, "initial_cash")?,
    )?;
    if initial_cash <= 0.0 {
        return Err(ConfigError::range(
            file,
            "$.initial_cash",
            "> 0",
            &initial_cash.to_string(),
        ));
    }

    let currency = read::string(
        file,
        "$.currency",
        read::field(file, ROOT, object, "currency")?,
    )?;
    if currency != Currency::Usd.code() {
        return Err(ConfigError::unsupported(
            file,
            "$.currency",
            Currency::Usd.code(),
            currency,
        ));
    }

    let sizing = sizing(file, read::field(file, ROOT, object, "sizing")?)?;

    let leverage = read::number(
        file,
        "$.leverage",
        read::field(file, ROOT, object, "leverage")?,
    )?;
    if leverage != LEVERAGE {
        return Err(ConfigError::unsupported(
            file,
            "$.leverage",
            "1.0",
            &leverage.to_string(),
        ));
    }

    let max_open_positions = read::u64(
        file,
        "$.max_open_positions",
        read::field(file, ROOT, object, "max_open_positions")?,
    )?;
    if max_open_positions != MAX_OPEN_POSITIONS {
        return Err(ConfigError::unsupported(
            file,
            "$.max_open_positions",
            "1",
            &max_open_positions.to_string(),
        ));
    }
    let max_open_positions = u32::try_from(max_open_positions).map_err(|_| {
        ConfigError::unsupported(
            file,
            "$.max_open_positions",
            "1",
            &max_open_positions.to_string(),
        )
    })?;

    let allow_short = read::boolean(
        file,
        "$.allow_short",
        read::field(file, ROOT, object, "allow_short")?,
    )?;

    let commission = commission(file, read::field(file, ROOT, object, "commission")?)?;
    let funding = funding(file, read::field(file, ROOT, object, "funding")?)?;
    let max_drawdown = max_drawdown(file, read::field(file, ROOT, object, "max_drawdown")?)?;

    let mark_to_market = read::string(
        file,
        "$.mark_to_market",
        read::field(file, ROOT, object, "mark_to_market")?,
    )?;
    if mark_to_market != MarkToMarket::BarClose.code() {
        return Err(ConfigError::unsupported(
            file,
            "$.mark_to_market",
            MarkToMarket::BarClose.code(),
            mark_to_market,
        ));
    }

    let config = PortfolioConfig {
        initial_cash,
        currency: Currency::Usd,
        sizing,
        leverage,
        max_open_positions,
        allow_short,
        commission,
        funding,
        max_drawdown,
        mark_to_market: MarkToMarket::BarClose,
    };
    config.validate().map_err(|error| {
        ConfigError::range(
            file,
            "$",
            "a valid portfolio configuration",
            &error.to_string(),
        )
    })?;

    Ok(config)
}

fn sizing(file: &str, value: &Value) -> Result<SizingMode, ConfigError> {
    let object = read::object(file, "$.sizing", value)?;
    let mode = read::string(
        file,
        "$.sizing.mode",
        read::field(file, "$.sizing", object, "mode")?,
    )?;

    match mode {
        "fixed_notional" => {
            read::check_fields(
                file,
                "$.sizing",
                object,
                &["mode", "notional", "quantity_step"],
            )?;
            let notional = read::number(
                file,
                "$.sizing.notional",
                read::field(file, "$.sizing", object, "notional")?,
            )?;
            let quantity_step = read::number(
                file,
                "$.sizing.quantity_step",
                read::field(file, "$.sizing", object, "quantity_step")?,
            )?;
            if notional <= 0.0 {
                return Err(ConfigError::range(
                    file,
                    "$.sizing.notional",
                    "> 0",
                    &notional.to_string(),
                ));
            }
            if quantity_step <= 0.0 {
                return Err(ConfigError::range(
                    file,
                    "$.sizing.quantity_step",
                    "> 0",
                    &quantity_step.to_string(),
                ));
            }
            Ok(SizingMode::FixedNotional {
                notional,
                quantity_step,
            })
        }
        "equity_fraction" => {
            read::check_fields(
                file,
                "$.sizing",
                object,
                &["mode", "fraction", "quantity_step"],
            )?;
            let fraction = read::number(
                file,
                "$.sizing.fraction",
                read::field(file, "$.sizing", object, "fraction")?,
            )?;
            let quantity_step = read::number(
                file,
                "$.sizing.quantity_step",
                read::field(file, "$.sizing", object, "quantity_step")?,
            )?;
            if fraction <= 0.0 || fraction > 1.0 {
                return Err(ConfigError::range(
                    file,
                    "$.sizing.fraction",
                    "(0, 1]",
                    &fraction.to_string(),
                ));
            }
            if quantity_step <= 0.0 {
                return Err(ConfigError::range(
                    file,
                    "$.sizing.quantity_step",
                    "> 0",
                    &quantity_step.to_string(),
                ));
            }
            Ok(SizingMode::EquityFraction {
                fraction,
                quantity_step,
            })
        }
        other => Err(ConfigError::unsupported(
            file,
            "$.sizing.mode",
            "fixed_notional or equity_fraction",
            other,
        )),
    }
}

fn commission(file: &str, value: &Value) -> Result<CommissionConfig, ConfigError> {
    let object = read::object(file, "$.commission", value)?;
    read::check_fields(file, "$.commission", object, &["rate", "base", "per"])?;

    let rate = read::number(
        file,
        "$.commission.rate",
        read::field(file, "$.commission", object, "rate")?,
    )?;
    if rate < 0.0 {
        return Err(ConfigError::range(
            file,
            "$.commission.rate",
            ">= 0",
            &rate.to_string(),
        ));
    }

    let base = read::string(
        file,
        "$.commission.base",
        read::field(file, "$.commission", object, "base")?,
    )?;
    if base != CommissionBase::Equity.code() {
        return Err(ConfigError::unsupported(
            file,
            "$.commission.base",
            CommissionBase::Equity.code(),
            base,
        ));
    }
    let per = read::string(
        file,
        "$.commission.per",
        read::field(file, "$.commission", object, "per")?,
    )?;
    if per != CommissionPer::Operation.code() {
        return Err(ConfigError::unsupported(
            file,
            "$.commission.per",
            CommissionPer::Operation.code(),
            per,
        ));
    }

    CommissionConfig::new(rate, CommissionBase::Equity, CommissionPer::Operation).map_err(|error| {
        ConfigError::range(
            file,
            "$.commission",
            "a valid commission configuration",
            &error.to_string(),
        )
    })
}

fn funding(file: &str, value: &Value) -> Result<FundingConfig, ConfigError> {
    let object = read::object(file, "$.funding", value)?;
    read::check_fields(file, "$.funding", object, &["rate", "interval_hours"])?;

    let rate = read::number(
        file,
        "$.funding.rate",
        read::field(file, "$.funding", object, "rate")?,
    )?;
    let hours = read::u64(
        file,
        "$.funding.interval_hours",
        read::field(file, "$.funding", object, "interval_hours")?,
    )?;
    if !matches!(hours, 1 | 2 | 4 | 8 | 12 | 24) {
        return Err(ConfigError::range(
            file,
            "$.funding.interval_hours",
            "1, 2, 4, 8, 12, or 24",
            &hours.to_string(),
        ));
    }
    let hours = u32::try_from(hours).map_err(|_| {
        ConfigError::range(
            file,
            "$.funding.interval_hours",
            "1, 2, 4, 8, 12, or 24",
            &hours.to_string(),
        )
    })?;
    let interval = FundingInterval::new(hours).map_err(|error| {
        ConfigError::range(
            file,
            "$.funding.interval_hours",
            "1, 2, 4, 8, 12, or 24",
            &error.to_string(),
        )
    })?;

    FundingConfig::new(rate, interval).map_err(|error| {
        ConfigError::range(
            file,
            "$.funding.rate",
            "a finite number",
            &error.to_string(),
        )
    })
}

fn max_drawdown(file: &str, value: &Value) -> Result<MaxDrawdownConfig, ConfigError> {
    let object = read::object(file, "$.max_drawdown", value)?;
    read::check_fields(file, "$.max_drawdown", object, &["limit", "action"])?;

    let limit_value = read::field(file, "$.max_drawdown", object, "limit")?;
    let limit = if limit_value.is_null() {
        None
    } else {
        let limit = read::number(file, "$.max_drawdown.limit", limit_value)?;
        if limit <= 0.0 || limit > 1.0 {
            return Err(ConfigError::range(
                file,
                "$.max_drawdown.limit",
                "(0, 1]",
                &limit.to_string(),
            ));
        }
        Some(limit)
    };

    let action = read::string(
        file,
        "$.max_drawdown.action",
        read::field(file, "$.max_drawdown", object, "action")?,
    )?;
    let action = match action {
        "halt" => DrawdownAction::Halt,
        "none" => DrawdownAction::None,
        other => {
            return Err(ConfigError::unsupported(
                file,
                "$.max_drawdown.action",
                "halt or none",
                other,
            ));
        }
    };
    if limit.is_none() && action == DrawdownAction::Halt {
        return Err(ConfigError::inconsistent(
            file,
            "$.max_drawdown.action",
            "none when the limit is null",
            action.code(),
        ));
    }

    MaxDrawdownConfig::new(limit, action).map_err(|error| {
        ConfigError::range(
            file,
            "$.max_drawdown",
            "a valid drawdown configuration",
            &error.to_string(),
        )
    })
}
