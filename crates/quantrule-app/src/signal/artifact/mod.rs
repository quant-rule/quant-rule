//! The `quantrule.signal-artifact/v1` contract.
//!
//! A signal artifact is the computed output of a frozen model artifact and a
//! signal-mode strategy pack. It carries one row per execution bar, the pack,
//! model and data identity, the timeframe and the requested period. A row says
//! what the reference position did or requested on that bar: hold, enter, exit,
//! or a two-sided bracket intent. It carries no fill, quantity, fee, cash,
//! equity, P&L, or edge metric.
//!
//! The artifact is content-addressed by the SHA-256 of its canonical bytes.
//! Canonical bytes sort every object key, remove insignificant whitespace, keep
//! the stored number form, and end with one newline. The writer stores those
//! bytes; [`SignalArtifact::from_stored_bytes`] refuses any other spelling, so
//! one document cannot acquire two identities.
//!
//! This module owns the wire shape and its closed validation. The reference
//! position that produces the rows is internal to the signal computation; the
//! pure computation is exposed through the parent module.

use std::fmt;

use quantrule_api::Digest;
use quantrule_backtest::intent::{BracketIntent, TradeIntent};
use quantrule_backtest::side::Side;
use quantrule_data::Timeframe;
use serde_json::Value;

use crate::config::ConfigError;
use crate::pack::StrategyMode;
use crate::pack::StrategyPack;
use crate::pack::canonical::canonical_json;

use super::error::SignalError;

mod parse;
mod value;

/// The signal artifact schema.
pub use crate::pack::references::SIGNAL_ARTIFACT_SCHEMA;

/// What a signal row asks the reference position to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SignalAction {
    /// Keep the current reference state.
    Hold,
    /// Open a reference position on the next bar.
    Enter,
    /// Close or cancel the reference state on the next bar.
    Exit,
    /// Place a two-sided stop-entry bracket for the next bar.
    Bracket,
}

impl SignalAction {
    /// The stable wire code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Hold => "hold",
            Self::Enter => "enter",
            Self::Exit => "exit",
            Self::Bracket => "bracket",
        }
    }

    /// Parses the stable wire code.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "hold" => Some(Self::Hold),
            "enter" => Some(Self::Enter),
            "exit" => Some(Self::Exit),
            "bracket" => Some(Self::Bracket),
            _ => None,
        }
    }
}

/// The side a signal names, independent of execution's fill direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SignalSide {
    /// A long reference position.
    Long,
    /// A short reference position.
    Short,
}

impl SignalSide {
    /// The stable wire code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Long => "long",
            Self::Short => "short",
        }
    }

    /// Parses the stable wire code.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "long" => Some(Self::Long),
            "short" => Some(Self::Short),
            _ => None,
        }
    }

    /// Projects the strategy's side.
    #[must_use]
    pub const fn from_side(side: Side) -> Self {
        match side {
            Side::Long => Self::Long,
            Side::Short => Self::Short,
        }
    }

    /// The strategy's side.
    #[must_use]
    pub const fn to_side(self) -> Side {
        match self {
            Self::Long => Side::Long,
            Self::Short => Side::Short,
        }
    }
}

/// Why a signal row exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SignalReason {
    /// A market entry condition matched.
    Entry,
    /// A two-sided bracket was placed.
    BracketEntry,
    /// A two-sided bracket was reached and chose the side.
    BracketFill,
    /// A registered event or discrete-state exit condition matched.
    EventCondition,
    /// The reference position reached its time stop.
    TimeStop,
    /// The reference position reached its ATR stop.
    StopLoss,
    /// The reference position reached its reward target.
    TakeProfit,
    /// The two-sided bracket passed its entry window without a trigger.
    BracketExpired,
    /// The two-sided bracket could not be placed or resolved.
    BracketRejected,
    /// The market entry could not be opened on the next bar.
    EntryRejected,
}

impl SignalReason {
    /// The stable wire code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Entry => "entry",
            Self::BracketEntry => "bracket_entry",
            Self::BracketFill => "bracket_fill",
            Self::EventCondition => "event_condition",
            Self::TimeStop => "time_stop",
            Self::StopLoss => "stop_loss",
            Self::TakeProfit => "take_profit",
            Self::BracketExpired => "bracket_expired",
            Self::BracketRejected => "bracket_rejected",
            Self::EntryRejected => "entry_rejected",
        }
    }

    /// Parses the stable wire code.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "entry" => Some(Self::Entry),
            "bracket_entry" => Some(Self::BracketEntry),
            "bracket_fill" => Some(Self::BracketFill),
            "event_condition" => Some(Self::EventCondition),
            "time_stop" => Some(Self::TimeStop),
            "stop_loss" => Some(Self::StopLoss),
            "take_profit" => Some(Self::TakeProfit),
            "bracket_expired" => Some(Self::BracketExpired),
            "bracket_rejected" => Some(Self::BracketRejected),
            "entry_rejected" => Some(Self::EntryRejected),
            _ => None,
        }
    }
}

/// The order or target a signal row asks execution to resolve.
///
/// The intent carries no fill and no quantity. Execution owns the fill, the
/// size, the costs, and the account.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SignalIntent {
    /// A market entry for the next bar.
    Trade(TradeIntent),
    /// A two-sided stop-entry bracket for the next bar.
    Bracket(BracketIntent),
    /// A flat target: close an open position or cancel a pending intent.
    Exit,
}

/// One signal row.
#[derive(Debug, Clone, PartialEq)]
pub struct SignalRow {
    /// The zero-based execution bar index.
    pub bar_index: usize,
    /// The bar's UTC open time as Unix seconds.
    pub open_time: i64,
    /// The first bar on which the row was knowable.
    pub available_bar_index: usize,
    /// The action the row asks for.
    pub action: SignalAction,
    /// The side of an entry or exit, or `None`.
    pub side: Option<SignalSide>,
    /// The order or target intent, or `None` for a hold.
    pub intent: Option<SignalIntent>,
    /// The stable reason code, or `None` for a hold.
    pub reason: Option<SignalReason>,
    /// The event and state evidence that justified the row.
    pub evidence: Vec<Value>,
}

impl SignalRow {
    /// Builds a hold row.
    #[must_use]
    pub fn hold(bar_index: usize, open_time: i64, available_bar_index: usize) -> Self {
        Self {
            bar_index,
            open_time,
            available_bar_index,
            action: SignalAction::Hold,
            side: None,
            intent: None,
            reason: None,
            evidence: Vec::new(),
        }
    }

    fn validate(&self, file: &str, path: &str) -> Result<(), ConfigError> {
        if self.available_bar_index < self.bar_index {
            return Err(ConfigError::inconsistent(
                file,
                path,
                "an available_bar_index at or after the bar",
                &format!(
                    "bar {} available {}",
                    self.bar_index, self.available_bar_index
                ),
            ));
        }
        match self.action {
            SignalAction::Hold => {
                require(
                    file,
                    path,
                    self.side.is_none() && self.intent.is_none() && self.reason.is_none(),
                    "a hold without a side, intent, or reason",
                )?;
                require(
                    file,
                    path,
                    self.evidence.is_empty(),
                    "a hold without evidence",
                )
            }
            SignalAction::Enter => {
                require(file, path, self.side.is_some(), "an entry with a side")?;
                let intent_matches = match self.reason {
                    Some(SignalReason::Entry) => {
                        matches!(self.intent, Some(SignalIntent::Trade(_)))
                    }
                    Some(SignalReason::BracketFill) => {
                        matches!(self.intent, Some(SignalIntent::Bracket(_)))
                    }
                    _ => false,
                };
                require(
                    file,
                    path,
                    intent_matches,
                    "an entry with the market intent for entry or the bracket intent for bracket_fill",
                )
            }
            SignalAction::Exit => {
                let side_required = matches!(
                    self.reason,
                    Some(
                        SignalReason::EventCondition
                            | SignalReason::TimeStop
                            | SignalReason::StopLoss
                            | SignalReason::TakeProfit
                    )
                );
                require(
                    file,
                    path,
                    self.intent == Some(SignalIntent::Exit),
                    "an exit with a flat intent",
                )?;
                require(
                    file,
                    path,
                    self.side.is_some() == side_required,
                    "an exit side exactly for a closed position",
                )?;
                require(
                    file,
                    path,
                    matches!(
                        self.reason,
                        Some(
                            SignalReason::EventCondition
                                | SignalReason::TimeStop
                                | SignalReason::StopLoss
                                | SignalReason::TakeProfit
                                | SignalReason::BracketExpired
                                | SignalReason::BracketRejected
                                | SignalReason::EntryRejected
                        )
                    ),
                    "a known exit reason",
                )
            }
            SignalAction::Bracket => {
                require(file, path, self.side.is_none(), "a bracket without a side")?;
                require(
                    file,
                    path,
                    matches!(self.intent, Some(SignalIntent::Bracket(_))),
                    "a bracket intent",
                )?;
                require(
                    file,
                    path,
                    self.reason == Some(SignalReason::BracketEntry),
                    "a bracket_entry reason",
                )
            }
        }
    }
}

/// The computed signal artifact.
#[derive(Debug, Clone, PartialEq)]
pub struct SignalArtifact {
    strategy_pack_id: Digest,
    model_artifact_id: Digest,
    data_version: String,
    timeframe: Timeframe,
    period_from: String,
    period_to: String,
    rows: Vec<SignalRow>,
    canonical: String,
    id: Digest,
}

impl SignalArtifact {
    /// Builds and seals an artifact from validated parts.
    pub(crate) fn new(
        strategy_pack_id: Digest,
        model_artifact_id: Digest,
        data_version: String,
        timeframe: Timeframe,
        period_from: String,
        period_to: String,
        rows: Vec<SignalRow>,
    ) -> Self {
        let mut artifact = Self {
            strategy_pack_id,
            model_artifact_id,
            data_version,
            timeframe,
            period_from,
            period_to,
            rows,
            canonical: String::new(),
            id: Digest::sha256(b""),
        };
        artifact.seal();
        artifact
    }

    /// Parses a signal artifact document.
    ///
    /// # Errors
    ///
    /// Returns [`SignalError`] when the document is not JSON, a field is
    /// missing, unknown, or mistyped, a row violates the action/reason shape,
    /// or the rows are not strictly ascending by `bar_index`.
    pub fn parse_json(text: &str, file: &str) -> Result<Self, SignalError> {
        let value: Value = serde_json::from_str(text)
            .map_err(|error| ConfigError::invalid_json(file, &error.to_string()))?;
        Self::from_value(&value, file)
    }

    /// Parses stored bytes and verifies they are the canonical form.
    ///
    /// The artifact id is the SHA-256 of the stored bytes. Bytes that parse to
    /// the same document but use another key order or whitespace are refused, so
    /// one document cannot acquire a second identity.
    ///
    /// # Errors
    ///
    /// Returns [`SignalError`] when the bytes are not UTF-8, are not JSON, do
    /// not match the signal contract, or are not canonical.
    pub fn from_stored_bytes(bytes: &[u8], file: &str) -> Result<Self, SignalError> {
        let text = std::str::from_utf8(bytes).map_err(|_| {
            ConfigError::invalid_json(file, "the stored signal artifact is not UTF-8")
        })?;
        let artifact = Self::parse_json(text, file)?;
        if artifact.canonical.as_bytes() != bytes {
            return Err(SignalError::NonCanonical {
                file: file.to_owned(),
            });
        }
        Ok(artifact)
    }

    /// Verifies the artifact names the expected frozen inputs.
    ///
    /// The computation records the model artifact id its caller resolved; this
    /// check is the service seam that refuses a pack or model artifact other
    /// than the one the artifact was computed from.
    ///
    /// # Errors
    ///
    /// Returns [`SignalError::FoldMode`] for a fold-mode pack and
    /// [`SignalError::SourceMismatch`] when the pack id, model artifact id, or
    /// execution timeframe disagrees.
    pub fn verify_sources(
        &self,
        pack: &StrategyPack,
        model_artifact_id: &Digest,
    ) -> Result<(), SignalError> {
        if pack.payload().mode() != StrategyMode::Signal {
            return Err(SignalError::FoldMode);
        }
        if self.strategy_pack_id != *pack.id() {
            return Err(SignalError::SourceMismatch {
                field: "strategy_pack_id",
                expected: pack.id().to_wire(),
                found: self.strategy_pack_id.to_wire(),
            });
        }
        if self.model_artifact_id != *model_artifact_id {
            return Err(SignalError::SourceMismatch {
                field: "model_artifact_id",
                expected: model_artifact_id.to_wire(),
                found: self.model_artifact_id.to_wire(),
            });
        }
        let expected = pack.payload().strategy().execution_timeframe;
        if self.timeframe != expected {
            return Err(SignalError::SourceMismatch {
                field: "timeframe",
                expected: expected.code().to_owned(),
                found: self.timeframe.code().to_owned(),
            });
        }
        Ok(())
    }

    /// The strategy pack the artifact was computed from.
    #[must_use]
    pub const fn strategy_pack_id(&self) -> &Digest {
        &self.strategy_pack_id
    }

    /// The model artifact the artifact was computed from.
    #[must_use]
    pub const fn model_artifact_id(&self) -> &Digest {
        &self.model_artifact_id
    }

    /// The input candle identity the model artifact recorded.
    #[must_use]
    pub fn data_version(&self) -> &str {
        &self.data_version
    }

    /// The execution timeframe the rows belong to.
    #[must_use]
    pub const fn timeframe(&self) -> Timeframe {
        self.timeframe
    }

    /// The requested period start, as written.
    #[must_use]
    pub fn period_from(&self) -> &str {
        &self.period_from
    }

    /// The requested period end, as written.
    #[must_use]
    pub fn period_to(&self) -> &str {
        &self.period_to
    }

    /// The one row per execution bar.
    #[must_use]
    pub fn rows(&self) -> &[SignalRow] {
        &self.rows
    }

    /// The content hash of the canonical bytes.
    #[must_use]
    pub const fn id(&self) -> &Digest {
        &self.id
    }

    /// The canonical bytes the identity covers.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        self.canonical.as_bytes()
    }

    /// The canonical JSON text, including its trailing newline.
    #[must_use]
    pub fn canonical_text(&self) -> &str {
        &self.canonical
    }

    fn seal(&mut self) {
        let value = self.to_value();
        self.canonical = canonical_json(&value);
        self.id = Digest::sha256(self.canonical.as_bytes());
    }
}

impl fmt::Display for SignalArtifact {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "signal artifact {} over {} with {} rows",
            self.id,
            self.timeframe.code(),
            self.rows.len()
        )
    }
}

fn require(file: &str, path: &str, condition: bool, expected: &str) -> Result<(), ConfigError> {
    if condition {
        Ok(())
    } else {
        Err(ConfigError::inconsistent(
            file,
            path,
            expected,
            "a row that disagrees",
        ))
    }
}
