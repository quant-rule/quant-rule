//! `strategy.json`: the human-owned, explainable trading rule.
//!
//! The strategy is the most important input in the system. It states, in human
//! trader words, why the edge should exist, which timeframe it trades, the
//! deterministic entry and exit conditions, and the risk/reward envelope. It
//! contains no indicator formula, fill, portfolio, or statistical fit. The
//! model measures; the strategy decides; execution fills; the portfolio accounts.
//!
//! The config keeps the current `axis-agreement` rule shape. A future rule adds
//! a new explicit `rule` value and its own parameters; it cannot smuggle a
//! statistical field into the strategy input.
//!
//! The risk block becomes a [`RiskPolicy`] on the strategy. Execution receives
//! the resulting `TradeIntent`; it does not own the stop or target distance.

use quantrule_backtest::intent::RiskPolicy;
use quantrule_backtest::strategy::StrategyConfig;
use serde_json::Value;

use super::data::timeframe;
use super::error::ConfigError;
use super::read::{self, ROOT};

/// The only strategy name accepted in v2.
const NAME: &str = "axis-agreement";

/// The resolved strategy blueprint.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StrategyRunConfig {
    /// The engine thresholds and strategy timeframe.
    pub(crate) engine: StrategyConfig,
    /// The human explanation of why the rule should have an edge.
    pub(crate) thesis: String,
    /// The risk intent attached to each entry.
    risk: RiskPolicy,
}

impl StrategyRunConfig {
    /// The strategy's risk policy.
    #[must_use]
    pub(crate) const fn risk(&self) -> RiskPolicy {
        self.risk
    }

    /// The stop distance in ATR units.
    #[must_use]
    pub(crate) const fn stop_atr_multiple(&self) -> f64 {
        self.risk.stop_atr_multiple()
    }

    /// The target distance as a multiple of the stop distance.
    #[must_use]
    pub(crate) const fn reward_to_risk(&self) -> f64 {
        self.risk.reward_to_risk()
    }

    /// The target distance in ATR units: `stop × reward_to_risk`.
    #[must_use]
    pub(crate) fn target_atr_multiple(&self) -> f64 {
        self.risk.stop_atr_multiple() * self.risk.reward_to_risk()
    }
}

/// Parses `strategy.json`.
pub(crate) fn parse(value: &Value, file: &str) -> Result<StrategyRunConfig, ConfigError> {
    let object = read::object(file, ROOT, value)?;
    read::check_fields(
        file,
        ROOT,
        object,
        &[
            "schema",
            "name",
            "thesis",
            "timeframe",
            "long_threshold",
            "short_threshold",
            "stop_atr_multiple",
            "reward_to_risk",
        ],
    )?;
    read::schema(file, object, "quantrule.strategy/v2")?;

    let name = read::string(file, "$.name", read::field(file, ROOT, object, "name")?)?;
    if name != NAME {
        return Err(ConfigError::unsupported(file, "$.name", NAME, name));
    }

    let thesis =
        read::non_empty_string(file, "$.thesis", read::field(file, ROOT, object, "thesis")?)?
            .to_owned();

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

    let stop_atr_multiple = read::number(
        file,
        "$.stop_atr_multiple",
        read::field(file, ROOT, object, "stop_atr_multiple")?,
    )?;
    if !stop_atr_multiple.is_finite() || stop_atr_multiple <= 0.0 {
        return Err(ConfigError::range(
            file,
            "$.stop_atr_multiple",
            "> 0",
            &stop_atr_multiple.to_string(),
        ));
    }

    let reward_to_risk = read::number(
        file,
        "$.reward_to_risk",
        read::field(file, ROOT, object, "reward_to_risk")?,
    )?;
    if !reward_to_risk.is_finite() || reward_to_risk <= 0.0 {
        return Err(ConfigError::range(
            file,
            "$.reward_to_risk",
            "> 0",
            &reward_to_risk.to_string(),
        ));
    }

    let engine = StrategyConfig::new(
        u64::from(timeframe.seconds()),
        long_threshold,
        short_threshold,
    )
    .map_err(|error| {
        ConfigError::range(
            file,
            "$",
            "a valid strategy configuration",
            &error.to_string(),
        )
    })?;
    let risk = RiskPolicy::new(stop_atr_multiple, reward_to_risk).map_err(|error| {
        ConfigError::range(file, "$", "a valid risk policy", &error.to_string())
    })?;

    Ok(StrategyRunConfig {
        engine,
        thesis,
        risk,
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::parse;
    use crate::config::error::ConfigErrorKind;

    fn valid() -> serde_json::Value {
        json!({
            "schema": "quantrule.strategy/v2",
            "name": "axis-agreement",
            "thesis": "Trade only when direction, momentum, and structure agree.",
            "timeframe": "15m",
            "long_threshold": 0.1,
            "short_threshold": -0.1,
            "stop_atr_multiple": 2.0,
            "reward_to_risk": 1.5
        })
    }

    #[test]
    fn parses_the_explainable_strategy() {
        let strategy = parse(&valid(), "strategy.json").expect("the strategy is valid");
        assert_eq!(
            strategy.thesis,
            "Trade only when direction, momentum, and structure agree."
        );
        assert_eq!(strategy.engine.timeframe_secs(), 900);
        assert_eq!(strategy.stop_atr_multiple(), 2.0);
        assert_eq!(strategy.reward_to_risk(), 1.5);
        assert_eq!(strategy.target_atr_multiple(), 3.0);
        assert_eq!(strategy.risk().stop_atr_multiple(), 2.0);
    }

    #[test]
    fn a_missing_thesis_is_rejected() {
        let mut value = valid();
        value["thesis"] = json!("");
        assert_eq!(
            parse(&value, "strategy.json")
                .expect_err("thesis cannot be empty")
                .kind(),
            ConfigErrorKind::Range
        );
    }

    #[test]
    fn risk_fields_must_be_positive() {
        let mut value = valid();
        value["stop_atr_multiple"] = json!(0.0);
        assert_eq!(
            parse(&value, "strategy.json")
                .expect_err("stop must be positive")
                .kind(),
            ConfigErrorKind::Range
        );

        let mut value = valid();
        value["reward_to_risk"] = json!(-1.0);
        assert_eq!(
            parse(&value, "strategy.json")
                .expect_err("reward must be positive")
                .kind(),
            ConfigErrorKind::Range
        );
    }
}
