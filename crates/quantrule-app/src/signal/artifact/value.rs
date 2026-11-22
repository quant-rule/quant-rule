//! Serialization of the signal artifact to its wire form.
//!
//! The value tree is the input to the canonical writer; the canonical bytes,
//! not this tree, are the identity. Keeping the writer separate from the parser
//! and the type definitions makes it obvious that an output change is a contract
//! change.

use quantrule_backtest::intent::{BracketIntent, EntryStyle, StopSpec, TargetSpec, TradeIntent};
use serde_json::{Value, json};

use super::{SIGNAL_ARTIFACT_SCHEMA, SignalArtifact, SignalIntent, SignalRow};

impl SignalIntent {
    /// The wire form.
    #[must_use]
    pub fn to_value(self) -> Value {
        match self {
            Self::Trade(intent) => trade_value(intent),
            Self::Bracket(intent) => bracket_value(intent),
            Self::Exit => json!({ "kind": "exit" }),
        }
    }
}

impl SignalRow {
    /// The wire form.
    #[must_use]
    pub fn to_value(&self) -> Value {
        json!({
            "bar_index": self.bar_index,
            "open_time": self.open_time,
            "available_bar_index": self.available_bar_index,
            "action": self.action.code(),
            "side": self.side.map(super::SignalSide::code),
            "intent": self.intent.map(SignalIntent::to_value),
            "reason": self.reason.map(super::SignalReason::code),
            "evidence": self.evidence,
        })
    }
}

impl SignalArtifact {
    /// The wire form.
    #[must_use]
    pub fn to_value(&self) -> Value {
        json!({
            "schema": SIGNAL_ARTIFACT_SCHEMA,
            "strategy_pack_id": self.strategy_pack_id.to_wire(),
            "model_artifact_id": self.model_artifact_id.to_wire(),
            "data_version": self.data_version,
            "timeframe": self.timeframe.code(),
            "period": {
                "from": self.period_from,
                "to": self.period_to,
            },
            "rows": self.rows.iter().map(SignalRow::to_value).collect::<Vec<_>>(),
        })
    }
}

fn trade_value(intent: TradeIntent) -> Value {
    json!({
        "kind": match intent.entry {
            EntryStyle::MarketNextOpen => "market_next_open",
            EntryStyle::Limit { .. } => "limit",
        },
        "stop": stop_value(intent.stop),
        "target": target_value(intent.target),
        "valid_until": intent.valid_until,
        "max_slippage_bps": intent.max_slippage_bps,
    })
}

fn bracket_value(intent: BracketIntent) -> Value {
    json!({
        "kind": "bracket",
        "upper_price": intent.upper_price,
        "lower_price": intent.lower_price,
        "stop": stop_value(intent.stop),
        "target": target_value(intent.target),
        "valid_for_bars": intent.valid_for_bars,
        "max_slippage_bps": intent.max_slippage_bps,
    })
}

fn stop_value(spec: StopSpec) -> Value {
    match spec {
        StopSpec::AtrMultiple { multiple } => json!({
            "kind": "atr_multiple",
            "multiple": multiple,
        }),
        StopSpec::Price { price } => json!({
            "kind": "price",
            "price": price,
        }),
    }
}

fn target_value(spec: TargetSpec) -> Value {
    match spec {
        TargetSpec::RewardToRisk { multiple } => json!({
            "kind": "reward_to_risk",
            "multiple": multiple,
        }),
        TargetSpec::Price { price } => json!({
            "kind": "price",
            "price": price,
        }),
    }
}
