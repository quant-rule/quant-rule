//! The portfolio policy pack payload and its content-addressed identity.
//!
//! A portfolio policy pack is the human-owned, reusable simulation policy: how
//! an entry is sized, how much a trade may risk, how much exposure the account
//! may carry, how many positions may be open, when drawdown halts the account,
//! whether the portfolio rebalances on a bar interval, and which declarative
//! constraints narrow the policy.
//!
//! Initial cash and currency are deliberately absent. They are account inputs
//! owned by the run, so one policy can be evaluated against several accounts.
//! The closed schema rejects every field it does not define, including
//! correlation, covariance, optimisation, fitted and other statistical fields:
//! a portfolio policy is a deterministic rule, not a fitted model.

use quantrule_api::Digest;
use quantrule_backtest::portfolio::{DrawdownAction, MaxDrawdownConfig, SizingMode};
use serde_json::Value;

use crate::config::ConfigError;
use crate::config::read;

use super::core::PackCore;
use super::envelope::{PackEnvelope, PackKind, PackMetadata};
use super::error::PackError;
use super::lifecycle::{PackState, PackStateError};

const REBALANCE_NONE: &str = "none";
const REBALANCE_BAR_INTERVAL: &str = "bar_interval";
const MAX_POSITIONS_PER_INSTRUMENT: &str = "max_positions_per_instrument";
const MIN_HOLDING_BARS: &str = "min_holding_bars";

/// A deterministic portfolio rebalance rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RebalanceRule {
    /// No periodic rebalance; the position changes only on strategy intent.
    None,
    /// Re-evaluate the target position every `bars` execution bars.
    BarInterval {
        /// The positive number of execution bars between rebalances.
        bars: u32,
    },
}

impl RebalanceRule {
    /// The stable wire code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::None => REBALANCE_NONE,
            Self::BarInterval { .. } => REBALANCE_BAR_INTERVAL,
        }
    }

    /// The bar interval, or `None` when periodic rebalancing is disabled.
    #[must_use]
    pub const fn bars(self) -> Option<u32> {
        match self {
            Self::None => None,
            Self::BarInterval { bars } => Some(bars),
        }
    }
}

/// One declarative, rule-based portfolio constraint.
///
/// A constraint carries one closed kind and one positive integer value. The
/// list is strictly ascending by kind and carries no duplicate, so one policy
/// has exactly one byte representation for the same set of constraints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortfolioConstraint {
    /// The maximum number of open positions in one instrument.
    MaxPositionsPerInstrument(u32),
    /// The minimum number of execution bars a position is held.
    MinHoldingBars(u32),
}

impl PortfolioConstraint {
    /// The stable wire code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::MaxPositionsPerInstrument(_) => MAX_POSITIONS_PER_INSTRUMENT,
            Self::MinHoldingBars(_) => MIN_HOLDING_BARS,
        }
    }

    /// The positive integer value.
    #[must_use]
    pub const fn value(self) -> u32 {
        match self {
            Self::MaxPositionsPerInstrument(value) | Self::MinHoldingBars(value) => value,
        }
    }
}

/// The validated payload of a portfolio policy pack.
#[derive(Debug, Clone)]
pub struct PortfolioPackPayload {
    sizing: SizingMode,
    per_trade_risk: f64,
    max_exposure: f64,
    max_open_positions: u32,
    drawdown: MaxDrawdownConfig,
    rebalance: RebalanceRule,
    constraints: Vec<PortfolioConstraint>,
}

impl PortfolioPackPayload {
    fn parse(file: &str, path: &str, value: &Value) -> Result<Self, PackError> {
        let object = read::object(file, path, value)?;
        read::check_fields(
            file,
            path,
            object,
            &["sizing", "risk", "drawdown", "rebalance", "constraints"],
        )?;

        let sizing = parse_sizing(
            file,
            &read::child(path, "sizing"),
            read::field(file, path, object, "sizing")?,
        )?;
        let (per_trade_risk, max_exposure, max_open_positions) = parse_risk(
            file,
            &read::child(path, "risk"),
            read::field(file, path, object, "risk")?,
        )?;
        let drawdown = parse_drawdown(
            file,
            &read::child(path, "drawdown"),
            read::field(file, path, object, "drawdown")?,
        )?;
        let rebalance = parse_rebalance(file, &read::child(path, "rebalance"), object)?;
        let constraints = parse_constraints(
            file,
            &read::child(path, "constraints"),
            object,
            max_open_positions,
        )?;

        Ok(Self {
            sizing,
            per_trade_risk,
            max_exposure,
            max_open_positions,
            drawdown,
            rebalance,
            constraints,
        })
    }

    /// The validated position-sizing rule.
    #[must_use]
    pub const fn sizing(&self) -> SizingMode {
        self.sizing
    }

    /// The fraction of account equity risked on one trade.
    #[must_use]
    pub const fn per_trade_risk(&self) -> f64 {
        self.per_trade_risk
    }

    /// The maximum account exposure, as a fraction of equity.
    #[must_use]
    pub const fn max_exposure(&self) -> f64 {
        self.max_exposure
    }

    /// The maximum number of open positions.
    #[must_use]
    pub const fn max_open_positions(&self) -> u32 {
        self.max_open_positions
    }

    /// The maximum-drawdown rule.
    #[must_use]
    pub const fn drawdown(&self) -> MaxDrawdownConfig {
        self.drawdown
    }

    /// The deterministic rebalance rule.
    #[must_use]
    pub const fn rebalance(&self) -> RebalanceRule {
        self.rebalance
    }

    /// The declarative constraints, in strictly ascending kind order.
    #[must_use]
    pub fn constraints(&self) -> &[PortfolioConstraint] {
        &self.constraints
    }
}

/// A portfolio policy pack with its canonical identity and lifecycle state.
#[derive(Debug, Clone)]
pub struct PortfolioPack {
    core: PackCore,
    payload: PortfolioPackPayload,
}

impl PortfolioPack {
    /// Parses and validates a portfolio policy pack document.
    ///
    /// The content hash is taken over the canonical bytes of the whole
    /// envelope, including the name, version, owner, and payload. The pack
    /// starts in [`PackState::Draft`].
    ///
    /// # Errors
    ///
    /// Returns [`PackError`] when the envelope is invalid, the kind is not
    /// `portfolio`, or the payload disagrees with the portfolio policy schema.
    pub fn parse(value: &Value, file: &str) -> Result<Self, PackError> {
        let (envelope, payload) = Self::parse_parts(value, file)?;
        Ok(Self {
            core: PackCore::new(value, envelope.metadata()),
            payload,
        })
    }

    fn parse_parts(
        value: &Value,
        file: &str,
    ) -> Result<(PackEnvelope, PortfolioPackPayload), PackError> {
        let envelope = PackEnvelope::parse(value, file)?;
        if envelope.metadata().kind() != PackKind::Portfolio {
            return Err(PackError::WrongKind {
                expected: PackKind::Portfolio,
                found: envelope.metadata().kind(),
            });
        }
        let payload = PortfolioPackPayload::parse(file, "$.payload", envelope.payload())?;
        Ok((envelope, payload))
    }

    /// Parses a JSON text document.
    ///
    /// # Errors
    ///
    /// Returns [`PackError`] when the text is not JSON or the document is not a
    /// valid portfolio policy pack.
    pub fn parse_json(text: &str, file: &str) -> Result<Self, PackError> {
        let value: Value = serde_json::from_str(text)
            .map_err(|error| ConfigError::invalid_json(file, &error.to_string()))?;
        Self::parse(&value, file)
    }

    /// Reloads a pack from stored canonical bytes.
    ///
    /// The bytes are hashed as stored; they are never re-serialised. Bytes that
    /// parse but differ from the canonical form are refused, so a stored pack
    /// cannot acquire a second identity through whitespace or key order.
    ///
    /// # Errors
    ///
    /// Returns [`PackError`] when the bytes are not UTF-8, are not JSON, are not
    /// a valid portfolio policy pack, or are not canonical.
    pub fn from_stored_bytes(bytes: &[u8], file: &str) -> Result<Self, PackError> {
        let text = std::str::from_utf8(bytes)
            .map_err(|_| ConfigError::invalid_json(file, "the stored pack is not UTF-8"))?;
        let mut pack = Self::parse_json(text, file)?;
        // The stored bytes are the identity, not a fresh serialisation of them.
        pack.core.reidentify(bytes, file)?;
        Ok(pack)
    }

    /// The immutable envelope metadata.
    #[must_use]
    pub fn metadata(&self) -> &PackMetadata {
        self.core.metadata()
    }

    /// The validated payload.
    #[must_use]
    pub fn payload(&self) -> &PortfolioPackPayload {
        &self.payload
    }

    /// The content hash of the canonical bytes.
    #[must_use]
    pub fn id(&self) -> &Digest {
        self.core.id()
    }

    /// The canonical bytes the identity covers.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        self.core.canonical_bytes()
    }

    /// The canonical JSON text, including its trailing newline.
    #[must_use]
    pub fn canonical_text(&self) -> &str {
        self.core.canonical_text()
    }

    /// The lifecycle state.
    #[must_use]
    pub const fn state(&self) -> PackState {
        self.core.state()
    }

    /// Marks the pack validated after the schema check.
    ///
    /// # Errors
    ///
    /// Returns [`PackStateError`] unless the pack is a draft.
    pub fn validate(&mut self) -> Result<(), PackStateError> {
        self.core.validate()
    }

    /// Rejects a validated pack and returns it to draft for editing.
    ///
    /// # Errors
    ///
    /// Returns [`PackStateError`] unless the pack is validated.
    pub fn reject(&mut self) -> Result<(), PackStateError> {
        self.core.reject()
    }

    /// Freezes a validated pack.
    ///
    /// The caller owns the human freeze decision and its Quant Validator
    /// record; this method enforces the lifecycle precondition only.
    ///
    /// # Errors
    ///
    /// Returns [`PackStateError`] unless the pack is validated.
    pub fn freeze(&mut self) -> Result<(), PackStateError> {
        self.core.freeze()
    }

    /// Retires a frozen pack from new runs.
    ///
    /// # Errors
    ///
    /// Returns [`PackStateError`] unless the pack is frozen.
    pub fn retire(&mut self) -> Result<(), PackStateError> {
        self.core.retire()
    }
}

fn parse_sizing(file: &str, path: &str, value: &Value) -> Result<SizingMode, ConfigError> {
    let object = read::object(file, path, value)?;
    let mode = read::string(
        file,
        &read::child(path, "mode"),
        read::field(file, path, object, "mode")?,
    )?;

    let sizing = match mode {
        "fixed_notional" => {
            read::check_fields(file, path, object, &["mode", "notional", "quantity_step"])?;
            let notional = read::number(
                file,
                &read::child(path, "notional"),
                read::field(file, path, object, "notional")?,
            )?;
            if notional <= 0.0 {
                return Err(ConfigError::range(
                    file,
                    &read::child(path, "notional"),
                    "> 0",
                    &notional.to_string(),
                ));
            }
            let quantity_step = parse_quantity_step(file, path, object)?;
            SizingMode::FixedNotional {
                notional,
                quantity_step,
            }
        }
        "equity_fraction" => {
            read::check_fields(file, path, object, &["mode", "fraction", "quantity_step"])?;
            let fraction = read::number(
                file,
                &read::child(path, "fraction"),
                read::field(file, path, object, "fraction")?,
            )?;
            if fraction <= 0.0 || fraction > 1.0 {
                return Err(ConfigError::range(
                    file,
                    &read::child(path, "fraction"),
                    "(0, 1]",
                    &fraction.to_string(),
                ));
            }
            let quantity_step = parse_quantity_step(file, path, object)?;
            SizingMode::EquityFraction {
                fraction,
                quantity_step,
            }
        }
        other => {
            return Err(ConfigError::range(
                file,
                &read::child(path, "mode"),
                "fixed_notional or equity_fraction",
                other,
            ));
        }
    };

    // The engine owns the sizing invariant. The path-specific checks above make
    // the error precise; this call keeps one definition of the invariant.
    sizing.validate().map_err(|error| {
        ConfigError::range(file, path, "a valid sizing rule", &error.to_string())
    })?;
    Ok(sizing)
}

fn parse_quantity_step(
    file: &str,
    path: &str,
    object: &serde_json::Map<String, Value>,
) -> Result<f64, ConfigError> {
    let quantity_step = read::number(
        file,
        &read::child(path, "quantity_step"),
        read::field(file, path, object, "quantity_step")?,
    )?;
    if quantity_step <= 0.0 {
        return Err(ConfigError::range(
            file,
            &read::child(path, "quantity_step"),
            "> 0",
            &quantity_step.to_string(),
        ));
    }
    Ok(quantity_step)
}

fn parse_risk(file: &str, path: &str, value: &Value) -> Result<(f64, f64, u32), ConfigError> {
    let object = read::object(file, path, value)?;
    read::check_fields(
        file,
        path,
        object,
        &["per_trade_risk", "max_exposure", "max_open_positions"],
    )?;

    let per_trade_risk = read::number(
        file,
        &read::child(path, "per_trade_risk"),
        read::field(file, path, object, "per_trade_risk")?,
    )?;
    if per_trade_risk <= 0.0 || per_trade_risk > 1.0 {
        return Err(ConfigError::range(
            file,
            &read::child(path, "per_trade_risk"),
            "(0, 1]",
            &per_trade_risk.to_string(),
        ));
    }

    let max_exposure = read::number(
        file,
        &read::child(path, "max_exposure"),
        read::field(file, path, object, "max_exposure")?,
    )?;
    if max_exposure <= 0.0 || max_exposure > 1.0 {
        return Err(ConfigError::range(
            file,
            &read::child(path, "max_exposure"),
            "(0, 1]",
            &max_exposure.to_string(),
        ));
    }

    let raw = read::u64(
        file,
        &read::child(path, "max_open_positions"),
        read::field(file, path, object, "max_open_positions")?,
    )?;
    let max_open_positions = positive_u32(file, &read::child(path, "max_open_positions"), raw)?;
    Ok((per_trade_risk, max_exposure, max_open_positions))
}

fn parse_drawdown(file: &str, path: &str, value: &Value) -> Result<MaxDrawdownConfig, ConfigError> {
    let object = read::object(file, path, value)?;
    read::check_fields(file, path, object, &["limit", "action"])?;

    let limit = read::number(
        file,
        &read::child(path, "limit"),
        read::field(file, path, object, "limit")?,
    )?;
    if limit <= 0.0 || limit > 1.0 {
        return Err(ConfigError::range(
            file,
            &read::child(path, "limit"),
            "(0, 1]",
            &limit.to_string(),
        ));
    }

    let action = read::string(
        file,
        &read::child(path, "action"),
        read::field(file, path, object, "action")?,
    )?;
    if action != DrawdownAction::Halt.code() {
        return Err(ConfigError::unsupported(
            file,
            &read::child(path, "action"),
            DrawdownAction::Halt.code(),
            action,
        ));
    }

    MaxDrawdownConfig::new(Some(limit), DrawdownAction::Halt).map_err(|error| {
        ConfigError::range(file, path, "a valid drawdown rule", &error.to_string())
    })
}

fn parse_rebalance(
    file: &str,
    path: &str,
    object: &serde_json::Map<String, Value>,
) -> Result<RebalanceRule, ConfigError> {
    let Some(value) = object.get("rebalance") else {
        return Ok(RebalanceRule::None);
    };
    match value {
        Value::String(code) => {
            if code != REBALANCE_NONE {
                return Err(ConfigError::range(
                    file,
                    path,
                    "none or a bar_interval object",
                    code,
                ));
            }
            Ok(RebalanceRule::None)
        }
        Value::Object(inner) => {
            read::check_fields(file, path, inner, &["mode", "bars"])?;
            let mode = read::string(
                file,
                &read::child(path, "mode"),
                read::field(file, path, inner, "mode")?,
            )?;
            if mode != REBALANCE_BAR_INTERVAL {
                return Err(ConfigError::unsupported(
                    file,
                    &read::child(path, "mode"),
                    REBALANCE_BAR_INTERVAL,
                    mode,
                ));
            }
            let raw = read::u64(
                file,
                &read::child(path, "bars"),
                read::field(file, path, inner, "bars")?,
            )?;
            let bars = positive_u32(file, &read::child(path, "bars"), raw)?;
            Ok(RebalanceRule::BarInterval { bars })
        }
        other => Err(ConfigError::wrong_type(
            file,
            path,
            "none or an object",
            &read::observed(other),
        )),
    }
}

fn parse_constraints(
    file: &str,
    path: &str,
    object: &serde_json::Map<String, Value>,
    max_open_positions: u32,
) -> Result<Vec<PortfolioConstraint>, ConfigError> {
    let Some(value) = object.get("constraints") else {
        return Ok(Vec::new());
    };
    let values = read::array(file, path, value)?;
    let mut constraints = Vec::with_capacity(values.len());
    let mut previous: Option<&'static str> = None;

    for (index, value) in values.iter().enumerate() {
        let item_path = read::index(path, index);
        let item = read::object(file, &item_path, value)?;
        read::check_fields(file, &item_path, item, &["kind", "value"])?;
        let kind = read::string(
            file,
            &read::child(&item_path, "kind"),
            read::field(file, &item_path, item, "kind")?,
        )?;
        let value_path = read::child(&item_path, "value");

        let constraint = match kind {
            MAX_POSITIONS_PER_INSTRUMENT => {
                let raw = read::u64(
                    file,
                    &value_path,
                    read::field(file, &item_path, item, "value")?,
                )?;
                let limit = positive_u32(file, &value_path, raw)?;
                if limit > max_open_positions {
                    return Err(ConfigError::inconsistent(
                        file,
                        &value_path,
                        &format!("at most max_open_positions ({max_open_positions})"),
                        &limit.to_string(),
                    ));
                }
                PortfolioConstraint::MaxPositionsPerInstrument(limit)
            }
            MIN_HOLDING_BARS => {
                let raw = read::u64(
                    file,
                    &value_path,
                    read::field(file, &item_path, item, "value")?,
                )?;
                PortfolioConstraint::MinHoldingBars(positive_u32(file, &value_path, raw)?)
            }
            other => {
                return Err(ConfigError::range(
                    file,
                    &read::child(&item_path, "kind"),
                    "max_positions_per_instrument or min_holding_bars",
                    other,
                ));
            }
        };

        let code = constraint.code();
        if let Some(previous) = previous
            && code <= previous
        {
            return Err(ConfigError::range(
                file,
                &read::child(&item_path, "kind"),
                "a strictly ascending constraint kind with no duplicate",
                code,
            ));
        }
        previous = Some(code);
        constraints.push(constraint);
    }

    Ok(constraints)
}

fn positive_u32(file: &str, path: &str, raw: u64) -> Result<u32, ConfigError> {
    if raw == 0 {
        return Err(ConfigError::range(
            file,
            path,
            "a positive integer",
            &raw.to_string(),
        ));
    }
    u32::try_from(raw).map_err(|_| {
        ConfigError::range(
            file,
            path,
            "an integer no greater than 4294967295",
            &raw.to_string(),
        )
    })
}

#[cfg(test)]
mod tests;
