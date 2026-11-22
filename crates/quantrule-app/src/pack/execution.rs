//! The execution-simulation pack payload and its content-addressed identity.
//!
//! An execution-simulation pack is the human-owned, reusable description of how
//! a strategy decision becomes a simulated fill: the fill timeframe, the order
//! and tie policy, the fee and slippage, commission, funding, and the
//! zero-volume and end-of-data rules. QuantRule researches historical candles;
//! this pack is not a broker connection and has no venue, order-routing,
//! latency, participation or live-market field.
//!
//! The payload reuses the engine's [`ExecutionConfig`], [`CommissionConfig`] and
//! [`FundingConfig`] so a cost has one definition. Only mechanics the current
//! simulation honours have a field; a closed schema keeps an unsupported order
//! style, venue or statistical assumption out of the pack.

use quantrule_api::Digest;
use quantrule_backtest::execution::ExecutionConfig;
use quantrule_backtest::portfolio::{CommissionConfig, FundingConfig};
use quantrule_data::Timeframe;
use serde_json::Value;

use crate::config::costs;
use crate::config::read;
use crate::config::{ConfigError, parse_timeframe};

use super::core::PackCore;
use super::envelope::{PackEnvelope, PackKind, PackMetadata};
use super::error::PackError;
use super::lifecycle::{PackState, PackStateError};

const PAYLOAD_FIELDS: &[&str] = &[
    "fill_timeframe",
    "order_type",
    "tie_break",
    "fee_rate",
    "slippage_bps",
    "commission",
    "funding",
    "zero_volume",
    "end_of_data",
];

/// The order style a simulated decision is submitted as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrderType {
    /// Submit the decision on its bar and fill at the next bar's open.
    MarketNextOpen,
}

impl OrderType {
    /// The stable wire code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::MarketNextOpen => "market_next_open",
        }
    }

    /// Parses the stable wire code.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "market_next_open" => Some(Self::MarketNextOpen),
            _ => None,
        }
    }
}

/// The rule that resolves a bar that reaches both the stop and the target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TieBreak {
    /// The stop fills first; the conservative accounting choice.
    StopFirst,
}

impl TieBreak {
    /// The stable wire code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::StopFirst => "stop_first",
        }
    }

    /// Parses the stable wire code.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "stop_first" => Some(Self::StopFirst),
            _ => None,
        }
    }
}

/// What the simulation does when a bar has zero volume.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ZeroVolumePolicy {
    /// The order cannot fill and is rejected.
    Reject,
}

impl ZeroVolumePolicy {
    /// The stable wire code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Reject => "reject",
        }
    }

    /// Parses the stable wire code.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "reject" => Some(Self::Reject),
            _ => None,
        }
    }
}

/// What the simulation does with a position still open at the end of the data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EndOfDataPolicy {
    /// Leave the position open; the run reports it rather than inventing an exit.
    LeaveOpen,
}

impl EndOfDataPolicy {
    /// The stable wire code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::LeaveOpen => "leave_open",
        }
    }

    /// Parses the stable wire code.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "leave_open" => Some(Self::LeaveOpen),
            _ => None,
        }
    }
}

/// The validated payload of an execution-simulation pack.
#[derive(Debug, Clone)]
pub struct ExecutionPackPayload {
    fill_timeframe: Timeframe,
    order_type: OrderType,
    tie_break: TieBreak,
    costs: ExecutionConfig,
    commission: CommissionConfig,
    funding: FundingConfig,
    zero_volume: ZeroVolumePolicy,
    end_of_data: EndOfDataPolicy,
}

impl ExecutionPackPayload {
    fn parse(file: &str, path: &str, value: &Value) -> Result<Self, PackError> {
        let object = read::object(file, path, value)?;
        read::check_fields(file, path, object, PAYLOAD_FIELDS)?;

        let fill_timeframe = parse_timeframe(
            file,
            &read::child(path, "fill_timeframe"),
            read::field(file, path, object, "fill_timeframe")?,
        )?;
        let order_type = parse_order_type(
            file,
            &read::child(path, "order_type"),
            read::field(file, path, object, "order_type")?,
        )?;
        let tie_break = parse_tie_break(
            file,
            &read::child(path, "tie_break"),
            read::field(file, path, object, "tie_break")?,
        )?;
        let execution_costs = costs::execution(file, path, object)?;
        let commission = costs::commission(
            file,
            &read::child(path, "commission"),
            read::field(file, path, object, "commission")?,
        )?;
        let funding = costs::funding(
            file,
            &read::child(path, "funding"),
            read::field(file, path, object, "funding")?,
        )?;
        let zero_volume = parse_zero_volume(
            file,
            &read::child(path, "zero_volume"),
            read::field(file, path, object, "zero_volume")?,
        )?;
        let end_of_data = parse_end_of_data(
            file,
            &read::child(path, "end_of_data"),
            read::field(file, path, object, "end_of_data")?,
        )?;

        Ok(Self {
            fill_timeframe,
            order_type,
            tie_break,
            costs: execution_costs,
            commission,
            funding,
            zero_volume,
            end_of_data,
        })
    }

    /// The timeframe fills are simulated on.
    #[must_use]
    pub const fn fill_timeframe(&self) -> Timeframe {
        self.fill_timeframe
    }

    /// The order style for a simulated decision.
    #[must_use]
    pub const fn order_type(&self) -> OrderType {
        self.order_type
    }

    /// The rule that resolves a bar reaching both the stop and the target.
    #[must_use]
    pub const fn tie_break(&self) -> TieBreak {
        self.tie_break
    }

    /// The validated fee and slippage configuration.
    #[must_use]
    pub const fn costs(&self) -> ExecutionConfig {
        self.costs
    }

    /// The fee rate as a fraction of the fill notional.
    #[must_use]
    pub const fn fee_rate(&self) -> f64 {
        self.costs.fee_rate()
    }

    /// The slippage in basis points of the reference price.
    #[must_use]
    pub const fn slippage_bps(&self) -> f64 {
        self.costs.slippage_bps()
    }

    /// The commission charged on every fill.
    #[must_use]
    pub const fn commission(&self) -> CommissionConfig {
        self.commission
    }

    /// The periodic funding charge on an open position.
    #[must_use]
    pub const fn funding(&self) -> FundingConfig {
        self.funding
    }

    /// What a zero-volume bar does to an order.
    #[must_use]
    pub const fn zero_volume(&self) -> ZeroVolumePolicy {
        self.zero_volume
    }

    /// What happens to a position still open at the end of the data.
    #[must_use]
    pub const fn end_of_data(&self) -> EndOfDataPolicy {
        self.end_of_data
    }
}

/// An execution-simulation pack with its canonical identity and lifecycle state.
#[derive(Debug, Clone)]
pub struct ExecutionPack {
    core: PackCore,
    payload: ExecutionPackPayload,
}

impl ExecutionPack {
    /// Parses and validates an execution-simulation pack document.
    ///
    /// The content hash is taken over the canonical bytes of the whole
    /// envelope, including the name, version, owner, and payload. The pack
    /// starts in [`PackState::Draft`].
    ///
    /// # Errors
    ///
    /// Returns [`PackError`] when the envelope is invalid, the kind is not
    /// `execution`, or the payload disagrees with the execution schema.
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
    ) -> Result<(PackEnvelope, ExecutionPackPayload), PackError> {
        let envelope = PackEnvelope::parse(value, file)?;
        if envelope.metadata().kind() != PackKind::Execution {
            return Err(PackError::WrongKind {
                expected: PackKind::Execution,
                found: envelope.metadata().kind(),
            });
        }
        let payload = ExecutionPackPayload::parse(file, "$.payload", envelope.payload())?;
        Ok((envelope, payload))
    }

    /// Parses a JSON text document.
    ///
    /// # Errors
    ///
    /// Returns [`PackError`] when the text is not JSON or the document is not a
    /// valid execution-simulation pack.
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
    /// a valid execution-simulation pack, or are not canonical.
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
    pub fn payload(&self) -> &ExecutionPackPayload {
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

fn parse_order_type(file: &str, path: &str, value: &Value) -> Result<OrderType, ConfigError> {
    let code = read::string(file, path, value)?;
    OrderType::from_code(code)
        .ok_or_else(|| ConfigError::unsupported(file, path, OrderType::MarketNextOpen.code(), code))
}

fn parse_tie_break(file: &str, path: &str, value: &Value) -> Result<TieBreak, ConfigError> {
    let code = read::string(file, path, value)?;
    TieBreak::from_code(code)
        .ok_or_else(|| ConfigError::unsupported(file, path, TieBreak::StopFirst.code(), code))
}

fn parse_zero_volume(
    file: &str,
    path: &str,
    value: &Value,
) -> Result<ZeroVolumePolicy, ConfigError> {
    let code = read::string(file, path, value)?;
    ZeroVolumePolicy::from_code(code)
        .ok_or_else(|| ConfigError::unsupported(file, path, ZeroVolumePolicy::Reject.code(), code))
}

fn parse_end_of_data(
    file: &str,
    path: &str,
    value: &Value,
) -> Result<EndOfDataPolicy, ConfigError> {
    let code = read::string(file, path, value)?;
    EndOfDataPolicy::from_code(code).ok_or_else(|| {
        ConfigError::unsupported(file, path, EndOfDataPolicy::LeaveOpen.code(), code)
    })
}

#[cfg(test)]
mod tests;
