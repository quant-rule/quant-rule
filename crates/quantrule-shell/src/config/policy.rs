//! `policy.json`: the human-owned capital, objective, risk, and universe envelope.
//!
//! The policy is the highest configuration layer. It states what the account is
//! allowed to do and what it is trying to achieve; it never contains an entry,
//! an exit, an indicator, or a statistical fit. The strategy and the engine
//! configs are validated against it in the composition root.
//!
//! The policy deliberately has no session, news, or weekend rules. The system is
//! a pure technical, OHLCV-only backtester; a policy either fits that model or
//! is rejected.

use quantrule_data::Timeframe;
use serde_json::Value;

use super::data::timeframe;
use super::error::ConfigError;
use super::read::{self, ROOT};

/// The policy's supported currency.
const CURRENCY: &str = "USD";

/// The only leverage the current portfolio engine models.
const SUPPORTED_LEVERAGE: f64 = 1.0;

/// The only open-position limit the current portfolio engine models.
const SUPPORTED_OPEN_POSITIONS: u32 = 1;

/// The resolved `policy.json`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PolicyConfig {
    /// Account currency and starting capital.
    pub(crate) account: PolicyAccount,
    /// The evaluation target and drawdown envelope.
    pub(crate) objective: PolicyObjective,
    /// Position and leverage limits.
    pub(crate) risk: PolicyRisk,
    /// The instruments and strategy timeframes the loop may use.
    pub(crate) universe: PolicyUniverse,
}

/// Account identity.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PolicyAccount {
    /// The account currency; exactly `USD` in v1.
    pub(crate) currency: String,
    /// The account's starting capital.
    pub(crate) initial_cash: f64,
}

/// The human's performance and risk objective.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PolicyObjective {
    /// The return target as a fraction, evaluated over `evaluation_days`.
    pub(crate) performance_target: f64,
    /// The evaluation window in whole UTC days.
    pub(crate) evaluation_days: u32,
    /// The maximum tolerated drawdown as a fraction.
    pub(crate) max_drawdown: f64,
}

/// The policy's position and leverage envelope.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PolicyRisk {
    /// The largest position notional as a fraction of account equity.
    pub(crate) max_position_fraction: f64,
    /// The maximum supported leverage.
    pub(crate) max_leverage: f64,
    /// The maximum number of simultaneous positions.
    pub(crate) max_open_positions: u32,
}

/// The instruments and timeframes a policy permits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PolicyUniverse {
    /// The permitted instrument tickets, in declaration order.
    pub(crate) tickets: Vec<String>,
    /// The permitted strategy timeframes, fastest first.
    pub(crate) timeframes: Vec<Timeframe>,
}

/// Parses `policy.json`.
pub(crate) fn parse(value: &Value, file: &str) -> Result<PolicyConfig, ConfigError> {
    let object = read::object(file, ROOT, value)?;
    read::check_fields(
        file,
        ROOT,
        object,
        &["schema", "account", "objective", "risk", "universe"],
    )?;
    read::schema(file, object, "quantrule.policy/v1")?;

    let account = account(file, read::field(file, ROOT, object, "account")?)?;
    let objective = objective(file, read::field(file, ROOT, object, "objective")?)?;
    let risk = risk(file, read::field(file, ROOT, object, "risk")?)?;
    let universe = universe(file, read::field(file, ROOT, object, "universe")?)?;

    Ok(PolicyConfig {
        account,
        objective,
        risk,
        universe,
    })
}

fn account(file: &str, value: &Value) -> Result<PolicyAccount, ConfigError> {
    let object = read::object(file, "$.account", value)?;
    read::check_fields(file, "$.account", object, &["currency", "initial_cash"])?;

    let currency = read::string(
        file,
        "$.account.currency",
        read::field(file, "$.account", object, "currency")?,
    )?;
    if currency != CURRENCY {
        return Err(ConfigError::unsupported(
            file,
            "$.account.currency",
            CURRENCY,
            currency,
        ));
    }

    let initial_cash = read::number(
        file,
        "$.account.initial_cash",
        read::field(file, "$.account", object, "initial_cash")?,
    )?;
    if initial_cash <= 0.0 {
        return Err(ConfigError::range(
            file,
            "$.account.initial_cash",
            "> 0",
            &initial_cash.to_string(),
        ));
    }

    Ok(PolicyAccount {
        currency: currency.to_owned(),
        initial_cash,
    })
}

fn objective(file: &str, value: &Value) -> Result<PolicyObjective, ConfigError> {
    let object = read::object(file, "$.objective", value)?;
    read::check_fields(
        file,
        "$.objective",
        object,
        &["performance_target", "evaluation_days", "max_drawdown"],
    )?;

    let performance_target = read::number(
        file,
        "$.objective.performance_target",
        read::field(file, "$.objective", object, "performance_target")?,
    )?;
    if performance_target <= 0.0 {
        return Err(ConfigError::range(
            file,
            "$.objective.performance_target",
            "> 0",
            &performance_target.to_string(),
        ));
    }
    if !performance_target.is_finite() {
        return Err(ConfigError::range(
            file,
            "$.objective.performance_target",
            "a finite number",
            &performance_target.to_string(),
        ));
    }

    let evaluation_days = read::u64(
        file,
        "$.objective.evaluation_days",
        read::field(file, "$.objective", object, "evaluation_days")?,
    )?;
    if evaluation_days == 0 || evaluation_days > 3_650 {
        return Err(ConfigError::range(
            file,
            "$.objective.evaluation_days",
            "1..=3650",
            &evaluation_days.to_string(),
        ));
    }
    let evaluation_days = u32::try_from(evaluation_days).map_err(|_| {
        ConfigError::range(
            file,
            "$.objective.evaluation_days",
            "1..=3650",
            &evaluation_days.to_string(),
        )
    })?;

    let max_drawdown = read::number(
        file,
        "$.objective.max_drawdown",
        read::field(file, "$.objective", object, "max_drawdown")?,
    )?;
    if max_drawdown <= 0.0 || max_drawdown > 1.0 {
        return Err(ConfigError::range(
            file,
            "$.objective.max_drawdown",
            "(0, 1]",
            &max_drawdown.to_string(),
        ));
    }

    Ok(PolicyObjective {
        performance_target,
        evaluation_days,
        max_drawdown,
    })
}

fn risk(file: &str, value: &Value) -> Result<PolicyRisk, ConfigError> {
    let object = read::object(file, "$.risk", value)?;
    read::check_fields(
        file,
        "$.risk",
        object,
        &[
            "max_position_fraction",
            "max_leverage",
            "max_open_positions",
        ],
    )?;

    let max_position_fraction = read::number(
        file,
        "$.risk.max_position_fraction",
        read::field(file, "$.risk", object, "max_position_fraction")?,
    )?;
    if max_position_fraction <= 0.0 || max_position_fraction > 1.0 {
        return Err(ConfigError::range(
            file,
            "$.risk.max_position_fraction",
            "(0, 1]",
            &max_position_fraction.to_string(),
        ));
    }

    let max_leverage = read::number(
        file,
        "$.risk.max_leverage",
        read::field(file, "$.risk", object, "max_leverage")?,
    )?;
    if max_leverage != SUPPORTED_LEVERAGE {
        return Err(ConfigError::unsupported(
            file,
            "$.risk.max_leverage",
            "1.0",
            &max_leverage.to_string(),
        ));
    }

    let max_open_positions = read::u64(
        file,
        "$.risk.max_open_positions",
        read::field(file, "$.risk", object, "max_open_positions")?,
    )?;
    if max_open_positions != u64::from(SUPPORTED_OPEN_POSITIONS) {
        return Err(ConfigError::unsupported(
            file,
            "$.risk.max_open_positions",
            "1",
            &max_open_positions.to_string(),
        ));
    }

    Ok(PolicyRisk {
        max_position_fraction,
        max_leverage,
        max_open_positions: SUPPORTED_OPEN_POSITIONS,
    })
}

fn universe(file: &str, value: &Value) -> Result<PolicyUniverse, ConfigError> {
    let object = read::object(file, "$.universe", value)?;
    read::check_fields(file, "$.universe", object, &["tickets", "timeframes"])?;

    let tickets = tickets(file, read::field(file, "$.universe", object, "tickets")?)?;
    let timeframes = timeframes(file, read::field(file, "$.universe", object, "timeframes")?)?;

    Ok(PolicyUniverse {
        tickets,
        timeframes,
    })
}

fn tickets(file: &str, value: &Value) -> Result<Vec<String>, ConfigError> {
    let items = read::array(file, "$.universe.tickets", value)?;
    if items.is_empty() {
        return Err(ConfigError::range(
            file,
            "$.universe.tickets",
            "a non-empty array",
            "[]",
        ));
    }

    let mut tickets = Vec::with_capacity(items.len());
    for (position, item) in items.iter().enumerate() {
        let path = read::index("$.universe.tickets", position);
        let ticket = read::non_empty_string(file, &path, item)?;
        if ticket.chars().any(char::is_whitespace) {
            return Err(ConfigError::range(
                file,
                &path,
                "a ticket without whitespace",
                ticket,
            ));
        }
        if tickets.iter().any(|seen| seen == ticket) {
            return Err(ConfigError::range(file, &path, "a unique ticket", ticket));
        }
        tickets.push(ticket.to_owned());
    }
    Ok(tickets)
}

fn timeframes(file: &str, value: &Value) -> Result<Vec<Timeframe>, ConfigError> {
    let items = read::array(file, "$.universe.timeframes", value)?;
    if items.is_empty() {
        return Err(ConfigError::range(
            file,
            "$.universe.timeframes",
            "a non-empty array",
            "[]",
        ));
    }

    let mut timeframes: Vec<Timeframe> = Vec::with_capacity(items.len());
    for (position, item) in items.iter().enumerate() {
        let path = read::index("$.universe.timeframes", position);
        let timeframe = timeframe(file, &path, item)?;
        if let Some(previous) = timeframes.last()
            && previous.seconds() >= timeframe.seconds()
        {
            return Err(ConfigError::range(
                file,
                &path,
                &format!("a timeframe longer than {}", previous.code()),
                timeframe.code(),
            ));
        }
        timeframes.push(timeframe);
    }
    Ok(timeframes)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::parse;
    use crate::config::error::ConfigErrorKind;

    fn valid() -> serde_json::Value {
        json!({
            "schema": "quantrule.policy/v1",
            "account": { "currency": "USD", "initial_cash": 10000.0 },
            "objective": {
                "performance_target": 0.10,
                "evaluation_days": 90,
                "max_drawdown": 0.10
            },
            "risk": {
                "max_position_fraction": 1.0,
                "max_leverage": 1.0,
                "max_open_positions": 1
            },
            "universe": {
                "tickets": ["EURUSD"],
                "timeframes": ["15m"]
            }
        })
    }

    #[test]
    fn parses_the_policy_blueprint() {
        let policy = parse(&valid(), "policy.json").expect("the policy is valid");
        assert_eq!(policy.account.initial_cash, 10_000.0);
        assert_eq!(policy.objective.evaluation_days, 90);
        assert_eq!(policy.universe.tickets, ["EURUSD"]);
    }

    #[test]
    fn rejects_non_usd_and_excess_leverage() {
        let mut value = valid();
        value["account"]["currency"] = json!("EUR");
        assert_eq!(
            parse(&value, "policy.json")
                .expect_err("EUR is unsupported")
                .kind(),
            ConfigErrorKind::Unsupported
        );

        let mut value = valid();
        value["risk"]["max_leverage"] = json!(2.0);
        assert_eq!(
            parse(&value, "policy.json")
                .expect_err("2x is unsupported")
                .kind(),
            ConfigErrorKind::Unsupported
        );
    }

    #[test]
    fn rejects_unordered_or_duplicate_universe_entries() {
        let mut value = valid();
        value["universe"]["timeframes"] = json!(["15m", "5m"]);
        assert_eq!(
            parse(&value, "policy.json")
                .expect_err("order is fixed")
                .kind(),
            ConfigErrorKind::Range
        );

        let mut value = valid();
        value["universe"]["tickets"] = json!(["EURUSD", "EURUSD"]);
        assert_eq!(
            parse(&value, "policy.json")
                .expect_err("duplicates are rejected")
                .kind(),
            ConfigErrorKind::Range
        );
    }
}
